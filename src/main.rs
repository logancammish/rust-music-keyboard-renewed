#![windows_subsystem = "windows"]
// use other files inside this project
mod gui;
mod chord;
mod midi;
mod note;
mod settings;
use gui::*;
use chord::*;
use note::*;
use settings::Settings;

// use dependencies
use iced::{
    event::{self, Event},
    keyboard, time, Element, Size, Subscription, Theme,
};
use once_cell::sync::Lazy;
use rodio::mixer::Mixer;
use rodio::stream::DeviceSinkBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

// How often the UI ticks. Drives the recording timer and the key/record
// animations, so it runs at roughly 60 fps for smooth motion.
const TICK: std::time::Duration = std::time::Duration::from_millis(16);
// Seconds for a key's highlight to fade out after it is released.
const GLOW_FADE: f32 = 0.28;

// The audio output stream owns a non-`Send` `cpal::Stream`, so it has to live
// on its own thread. We park that thread forever to keep the stream alive and
// hand the (cloneable, `Send + Sync`) mixer back to the rest of the program.
// Every voice is queued onto this single mixer, which gives us polyphony for
// free without spawning a thread per note.
static AUDIO: Lazy<Mixer> = Lazy::new(|| {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(stream) => {
                let _ = tx.send(Some(stream.mixer().clone()));
                std::thread::park();
                drop(stream);
            }
            Err(e) => {
                eprintln!("Failed to open audio output: {e}");
                let _ = tx.send(None);
            }
        }
    });

    rx.recv()
        .ok()
        .flatten()
        .expect("audio output stream could not be initialised")
});

pub fn audio_mixer() -> &'static Mixer {
    &AUDIO
}

#[derive(Debug, Clone)]
struct Song {
    notes: Vec<(Note, f32, f32, f32)>, // Note, octave, start_time, duration
    bpm: f32,
}

// Tracks timing metadata for the current recording. This runs entirely on the
// UI thread (driven by Play/EndPlaying messages), so it no longer needs the
// shared, mutex-guarded statics the old version relied on.
#[derive(Default)]
struct Recording {
    start: Option<Instant>,
    events: Vec<(Note, f32, f32, f32)>,
    pending: HashMap<Note, (f32, f32)>, // note -> (octave, start_time)
}

impl Recording {
    fn is_active(&self) -> bool {
        self.start.is_some()
    }

    fn elapsed(&self) -> f32 {
        self.start.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0)
    }

    fn begin(&mut self) {
        self.start = Some(Instant::now());
        self.events.clear();
        self.pending.clear();
    }

    // A note whose length is determined by how long it is held.
    fn note_on(&mut self, note: Note, octave: f32) {
        if self.is_active() {
            self.pending.insert(note, (octave, self.elapsed()));
        }
    }

    fn note_off(&mut self, note: Note) {
        if !self.is_active() {
            return;
        }
        if let Some((octave, start)) = self.pending.remove(&note) {
            let duration = (self.elapsed() - start).max(0.05);
            self.events.push((note, octave, start, duration));
        }
    }

    // A note with a predetermined length.
    fn note_fixed(&mut self, note: Note, octave: f32, duration: f32) {
        if self.is_active() {
            self.events.push((note, octave, self.elapsed(), duration));
        }
    }

    fn finish(&mut self, bpm: f32) -> Song {
        // Close any notes still being held when recording stopped.
        let pending: Vec<Note> = self.pending.keys().copied().collect();
        for note in pending {
            self.note_off(note);
        }
        let song = Song {
            notes: std::mem::take(&mut self.events),
            bpm,
        };
        self.start = None;
        song
    }
}

// A note (or chord) currently sounding in hold mode.
struct ActiveNote {
    gates: Vec<Arc<AtomicBool>>,
    recorded: Vec<Note>,
}

// Message enum
#[derive(Debug, Clone, PartialEq)]
enum Message {
    Scale(Note),
    ScaleTypeChange(ScaleType),
    OctaveChange(f32),
    BpmChange(f32),
    CustomBpmChange(String),
    Play(Note),
    EndPlaying(Note),
    KeyPressed(iced::keyboard::Key),
    KeyReleased(iced::keyboard::Key),
    ToggleChords,
    ToggleHold,
    ToggleRecording,
    NoteLengthChange(f32),
    VolumeChange(f32),
    WaveformChange(Waveform),
    FileNameChange(String),
    SwitchMenu(CurrentMenu),
    // Advanced settings
    SetInfoPopup(bool),
    SetDefaultHold(bool),
    SetVisualizer(VisualizerStyle),
    AttackChange(f32),
    ReleaseChange(f32),
    GainChange(f32),
    OutputDirChange(String),
    SaveSettings,
    ResetSettings,
    Tick,
}

// Program struct, which stores the current information the program may need
struct Program {
    octave: f32,
    bpm: f32,
    custom_bpm: String,
    play_chords: bool,
    hold_mode: bool,
    selected_scale: Option<Note>,
    scale_type: ScaleType,
    note_length: f32,
    volume: f32,
    buttons_pressed: HashMap<Note, bool>,
    key_glow: HashMap<Note, f32>,
    clock: f32,
    current_menu: CurrentMenu,
    settings: Settings,
    settings_saved: bool,
    recording: Recording,
    active: HashMap<Note, ActiveNote>,
    file_name: String,
    last_saved: Option<String>,
}

impl Program {
    pub fn get_note_length(length: f32) -> NoteLength {
        match length {
            5.0 => NoteLength::Whole,
            4.0 => NoteLength::Half,
            3.0 => NoteLength::Quarter,
            2.0 => NoteLength::Eighth,
            1.0 => NoteLength::Sixteenth,
            _ => NoteLength::Whole,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording.is_active()
    }

    pub fn update_bpm(&mut self, value: f32) {
        if NoteLength::check_bpm(value) {
            self.bpm = value;
            self.custom_bpm = (value.round() as i32).to_string();
        } else {
            self.bpm = 60.0;
            self.custom_bpm = "60".to_string();
        }
    }

    fn view(&self) -> Element<'_, Message> {
        self.get_ui_information()
    }

    fn match_keyboard_key(key: keyboard::Key) -> Option<Note> {
        match key {
            keyboard::Key::Character(c) => match c.as_str() {
                "a" => Some(Note::C),
                "w" => Some(Note::Csharp),
                "s" => Some(Note::D),
                "r" => Some(Note::Dsharp),
                "d" => Some(Note::E),
                "f" => Some(Note::F),
                "t" => Some(Note::Fsharp),
                "g" => Some(Note::G),
                "y" => Some(Note::Gsharp),
                "h" => Some(Note::A),
                "u" => Some(Note::Asharp),
                "j" => Some(Note::B),
                _ => None,
            },
            _ => None,
        }
    }

    // The notes that actually sound when `note` is pressed (a single note, or a
    // triad when chord mode is on).
    fn sounding_notes(&self, note: Note) -> Vec<Note> {
        if self.play_chords {
            let real = RealNote { note, length: NoteLength::Quarter, octave: self.octave };
            Chord::triad_from_note(&real, self.scale_type)
                .notes
                .iter()
                .map(|n| n.note)
                .collect()
        } else {
            vec![note]
        }
    }

    fn press(&mut self, note: Note) {
        if note == Note::None {
            return;
        }

        self.buttons_pressed.insert(note, true);
        self.key_glow.insert(note, 1.0);

        let real_note = RealNote {
            note,
            length: Self::get_note_length(self.note_length),
            octave: self.octave,
        };
        let tone = self.settings.tone();

        if self.hold_mode {
            // Avoid retriggering a note that is already sounding.
            if self.active.contains_key(&note) {
                return;
            }

            let gates = if self.play_chords {
                Chord::triad_from_note(&real_note, self.scale_type).play_held(self.volume, tone)
            } else {
                real_note.play_held(self.volume, tone)
            };

            let recorded = self.sounding_notes(note);
            for &n in &recorded {
                self.recording.note_on(n, self.octave);
            }

            self.active.insert(note, ActiveNote { gates, recorded });
        } else {
            let duration = Self::get_note_length(self.note_length)
                .duration_in_seconds(self.bpm);

            if self.play_chords {
                Chord::triad_from_note(&real_note, self.scale_type)
                    .play_fixed(self.bpm, self.volume, tone);
            } else {
                real_note.play_fixed(self.bpm, self.volume, tone);
            }

            for n in self.sounding_notes(note) {
                self.recording.note_fixed(n, self.octave, duration);
            }
        }
    }

    fn release(&mut self, note: Note) {
        self.buttons_pressed.insert(note, false);

        if let Some(active) = self.active.remove(&note) {
            for gate in active.gates {
                gate.store(false, Ordering::Relaxed);
            }
            for n in active.recorded {
                self.recording.note_off(n);
            }
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SwitchMenu(menu) => {
                self.current_menu = menu;
            }

            Message::NoteLengthChange(value) => self.note_length = value,
            Message::VolumeChange(value) => self.volume = value,

            Message::WaveformChange(waveform) => {
                self.settings.waveform = waveform;
                self.settings_saved = false;
            }

            Message::Tick => {
                let dt = TICK.as_secs_f32();
                self.clock += dt;
                for note in Note::NOTES {
                    let held = self.buttons_pressed.get(&note).copied().unwrap_or(false);
                    let glow = self.key_glow.entry(note).or_insert(0.0);
                    if held {
                        *glow = 1.0;
                    } else if *glow > 0.0 {
                        *glow = (*glow - dt / GLOW_FADE).max(0.0);
                    }
                }
            }

            Message::Scale(note) => {
                self.selected_scale = if note == Note::None { None } else { Some(note) };
            }

            Message::ScaleTypeChange(scale_type) => self.scale_type = scale_type,

            Message::KeyPressed(key) => {
                // Octave shortcuts: ] raises the octave, [ lowers it.
                if let keyboard::Key::Character(c) = &key {
                    match c.as_str() {
                        "]" => {
                            self.octave = (self.octave + 1.0).min(6.0);
                            return;
                        }
                        "[" => {
                            self.octave = (self.octave - 1.0).max(0.0);
                            return;
                        }
                        _ => {}
                    }
                }

                if let Some(note) = Self::match_keyboard_key(key)
                    && !self.buttons_pressed.get(&note).copied().unwrap_or(false)
                {
                    self.press(note);
                }
            }

            Message::KeyReleased(key) => {
                if let Some(note) = Self::match_keyboard_key(key) {
                    self.release(note);
                }
            }

            Message::ToggleRecording => {
                if !self.recording.is_active() {
                    self.recording.begin();
                } else {
                    let song = self.recording.finish(self.bpm);
                    match midi::Midi::midi_file_create(
                        song,
                        &self.settings.output_dir,
                        &self.file_name,
                    ) {
                        Some(path) => self.last_saved = Some(path.display().to_string()),
                        None => self.last_saved = Some("Failed to save MIDI file".to_string()),
                    }
                }
            }

            Message::FileNameChange(value) => self.file_name = value,

            Message::OutputDirChange(value) => {
                self.settings.output_dir = value;
                self.settings_saved = false;
            }

            Message::ToggleChords => self.play_chords = !self.play_chords,

            Message::ToggleHold => {
                self.hold_mode = !self.hold_mode;
                // Release anything currently held so notes don't get stuck.
                let active: Vec<Note> = self.active.keys().copied().collect();
                for note in active {
                    self.release(note);
                }
            }

            Message::OctaveChange(value) => self.octave = value,

            Message::CustomBpmChange(value) => {
                // Always reflect exactly what the user typed so the field never
                // fights the cursor; only adopt the value as the tempo once it
                // parses and lands in range (so a partial "3" on the way to
                // "300" doesn't snap the BPM around).
                if let Ok(parsed) = value.parse::<f32>()
                    && NoteLength::check_bpm(parsed)
                {
                    self.bpm = parsed;
                }
                self.custom_bpm = value;
            }

            Message::BpmChange(value) => self.update_bpm(value),

            Message::EndPlaying(note) => self.release(note),

            Message::Play(note) => self.press(note),

            Message::SetInfoPopup(value) => {
                self.settings.info_popup = value;
                self.settings_saved = false;
            }
            Message::SetDefaultHold(value) => {
                self.settings.default_hold_mode = value;
                self.settings_saved = false;
            }
            Message::SetVisualizer(style) => {
                self.settings.visualizer = style;
                self.settings_saved = false;
            }
            Message::AttackChange(value) => {
                self.settings.attack_ms = value as u32;
                self.settings_saved = false;
            }
            Message::ReleaseChange(value) => {
                self.settings.release_ms = value as u32;
                self.settings_saved = false;
            }
            Message::GainChange(value) => {
                self.settings.master_gain = value;
                self.settings_saved = false;
            }
            Message::SaveSettings => {
                match self.settings.save() {
                    Ok(()) => self.settings_saved = true,
                    Err(e) => eprintln!("Failed to save settings: {e}"),
                }
            }
            Message::ResetSettings => {
                self.settings = Settings::default();
                self.settings_saved = false;
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // `listen_with` exposes the event status, which lets us ignore key
        // presses that a focused widget already handled. That stops the
        // computer-keyboard note mapping from firing notes while the user is
        // typing in the filename / BPM / save-folder fields. Releases are
        // always handled so a held note can never get stuck.
        let keyboard = event::listen_with(|event, status, _window| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, repeat, .. }) => {
                if repeat || status == event::Status::Captured {
                    None
                } else {
                    Some(Message::KeyPressed(key))
                }
            }
            Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) => {
                Some(Message::KeyReleased(key))
            }
            _ => None,
        });

        let timer = time::every(TICK).map(|_| Message::Tick);

        Subscription::batch(vec![keyboard, timer])
    }
}

// changing Default for Program
impl Default for Program {
    fn default() -> Self {
        let settings = Settings::load();

        let mut buttons_pressed = HashMap::new();
        let mut key_glow = HashMap::new();
        for note in Note::NOTES {
            buttons_pressed.insert(note, false);
            key_glow.insert(note, 0.0);
        }

        let current_menu = if settings.info_popup {
            CurrentMenu::Help
        } else {
            CurrentMenu::Standard
        };

        Self {
            note_length: 2.0,
            selected_scale: None,
            scale_type: ScaleType::Major,
            octave: 4.0,
            bpm: 120.0,
            custom_bpm: "120".to_string(),
            play_chords: false,
            hold_mode: settings.default_hold_mode,
            volume: 30.0,
            buttons_pressed,
            key_glow,
            clock: 0.0,
            current_menu,
            settings,
            settings_saved: true,
            recording: Recording::default(),
            active: HashMap::new(),
            file_name: "recording".to_string(),
            last_saved: None,
        }
    }
}

fn theme(_state: &Program) -> Theme {
    Theme::TokyoNight
}

fn load_icon() -> Option<iced::window::Icon> {
    let mut icon_bytes = Vec::new();
    File::open("./assets/icon.ico")
        .ok()?
        .read_to_end(&mut icon_bytes)
        .ok()?;

    let image = image::load_from_memory(&icon_bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    iced::window::icon::from_rgba(image.into_raw(), width, height).ok()
}

// main function
pub fn main() -> iced::Result {
    let window_settings = iced::window::Settings {
        icon: load_icon(),
        // Generous minimum so the keyboard always fits, but small enough for
        // modest laptop displays; the content scrolls if the window is shorter.
        min_size: Some(Size::new(520.0, 460.0)),
        ..iced::window::Settings::default()
    };

    iced::application(Program::default, Program::update, Program::view)
        .title("Rust Music Keyboard Renewed")
        .window_size(Size::new(720.0, 760.0))
        .subscription(Program::subscription)
        .theme(theme)
        .window(window_settings)
        .run()
}
