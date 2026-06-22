use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::{ToneSettings, VisualizerStyle, Waveform};

const SETTINGS_PATH: &str = "./config/settings.json";

// User-editable settings persisted to config/settings.json and surfaced in the
// "Advanced Settings" tab. `#[serde(default)]` means a file missing any field
// (such as one written by an older version) still loads cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // Show the help screen when the program starts.
    pub info_popup: bool,
    // Start in hold-to-play mode (note length follows how long a key is held).
    pub default_hold_mode: bool,
    // Envelope attack time in milliseconds (how quickly a note fades in).
    pub attack_ms: u32,
    // Envelope release time in milliseconds (how gently a note fades out).
    pub release_ms: u32,
    // Master output gain (per-voice amplitude). Lower values leave more
    // headroom before the mixed output clips.
    pub master_gain: f32,
    // The oscillator shape used for every voice (the "sound" of the keyboard).
    pub waveform: Waveform,
    // Which animated visualiser to draw (or `Off` to disable it).
    pub visualizer: VisualizerStyle,
    // Directory that recorded MIDI files are written to. Leave blank to use the
    // platform default (Documents/RustMusicKeyboardRenewed on Windows).
    pub output_dir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            info_popup: true,
            default_hold_mode: true,
            attack_ms: 12,
            release_ms: 160,
            master_gain: 0.18,
            waveform: Waveform::default(),
            visualizer: VisualizerStyle::default(),
            output_dir: String::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        match fs::read_to_string(SETTINGS_PATH) {
            Ok(raw) => {
                // Editors and Windows tooling often save JSON with a UTF-8 BOM,
                // which serde_json rejects; strip it (and surrounding whitespace)
                // before parsing.
                let cleaned = raw.trim_start_matches('\u{feff}').trim();
                serde_json::from_str(cleaned).unwrap_or_else(|e| {
                    eprintln!("settings.json could not be parsed ({e}); using defaults");
                    Settings::default()
                })
            }
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = Path::new(SETTINGS_PATH).parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(SETTINGS_PATH, json).map_err(|e| e.to_string())
    }

    // The synthesis parameters derived from these settings.
    pub fn tone(&self) -> ToneSettings {
        ToneSettings {
            attack_ms: self.attack_ms.max(1),
            release_ms: self.release_ms.max(1),
            gain: self.master_gain.clamp(0.02, 0.5),
            waveform: self.waveform,
        }
    }
}
