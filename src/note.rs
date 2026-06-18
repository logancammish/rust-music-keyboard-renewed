use std::f32::consts::TAU;
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use strum_macros::Display;
use rodio::{ChannelCount, Sample, SampleRate, Source};
use crate::audio_mixer;

// Internal synthesis sample rate. The mixer resamples to the device rate.
const SAMPLE_RATE: u32 = 48_000;

// Relative amplitudes of the fundamental and its overtones. A handful of
// gently decaying harmonics turns the bare, "stupid" sounding sine wave into a
// warmer, organ-like timbre.
const HARMONICS: [f32; 5] = [1.0, 0.5, 0.28, 0.12, 0.06];

// Note enum defines all notes in Western music
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Note {
    A, Asharp, B, C, Csharp, D, Dsharp, E, F, Fsharp, G, Gsharp, None
}

impl Note {
    // Every note that can actually be played, in keyboard order starting at C.
    // This doubles as the chromatic scale used for scale/chord maths.
    pub const NOTES: [Note; 12] = [
        Note::C, Note::Csharp, Note::D, Note::Dsharp, Note::E, Note::F,
        Note::Fsharp, Note::G, Note::Gsharp, Note::A, Note::Asharp, Note::B,
    ];

    // Position in the chromatic scale (C = 0).
    pub fn chromatic_index(self) -> usize {
        Note::NOTES.iter().position(|&n| n == self).unwrap_or(0)
    }
}

// NoteLength enum defines the length of a note
// to be calculated according to beats per minute
#[derive(Debug, Clone, Copy, Display)]
pub enum NoteLength {
    Whole, Half, Quarter, Eighth, Sixteenth
}

// implement the NoteLength enum
impl NoteLength {
    pub fn duration_in_seconds(&self, bpm: f32) -> f32 {
        match self {
            NoteLength::Whole => (60.0 / bpm) * 4.0,
            NoteLength::Half => (60.0 / bpm) * 2.0,
            NoteLength::Quarter => 60.0 / bpm,
            NoteLength::Eighth => (60.0 / bpm) * 0.5,
            NoteLength::Sixteenth => (60.0 / bpm) * 0.25,
        }
    }

    pub fn check_bpm(bpm: f32) -> bool {
        bpm > 0.1 && bpm <= 300.0
    }
}

// Synthesis parameters shared by every voice, sourced from the user's settings.
#[derive(Debug, Clone, Copy)]
pub struct ToneSettings {
    pub attack_ms: u32,
    pub release_ms: u32,
    pub gain: f32,
}

// A single synthesised voice: a bank of harmonics shaped by an attack/sustain/
// release envelope. The envelope removes the clicks that an abruptly started or
// stopped sine wave produces, and the gradual release keeps held notes from
// popping when they end.
//
// Sustain is controlled in one of two ways:
//   * `gate` present (hold mode) -> sustains until the gate is set to `false`.
//   * `gate` absent (fixed mode) -> sustains for `sustain_samples` samples.
pub struct Voice {
    frequency: f32,
    amplitude: f32,
    phase: f32,
    norm: f32,

    attack_samples: u32,
    release_samples: u32,
    elapsed: u32,

    gate: Option<Arc<AtomicBool>>,
    sustain_samples: Option<u32>,

    releasing: bool,
    release_elapsed: u32,
    release_level: f32,
    finished: bool,
}

impl Voice {
    fn new(
        frequency: f32,
        amplitude: f32,
        settings: ToneSettings,
        gate: Option<Arc<AtomicBool>>,
        sustain_samples: Option<u32>,
    ) -> Self {
        let attack_samples = (settings.attack_ms * SAMPLE_RATE / 1000).max(1);
        let release_samples = (settings.release_ms * SAMPLE_RATE / 1000).max(1);
        let norm = 1.0 / HARMONICS.iter().sum::<f32>();

        Voice {
            frequency,
            amplitude,
            phase: 0.0,
            norm,
            attack_samples,
            release_samples,
            elapsed: 0,
            gate,
            sustain_samples,
            releasing: false,
            release_elapsed: 0,
            release_level: 1.0,
            finished: false,
        }
    }

    // Envelope level (0.0..=1.0) during the attack/sustain phases.
    fn pre_release_level(&self) -> f32 {
        if self.elapsed < self.attack_samples {
            self.elapsed as f32 / self.attack_samples as f32
        } else {
            1.0
        }
    }

    fn should_release(&self) -> bool {
        match (&self.gate, self.sustain_samples) {
            (Some(gate), _) => !gate.load(Ordering::Relaxed),
            (None, Some(sustain)) => self.elapsed >= self.attack_samples + sustain,
            (None, None) => false,
        }
    }
}

impl Iterator for Voice {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if self.finished {
            return None;
        }

        if !self.releasing && self.should_release() {
            self.releasing = true;
            self.release_elapsed = 0;
            self.release_level = self.pre_release_level();
        }

        let envelope = if self.releasing {
            let progress = self.release_elapsed as f32 / self.release_samples as f32;
            if progress >= 1.0 {
                self.finished = true;
                return None;
            }
            self.release_level * (1.0 - progress)
        } else {
            self.pre_release_level()
        };

        // Additive synthesis: sum the fundamental and its harmonics.
        let mut value = 0.0;
        for (index, harmonic) in HARMONICS.iter().enumerate() {
            let k = (index + 1) as f32;
            value += harmonic * (self.phase * k).sin();
        }
        let sample = value * self.norm * self.amplitude * envelope;

        self.phase += TAU * self.frequency / SAMPLE_RATE as f32;
        if self.phase >= TAU {
            self.phase -= TAU;
        }
        self.elapsed += 1;
        if self.releasing {
            self.release_elapsed += 1;
        }

        Some(sample)
    }
}

impl Source for Voice {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        NonZero::new(SAMPLE_RATE).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// RealNote struct, used for playing sounds according
// to their length and octave
#[derive(Debug, Clone)]
pub struct RealNote {
    pub note: Note,
    pub length: NoteLength,
    pub octave: f32,
}

impl RealNote {
    pub fn base_frequencies(note: Note) -> f32 {
        match note {
            Note::C => 16.35,
            Note::Csharp => 17.32,
            Note::D => 18.35,
            Note::Dsharp => 19.45,
            Note::E => 20.60,
            Note::F => 21.83,
            Note::Fsharp => 23.12,
            Note::G => 24.50,
            Note::Gsharp => 25.96,
            Note::A => 27.50,
            Note::Asharp => 29.14,
            Note::B => 30.87,
            Note::None => 0.0,
        }
    }

    fn frequency(&self) -> f32 {
        Self::base_frequencies(self.note) * 2_f32.powf(self.octave)
    }

    // Per-voice peak amplitude. Kept low so that several simultaneous notes sum
    // together without clipping the output.
    fn amplitude(volume: f32, settings: ToneSettings) -> f32 {
        (volume / 100.0).clamp(0.0, 1.0) * settings.gain
    }
}

// A sound source can be triggered in either of the program's two modes.
// `play_held` returns the gates that keep the voices sounding until released;
// `play_fixed` plays for a length derived from the BPM and note length.
pub trait Playable {
    fn play_held(&self, volume: f32, settings: ToneSettings) -> Vec<Arc<AtomicBool>>;
    fn play_fixed(&self, bpm: f32, volume: f32, settings: ToneSettings);
}

impl Playable for RealNote {
    fn play_held(&self, volume: f32, settings: ToneSettings) -> Vec<Arc<AtomicBool>> {
        if self.note == Note::None {
            return vec![];
        }
        let gate = Arc::new(AtomicBool::new(true));
        let voice = Voice::new(
            self.frequency(),
            Self::amplitude(volume, settings),
            settings,
            Some(gate.clone()),
            None,
        );
        audio_mixer().add(voice);
        vec![gate]
    }

    fn play_fixed(&self, bpm: f32, volume: f32, settings: ToneSettings) {
        if self.note == Note::None {
            return;
        }
        let seconds = self.length.duration_in_seconds(bpm);
        let sustain_samples = (seconds * SAMPLE_RATE as f32) as u32;
        let voice = Voice::new(
            self.frequency(),
            Self::amplitude(volume, settings),
            settings,
            None,
            Some(sustain_samples),
        );
        audio_mixer().add(voice);
    }
}
