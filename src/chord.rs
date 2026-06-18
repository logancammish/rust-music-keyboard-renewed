// use other files inside this project
use crate::{Program, Note, RealNote, Playable, ToneSettings};

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

// Whether scales/triads are built as major or (natural) minor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleType {
    Major,
    Minor,
}

impl ScaleType {
    pub const ALL: [ScaleType; 2] = [ScaleType::Major, ScaleType::Minor];

    // Semitone offsets from the root for each degree of the scale.
    fn intervals(self) -> [usize; 7] {
        match self {
            ScaleType::Major => [0, 2, 4, 5, 7, 9, 11],
            ScaleType::Minor => [0, 2, 3, 5, 7, 8, 10],
        }
    }
}

impl fmt::Display for ScaleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ScaleType::Major => "Major",
            ScaleType::Minor => "Minor",
        })
    }
}

// Chord struct, which is used to play multiple notes at once
pub struct Chord {
    pub notes: Vec<RealNote>,
}

impl Chord {
    // Builds a scale from a root note and a scale type by walking the
    // chromatic circle. Replaces the old hand-written lookup table.
    pub fn scale(root: Note, scale_type: ScaleType) -> Vec<Note> {
        if root == Note::None {
            return Note::NOTES.to_vec();
        }
        let root_index = root.chromatic_index();
        scale_type
            .intervals()
            .iter()
            .map(|offset| Note::NOTES[(root_index + offset) % 12])
            .collect()
    }

    pub fn is_note_in_scale(program: &Program, note: Note) -> bool {
        match program.selected_scale {
            None => true,
            Some(root) => Chord::scale(root, program.scale_type).contains(&note),
        }
    }

    // The 1st, 3rd and 5th degrees of the scale rooted on the given note,
    // producing a major or minor triad depending on the scale type.
    pub fn triad_from_note(note: &RealNote, scale_type: ScaleType) -> Chord {
        let scale = Self::scale(note.note, scale_type);
        Chord {
            notes: vec![
                RealNote { note: scale[0], length: note.length, octave: note.octave },
                RealNote { note: scale[2], length: note.length, octave: note.octave },
                RealNote { note: scale[4], length: note.length, octave: note.octave },
            ],
        }
    }
}

// implement Playable trait for Chord
impl Playable for Chord {
    fn play_held(&self, volume: f32, settings: ToneSettings) -> Vec<Arc<AtomicBool>> {
        // The shared mixer is polyphonic, so every note of the chord can sound
        // at once. Collect each voice's gate so they can be released together.
        self.notes
            .iter()
            .flat_map(|note| note.play_held(volume, settings))
            .collect()
    }

    fn play_fixed(&self, bpm: f32, volume: f32, settings: ToneSettings) {
        for note in &self.notes {
            note.play_fixed(bpm, volume, settings);
        }
    }
}
