## Rust Music Keyboard Renewed v0.4.1

Renewed is a remastered fork of the original
[rust-music-keyboard](https://github.com/logancammish/rust-music-keyboard).

### :sparkles: New
- **Selectable sound**: pick the keyboard's voice — **Sine, Triangle, Organ, Sawtooth or Square** — from the new Sound picker on the Play tab. Your choice is saved to `config/settings.json`.
- **Live visualisers**: an animated display on the Play tab and in the recording panel that swells while you play and turns red while you record. Choose the style in **Advanced Settings** — a **Waveform** line, a full **Bars** spectrum, or switch it **Off**. The render path is a single pluggable enum, so a custom "fully-fledged" visualiser can be dropped in with one new arm.
- **Cleaner Play tab**: controls are now grouped into **Sound**, **Timing** and **Octave & Scale** sections with inline hints (e.g. "or use the `[` and `]` keys").

### :loud_sound: Audio
- New per-waveform oscillator, level-matched so switching sounds doesn't jump in loudness, while keeping the original warm additive tone as the **Organ** voice.

### :bug: Fixes
- Typing in the **file name**, **BPM** or **save folder** fields no longer fires notes — key presses captured by a focused field are ignored, and key releases are always handled so notes can't stick.
- The **BPM** field no longer snaps to 60 on a partial or out-of-range entry; it keeps exactly what you type and only adopts valid in-range tempos.
- The note-length slider now snaps cleanly to the discrete lengths (Whole → Sixteenth).

### :wrench: Internals
- New settings keys `waveform` and `visualizer` (older `config/settings.json` files still load via defaults).
- Enabled the `iced` `canvas` feature to power the visualisers.

**Windows:** download `RustMusicKeyboardRenewed_Installer-Windows-x86_64.exe` below.
*Note: This version requires admin privileges to modify the settings file*
