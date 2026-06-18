use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent};
use midly::num::{u28, u24, u7, u4};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use crate::{Note, Song};

pub struct Midi {}

// Default per-platform output directory, used when no custom directory is set.
fn default_output_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let home = env::var("USERPROFILE")
            .or_else(|_| env::var("USERNAME").map(|u| format!("C:\\Users\\{u}")))
            .unwrap_or_else(|_| "C:\\Users\\Default".to_string());
        PathBuf::from(home)
            .join("Documents")
            .join("RustMusicKeyboardRenewed")
    } else if cfg!(target_os = "linux") {
        PathBuf::from("/tmp/RustMusicKeyboardRenewed")
    } else {
        PathBuf::from("./RustMusicKeyboardRenewed")
    }
}

// Strip anything that isn't safe in a file name and fall back to a default.
fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        "recording".to_string()
    } else {
        cleaned
    }
}

// Resolve a unique path inside `dir` for `<name>.mid`, appending " (n)" if a
// file with that name already exists so each recording becomes its own file.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(format!("{name}.mid"));
    if !candidate.exists() {
        return candidate;
    }
    for n in 2..10_000 {
        let candidate = dir.join(format!("{name} ({n}).mid"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{name}.mid"))
}

// impliment for Midi
// functions: 
// 1. note_to_midi  -> converts note to u7 midi value
// 2. bpm_to_microseconds_per_beat  -> converts bpm to u24 microseconds per beat
// 3. midi_file_create -> creates a midi file with the valid info
impl Midi {
    pub fn note_to_midi(note: Note, octave: f32) -> u7 {
        let note_index = match note {
            Note::C => 0,
            Note::Csharp => 1,
            Note::D => 2,
            Note::Dsharp => 3,
            Note::E => 4,
            Note::F => 5,
            Note::Fsharp => 6,
            Note::G => 7,
            Note::Gsharp => 8,
            Note::A => 9,
            Note::Asharp => 10,
            Note::B => 11,
            Note::None => 0, // Default to C when Note is None
        };
        let midi_note = 12 * (octave as i32 + 1) + note_index;

        u7::new(midi_note as u8)
    }

    pub fn bpm_to_microseconds_per_beat(bpm: f32) -> u24 {
        u24::from((60_000_000.0 / bpm) as u32)
    }

    // Serialises `song` to a `.mid` file. The directory comes from settings
    // (or a platform default when blank) and the file name is supplied by the
    // user; a unique suffix is added so existing recordings are never
    // overwritten. Returns the path written, if successful.
    pub fn midi_file_create(song: Song, output_dir: &str, file_name: &str) -> Option<PathBuf> {
        let header = Header::new(Format::SingleTrack, Timing::Metrical(480.into()));
        let mut smf = Smf::new(header);
    
        let mut track: Vec<TrackEvent<'_>> = Track::new();
        let tempo = MetaMessage::Tempo(Self::bpm_to_microseconds_per_beat(song.bpm));
        track.push(TrackEvent {
            delta: u28::new(0),
            kind: midly::TrackEventKind::Meta(tempo),
        });
    
        let mut events = Vec::new();
        
        for (note, octave, start_time, duration) in &song.notes {
            // Skip Note::None entries
            if *note == Note::None {
                continue;
            }
            
            let midi_note = Self::note_to_midi(*note, *octave);
            let beats_per_second = song.bpm / 60.0;
            let start_ticks = (start_time * beats_per_second * 480.0).round() as u32;
            let duration_ticks = (duration * beats_per_second * 480.0).round() as u32;
    
            events.push((
                start_ticks,
                midly::TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: MidiMessage::NoteOn {
                        key: midi_note,
                        vel: u7::new(64),
                    },
                },
            ));
    
            events.push((
                start_ticks + duration_ticks,
                midly::TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: MidiMessage::NoteOff {
                        key: midi_note,
                        vel: u7::new(64),
                    },
                },
            ));
        }
    
        events.sort_by_key(|(time, _)| *time);
    
        let mut last_time = 0;
        for (time, event) in events {
            track.push(TrackEvent {
                delta: u28::new(time - last_time),
                kind: event,
            });
            last_time = time;
        }
    
        smf.tracks.push(track);

        let dir = if output_dir.trim().is_empty() {
            default_output_dir()
        } else {
            PathBuf::from(output_dir.trim())
        };

        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("Failed to create output directory {dir:?}: {e}");
            return None;
        }

        let output_file = unique_path(&dir, &sanitize_file_name(file_name));

        // Actually serialise the song to disk. The original version wrote an
        // empty buffer here, so every exported file was 0 bytes.
        match smf.save(&output_file) {
            Ok(()) => {
                println!("MIDI file saved at: {output_file:?}");
                Some(output_file)
            }
            Err(e) => {
                eprintln!("Failed to save MIDI file: {e}");
                None
            }
        }
    }
}