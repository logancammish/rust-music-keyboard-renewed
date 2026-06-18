use iced::{
    alignment,
    border::Radius,
    font::Weight,
    widget::{
        self, button, checkbox, column, container, pick_list, row, scrollable, slider, text,
        text_input, toggler, MouseArea, Space,
    },
    Background, Border, Color, Element, Font, Length, Shadow, Theme, Vector,
};
use crate::{Chord, Message, Note, Program, ScaleType};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentMenu {
    Standard,
    Advanced,
    Help,
}

// Tokyo Night accent colours.
const ACCENT: Color = Color::from_rgb(0.478, 0.635, 0.968); // #7aa2f7
const MUTED: Color = Color::from_rgb(0.59, 0.63, 0.76);
const REC: Color = Color::from_rgb(0.96, 0.43, 0.50); // #f7768e

// Piano key heights (widths are proportional to the window, see `piano`).
const NATURAL_HEIGHT: f32 = 240.0;
const ACCIDENTAL_HEIGHT: f32 = 144.0;

// allows Note to be displayed (and, via the blanket impl, converted to String)
impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Note::C => "C",
            Note::Csharp => "C#",
            Note::D => "D",
            Note::Dsharp => "D#",
            Note::E => "E",
            Note::F => "F",
            Note::Fsharp => "F#",
            Note::G => "G",
            Note::Gsharp => "G#",
            Note::A => "A",
            Note::Asharp => "A#",
            Note::B => "B",
            Note::None => "No scale",
        };
        f.write_str(label)
    }
}

fn bold() -> Font {
    Font {
        weight: Weight::Bold,
        ..Font::default()
    }
}

fn lerp(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

impl Program {
    // Styling for a piano key. `glow` (0..=1) animates the highlight, fading
    // out smoothly after a key is released.
    fn key_style(glow: f32, natural: bool, in_scale: bool) -> button::Style {
        let base = if natural {
            if in_scale {
                Color::from_rgb(0.93, 0.94, 0.98)
            } else {
                Color::from_rgb(0.56, 0.58, 0.66)
            }
        } else if in_scale {
            Color::from_rgb(0.10, 0.11, 0.16)
        } else {
            Color::from_rgb(0.22, 0.23, 0.30)
        };

        let background = lerp(base, ACCENT, glow);
        let text_color = if natural || glow > 0.45 {
            Color::from_rgb(0.07, 0.08, 0.12)
        } else {
            Color::from_rgb(0.85, 0.87, 0.93)
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: Color::from_rgb(0.04, 0.04, 0.07),
                width: 1.0,
                radius: Radius::from(5),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                offset: Vector::new(0.0, 3.0),
                blur_radius: 6.0,
            },
            ..button::Style::default()
        }
    }

    // Soft "card" background used to group related controls.
    fn card_style(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.background.weak.color)),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.04),
                width: 1.0,
                radius: Radius::from(14),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.30),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 14.0,
            },
            ..container::Style::default()
        }
    }

    pub fn get_ui_information(&self) -> Element<'_, Message> {
        let content = match self.current_menu {
            CurrentMenu::Standard => self.standard_ui(),
            CurrentMenu::Advanced => self.advanced_ui(),
            CurrentMenu::Help => self.help_ui(),
        };

        // Constrain the content to a comfortable column width (max_width on a
        // Container is honoured, unlike on a Column) and centre it. A scrollable
        // wrapper keeps everything reachable on small windows without clipping.
        let body = container(content).max_width(720);
        let centered = container(body).center_x(Length::Fill).padding(22);

        container(scrollable(centered).width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn tab(&self, label: &str, target: CurrentMenu) -> Element<'_, Message> {
        let active = self.current_menu == target;
        let style: fn(&Theme, button::Status) -> button::Style =
            if active { button::primary } else { button::text };
        button(text(label.to_string()).size(15))
            .style(style)
            .padding([6, 14])
            .on_press(Message::SwitchMenu(target))
            .into()
    }

    fn header(&self) -> Element<'_, Message> {
        let tabs = row![
            self.tab("Play", CurrentMenu::Standard),
            self.tab("Advanced", CurrentMenu::Advanced),
            self.tab("Help", CurrentMenu::Help),
        ]
        .spacing(6);

        column![
            text("Rust Music Keyboard Renewed").size(26).font(bold()),
            text("A playable polyphonic synth keyboard")
                .size(13)
                .color(MUTED),
            Space::new().height(Length::Fixed(8.0)),
            tabs,
        ]
        .spacing(2)
        .padding([2, 6])
        .into()
    }

    fn label(text_str: &str) -> Element<'_, Message> {
        text(text_str.to_string())
            .width(Length::Fixed(118.0))
            .color(MUTED)
            .into()
    }

    fn controls(&self) -> Element<'_, Message> {
        let mut rows = column![].spacing(14);

        // Hold-to-play toggle. When on, the note-length control is irrelevant
        // (length follows how long you hold the key) so it is hidden.
        rows = rows.push(
            toggler(self.hold_mode)
                .label("Hold to play  (note length follows your press)")
                .on_toggle(|_| Message::ToggleHold)
                .spacing(12)
                .size(22),
        );

        if !self.hold_mode {
            rows = rows.push(
                row![
                    Self::label("Note length"),
                    slider(1.0..=5.0, self.note_length, Message::NoteLengthChange),
                    text(format!("{}", Self::get_note_length(self.note_length)))
                        .width(Length::Fixed(90.0)),
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
            );
        }

        rows = rows
            .push(
                row![
                    Self::label("BPM"),
                    slider(10.0..=300.0, self.bpm, Message::BpmChange),
                    text_input("120", &self.custom_bpm)
                        .on_input(Message::CustomBpmChange)
                        .padding(6)
                        .width(Length::Fixed(60.0)),
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
            )
            .push(
                row![
                    Self::label("Volume"),
                    slider(0.0..=100.0, self.volume, Message::VolumeChange),
                    text(format!("{}%", self.volume.round() as i32)).width(Length::Fixed(90.0)),
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
            )
            .push(
                row![
                    Self::label("Octave"),
                    button(text("−"))
                        .style(button::secondary)
                        .on_press(Message::OctaveChange((self.octave - 1.0).max(0.0))),
                    text(format!("{}", self.octave as i32))
                        .size(18)
                        .width(Length::Fixed(34.0))
                        .align_x(alignment::Horizontal::Center),
                    button(text("+"))
                        .style(button::secondary)
                        .on_press(Message::OctaveChange((self.octave + 1.0).min(6.0))),
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
            );

        let scale_options: Vec<Note> = std::iter::once(Note::None).chain(Note::NOTES).collect();
        rows = rows.push(
            row![
                Self::label("Scale"),
                pick_list(
                    scale_options,
                    Some(self.selected_scale.unwrap_or(Note::None)),
                    Message::Scale,
                )
                .width(Length::Fixed(130.0)),
                pick_list(
                    ScaleType::ALL.to_vec(),
                    Some(self.scale_type),
                    Message::ScaleTypeChange,
                )
                .width(Length::Fixed(110.0)),
                checkbox(self.play_chords)
                    .label("Play triads")
                    .on_toggle(|_| Message::ToggleChords)
                    .spacing(8),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
        );

        container(rows)
            .style(Self::card_style)
            .padding(20)
            .width(Length::Fill)
            .into()
    }

    fn piano_key<'a>(
        &self,
        note: Note,
        label: String,
        natural: bool,
        width: Length,
        height: f32,
    ) -> Element<'a, Message> {
        let glow = self.key_glow.get(&note).copied().unwrap_or(0.0);
        let in_scale = Chord::is_note_in_scale(self, note);
        let size = if natural { 18 } else { 13 };

        MouseArea::new(
            button(
                text(label)
                    .size(size)
                    .align_x(alignment::Horizontal::Center)
                    .align_y(alignment::Vertical::Bottom)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .font(bold()),
            )
            .style(move |_theme, _status| Self::key_style(glow, natural, in_scale))
            .width(width)
            .height(Length::Fixed(height))
            .padding(4),
        )
        .on_press(Message::Play(note))
        .on_release(Message::EndPlaying(note))
        .on_exit(Message::EndPlaying(note))
        .into()
    }

    // The piano scales to the available width using proportional `FillPortion`
    // widths. White keys are 12 units wide and black keys 8, so the black keys
    // stay centred on the white-key boundaries at any window size.
    fn piano(&self) -> Element<'_, Message> {
        let octave = self.octave;
        let white = Length::FillPortion(12);
        let black = Length::FillPortion(8);

        let naturals = row![
            self.piano_key(Note::C, format!("C{octave}"), true, white, NATURAL_HEIGHT),
            self.piano_key(Note::D, format!("D{octave}"), true, white, NATURAL_HEIGHT),
            self.piano_key(Note::E, format!("E{octave}"), true, white, NATURAL_HEIGHT),
            self.piano_key(Note::F, format!("F{octave}"), true, white, NATURAL_HEIGHT),
            self.piano_key(Note::G, format!("G{octave}"), true, white, NATURAL_HEIGHT),
            self.piano_key(Note::A, format!("A{octave}"), true, white, NATURAL_HEIGHT),
            self.piano_key(Note::B, format!("B{octave}"), true, white, NATURAL_HEIGHT),
        ]
        .spacing(0);

        let accidentals = row![
            portion(8),
            self.piano_key(Note::Csharp, "Db\nC#".to_string(), false, black, ACCIDENTAL_HEIGHT),
            portion(4),
            self.piano_key(Note::Dsharp, "Eb\nD#".to_string(), false, black, ACCIDENTAL_HEIGHT),
            portion(16),
            self.piano_key(Note::Fsharp, "Gb\nF#".to_string(), false, black, ACCIDENTAL_HEIGHT),
            portion(4),
            self.piano_key(Note::Gsharp, "Ab\nG#".to_string(), false, black, ACCIDENTAL_HEIGHT),
            portion(4),
            self.piano_key(Note::Asharp, "Bb\nA#".to_string(), false, black, ACCIDENTAL_HEIGHT),
            portion(8),
        ]
        .spacing(0);

        container(widget::stack![naturals, accidentals])
            .width(Length::Fill)
            .into()
    }

    fn recorder(&self) -> Element<'_, Message> {
        let recording = self.is_recording();

        let (label, style): (&str, fn(&Theme, button::Status) -> button::Style) = if recording {
            ("Stop recording", button::danger)
        } else {
            ("Start recording", button::primary)
        };

        // Pulsing dot that breathes while recording.
        let pulse = if recording {
            0.45 + 0.55 * (0.5 + 0.5 * (self.clock * 5.0).sin())
        } else {
            0.18
        };
        let dot = container(Space::new())
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(move |_theme| container::Style {
                background: Some(Background::Color(Color { a: pulse, ..REC })),
                border: Border {
                    radius: Radius::from(7),
                    ..Border::default()
                },
                ..container::Style::default()
            });

        let top = row![
            dot,
            button(text(label.to_string()))
                .style(style)
                .on_press(Message::ToggleRecording),
            Space::new().width(Length::Fill),
            text(format!("Recorded: {:.2}s", self.recording.elapsed()))
                .size(16)
                .color(MUTED),
        ]
        .align_y(alignment::Vertical::Center)
        .spacing(12);

        let name_row = row![
            Self::label("Save as"),
            text_input("recording", &self.file_name)
                .on_input(Message::FileNameChange)
                .padding(6)
                .width(Length::Fill),
            text(".mid").color(MUTED),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center);

        let mut content = column![top, name_row].spacing(12);

        if let Some(saved) = &self.last_saved {
            content = content.push(text(format!("Last saved: {saved}")).size(12).color(MUTED));
        }

        container(content)
            .style(Self::card_style)
            .padding(16)
            .width(Length::Fill)
            .into()
    }

    fn standard_ui(&self) -> Element<'_, Message> {
        column![
            self.header(),
            self.controls(),
            self.piano(),
            self.recorder(),
        ]
        .spacing(18)
        .into()
    }

    fn advanced_ui(&self) -> Element<'_, Message> {
        let s = &self.settings;

        let toggles = column![
            toggler(s.info_popup)
                .label("Show the help screen on startup")
                .on_toggle(Message::SetInfoPopup)
                .spacing(12)
                .size(22),
            toggler(s.default_hold_mode)
                .label("Start in hold-to-play mode")
                .on_toggle(Message::SetDefaultHold)
                .spacing(12)
                .size(22),
        ]
        .spacing(12);

        let sliders = column![
            row![
                Self::label("Attack"),
                slider(1.0..=120.0, s.attack_ms as f32, Message::AttackChange),
                text(format!("{} ms", s.attack_ms)).width(Length::Fixed(80.0)),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
            row![
                Self::label("Release"),
                slider(1.0..=600.0, s.release_ms as f32, Message::ReleaseChange),
                text(format!("{} ms", s.release_ms)).width(Length::Fixed(80.0)),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
            row![
                Self::label("Master gain"),
                slider(0.02..=0.5, s.master_gain, Message::GainChange).step(0.01),
                text(format!("{:.2}", s.master_gain)).width(Length::Fixed(80.0)),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
            row![
                Self::label("Save folder"),
                text_input("(default Documents folder)", &s.output_dir)
                    .on_input(Message::OutputDirChange)
                    .padding(6)
                    .width(Length::Fill),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(12);

        let status = if self.settings_saved {
            text("Saved to config/settings.json").size(13).color(MUTED)
        } else {
            text("Unsaved changes").size(13).color(REC)
        };

        let actions = row![
            button(text("Save settings"))
                .style(button::primary)
                .on_press(Message::SaveSettings),
            button(text("Reset to defaults"))
                .style(button::secondary)
                .on_press(Message::ResetSettings),
            Space::new().width(Length::Fill),
            status,
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center);

        let card = container(
            column![
                text("Advanced Settings").size(22).font(bold()),
                text("Lower the master gain if simultaneous notes distort. Longer release makes notes fade out more gently.")
                    .size(13)
                    .color(MUTED),
                toggles,
                sliders,
                actions,
            ]
            .spacing(20),
        )
        .style(Self::card_style)
        .padding(24)
        .width(Length::Fill);

        column![self.header(), card].spacing(18).into()
    }

    fn help_ui(&self) -> Element<'_, Message> {
        let steps = column![
            text("Getting started").size(24).font(bold()),
            text(
                "1.  Choose hold-to-play (length follows your press) or pick a fixed note length.\n\
                 2.  Set the BPM for fixed-length notes.\n\
                 3.  Play by clicking the keys or using your computer keyboard.\n\
                 4.  Change octave with the on-screen buttons or the [ and ] keys.\n\
                 5.  Pick a scale to highlight its notes, and toggle Major / Minor.\n\
                 6.  Name your file and press Record to export it as a MIDI file."
            )
            .size(15),
        ]
        .spacing(14);

        // Two rows of six so the legend stays readable on narrow windows.
        let legend_pairs = [
            ("A", "C"), ("W", "C#"), ("S", "D"), ("R", "D#"),
            ("D", "E"), ("F", "F"), ("T", "F#"), ("G", "G"),
            ("Y", "G#"), ("H", "A"), ("U", "A#"), ("J", "B"),
        ];
        let key_cap = |k: &str, n: &str| {
            container(
                column![
                    text(k.to_string()).size(16).font(bold()),
                    text(n.to_string()).size(12).color(MUTED),
                ]
                .align_x(alignment::Horizontal::Center)
                .spacing(2),
            )
            .style(Self::card_style)
            .padding(8)
            .width(Length::Fixed(46.0))
            .align_x(alignment::Horizontal::Center)
        };
        let mut top_row = row![].spacing(8);
        let mut bottom_row = row![].spacing(8);
        for (i, (k, n)) in legend_pairs.iter().enumerate() {
            if i < 6 {
                top_row = top_row.push(key_cap(k, n));
            } else {
                bottom_row = bottom_row.push(key_cap(k, n));
            }
        }
        let legend = column![top_row, bottom_row].spacing(8);

        let files = column![
            text("Recordings are saved to your chosen \"Save as\" name inside the").size(13).color(MUTED),
            text("save folder set in Advanced Settings (defaults shown below):").size(13).color(MUTED),
            text(
                "Windows:  C:\\Users\\USERNAME\\Documents\\RustMusicKeyboardRenewed\\\n\
                 Linux:    /tmp/RustMusicKeyboardRenewed/\n\
                 Other:    ./RustMusicKeyboardRenewed/"
            )
            .size(13),
        ]
        .spacing(6);

        let card = container(
            column![
                steps,
                text("Keyboard mapping").size(18).font(bold()),
                legend,
                files,
                container(
                    button(text("Start playing"))
                        .style(button::primary)
                        .on_press(Message::SwitchMenu(CurrentMenu::Standard))
                )
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Center),
            ]
            .spacing(22),
        )
        .style(Self::card_style)
        .padding(28)
        .width(Length::Fill);

        column![self.header(), card].spacing(18).into()
    }
}

// A flexible spacer measured in the same proportional units as the piano keys.
fn portion(units: u16) -> Space {
    Space::new().width(Length::FillPortion(units))
}
