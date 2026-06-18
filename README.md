# Rust Music Keyboard Renewed

[![build](https://github.com/logancammish/rust-music-keyboard-renewed/actions/workflows/rust.yml/badge.svg)](https://github.com/logancammish/rust-music-keyboard-renewed/actions/workflows/rust.yml)

A polished, playable polyphonic synth keyboard built in Rust with [`iced`](https://iced.rs) and [`rodio`](https://github.com/RustAudio/rodio), featuring MIDI export.

> **Fork notice** — this is a remastered fork of the original
> [logancammish/rust-music-keyboard](https://github.com/logancammish/rust-music-keyboard),
> which was awarded NZQA Scholarship in Technology and is no longer updated.
> *Renewed* modernises the dependencies, rebuilds the audio engine and UI, and
> adds new features. Development continues here:
> [logancammish/rust-music-keyboard-renewed](https://github.com/logancammish/rust-music-keyboard-renewed).

![Rust Music Keyboard Renewed](assets/icon.ico)

## Features

| Feature | |
|---|:---:|
| Polyphonic synth voice (harmonics + attack/release envelope) | ✔️ |
| **Hold-to-play** — note length follows how long you hold a key | ✔️ |
| Fixed note lengths (Whole → Sixteenth) with adjustable BPM | ✔️ |
| Play every note in Western music, by mouse or computer keyboard | ✔️ |
| Major **and minor** scales, with key highlighting | ✔️ |
| Play major/minor triads from the pressed note | ✔️ |
| Adjustable octave (buttons or the `[` / `]` keys), volume | ✔️ |
| MIDI recording & export to a custom folder and file name | ✔️ |
| Advanced Settings tab (envelope, gain, save folder, startup) | ✔️ |
| Responsive UI that scales to different window sizes | ✔️ |

## Keyboard mapping

| Key | A | W | S | R | D | F | T | G | Y | H | U | J |
|-----|---|---|---|---|---|---|---|---|---|---|---|---|
| Note| C | C#| D | D#| E | F | F#| G | G#| A | A#| B |

`[` lowers the octave, `]` raises it.

## Building

This application officially supports Windows and should work on Linux.

1. Install Rust via [Rustup](https://www.rust-lang.org/tools/install) if you haven't already.
2. Clone the repository:
   ```
   git clone https://github.com/logancammish/rust-music-keyboard-renewed.git
   ```
3. Build a release binary:
   ```
   cargo build --release
   ```
4. The executable is at `target/release/RustMusicKeyboardRenewed.exe` (or `RustMusicKeyboardRenewed` on Linux).

Windows users can also grab the latest installer from the
[releases page](https://github.com/logancammish/rust-music-keyboard-renewed/releases/latest).

## Recordings

Press **Start recording**, play, then **Stop recording** to export a MIDI file.
The file is named from the **Save as** field (a number is appended if the name
already exists, so nothing is overwritten) and written to the **save folder**,
which can be changed in **Advanced Settings** or in `config/settings.json`.

Default save folder:

| Platform | Location |
|----------|----------|
| Windows  | `C:\Users\USERNAME\Documents\RustMusicKeyboardRenewed\` |
| Linux    | `/tmp/RustMusicKeyboardRenewed/` |
| Other    | `./RustMusicKeyboardRenewed/` |

## Configuration

`config/settings.json` holds user settings (also editable in the Advanced tab):

| Key | Meaning |
|-----|---------|
| `info_popup` | Show the help screen on startup |
| `default_hold_mode` | Start in hold-to-play mode |
| `attack_ms` | Envelope attack time (note fade-in) |
| `release_ms` | Envelope release time (note fade-out) |
| `master_gain` | Per-voice output gain (lower = more headroom before clipping) |
| `output_dir` | Save folder for recordings (blank = platform default) |

## Versioning

Format: `MAJOR.MINOR.PATCH`. Current version: **0.4.0**.

## License

See [LICENSE](LICENSE).
