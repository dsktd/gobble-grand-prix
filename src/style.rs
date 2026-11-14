//! UI look in one place: the design tokens from the UI spec (colours, type, spacing) and the
//! small widgets every screen is built from.
//!
//! Sizes are px on a 1280x800 screen; [`UiScale`] maps them onto any window.

use bevy::prelude::*;
use bevy::text::FontWeight;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Fill;
    pub use super::Fitted;
    pub use super::Focus;
    pub use super::Ink;
    pub use super::Ring;
    pub use super::Size;
    pub use super::Slot;
    pub use super::Space;
    pub use super::Tone;
    pub use super::Weight;
    pub use super::bar;
    pub use super::boxed;
    pub use super::fit_text;
    pub use super::fit_text_min;
    pub use super::hint_bar;
    pub use super::pill;
    pub use super::seat_color;
    pub use super::shadowed;
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Focus>();
    app.add_systems(Startup, fonts);
    app.add_systems(Update, (scale, refit, paint));
}

/// Colour tokens.
pub struct Tone;
// The design tokens are a palette: entries stay even when nothing uses them yet.
#[allow(dead_code)]
impl Tone {
    pub const BROWN_900: Color = Color::srgb_u8(0x2A, 0x17, 0x0E);
    pub const BROWN_800: Color = Color::srgb_u8(0x3E, 0x23, 0x16);
    pub const BROWN_700: Color = Color::srgb_u8(0x5B, 0x34, 0x21);
    pub const CRANBERRY: Color = Color::srgb_u8(0xB3, 0x20, 0x3C);
    pub const PUMPKIN: Color = Color::srgb_u8(0xF2, 0x8C, 0x28);
    pub const CREAM_100: Color = Color::srgb_u8(0xFF, 0xF4, 0xE0);
    pub const CREAM_300: Color = Color::srgb_u8(0xD9, 0xC3, 0xA0);
    /// HUD chip background: brown/800 at 0.85.
    pub const CHIP: Color = Color::srgba(
        0x3E as f32 / 255.0,
        0x23 as f32 / 255.0,
        0x16 as f32 / 255.0,
        0.85,
    );

    // Older names, kept while screens move over to the spec
    pub const GOLD: Color = Self::PUMPKIN;
    pub const CREAM: Color = Self::CREAM_100;
    pub const DIM: Color = Self::CREAM_300;
    pub const PANEL: Color = Self::CHIP;
    pub const MENU_BG: Color = Self::BROWN_900;
}

/// Type scale tokens (px).
pub struct Size;
// The design tokens are a palette: entries stay even when nothing uses them yet.
#[allow(dead_code)]
impl Size {
    pub const DISPLAY: f32 = 88.0;
    pub const DISPLAY_SUB: f32 = 64.0;
    pub const H1: f32 = 40.0;
    pub const H2: f32 = 32.0;
    pub const BODY: f32 = 24.0;
    pub const LABEL: f32 = 20.0;
    pub const HUD_NUM: f32 = 32.0;
    pub const HUD_PLACE: f32 = 72.0;
    pub const BANNER: f32 = 120.0;
    pub const GLYPH: f32 = 40.0;

    // Older names
    pub const TITLE: f32 = Self::H1;
    pub const HEADING: f32 = Self::H1;
    pub const SMALL: f32 = Self::LABEL;
    pub const HUD: f32 = Self::HUD_NUM;
    pub const PLACE: f32 = Self::HUD_PLACE;
    pub const PAD: f32 = Space::L;
    pub const GAP: f32 = Space::S;
}

/// Spacing and radius tokens (px).
pub struct Space;
// The design tokens are a palette: entries stay even when nothing uses them yet.
#[allow(dead_code)]
impl Space {
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 16.0;
    pub const L: f32 = 24.0;
    pub const XL: f32 = 32.0;
    pub const XXL: f32 = 48.0;
    pub const SAFE: f32 = 48.0;
    pub const RADIUS_S: f32 = 8.0;
    pub const RADIUS_M: f32 = 16.0;
    pub const FOCUS: f32 = 4.0;
}

/// Fredoka weights shipped with the game.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Weight {
    Medium,
    SemiBold,
    Bold,
}
impl Weight {
    const MEDIUM: Handle<Font> = bevy::asset::uuid_handle!("64c9fc1f-47f1-43a4-b013-838df26f3161");
    const SEMI_BOLD: Handle<Font> =
        bevy::asset::uuid_handle!("237d8670-f8dc-4997-b3af-1749191f422e");
    const BOLD: Handle<Font> = bevy::asset::uuid_handle!("b0eb91dd-b953-4bb7-bbf7-74aa3e0d1d35");

    const fn handle(self) -> Handle<Font> {
        match self {
            Self::Medium => Self::MEDIUM,
            Self::SemiBold => Self::SEMI_BOLD,
            Self::Bold => Self::BOLD,
        }
    }
}

fn fonts(mut fonts: ResMut<Assets<Font>>) {
    let faces: [(Weight, &[u8], u16); 3] = [
        (
            Weight::Medium,
            include_bytes!("../assets/fonts/Fredoka-500.ttf"),
            500,
        ),
        (
            Weight::SemiBold,
            include_bytes!("../assets/fonts/Fredoka-600.ttf"),
            600,
        ),
        (
            Weight::Bold,
            include_bytes!("../assets/fonts/Fredoka-700.ttf"),
            700,
        ),
    ];
    for (weight, bytes, _) in faces {
        let _ = fonts.insert(weight.handle().id(), Font::from_bytes(bytes.to_vec()));
    }
}

/// Fit the 1280x800 design into the window.
fn scale(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut ui: ResMut<UiScale>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let fit = (window.width() / 1280.0)
        .min(window.height() / 800.0)
        .max(0.25);
    if (ui.0 - fit).abs() > 0.001 {
        ui.0 = fit;
    }
}

/// Advance widths of the shipped faces, to size text before the layout ever sees it.
fn face(weight: Weight) -> &'static ttf_parser::Face<'static> {
    use std::sync::OnceLock;
    static FACES: OnceLock<[ttf_parser::Face<'static>; 3]> = OnceLock::new();
    let faces = FACES.get_or_init(|| {
        let parse = |bytes: &'static [u8]| ttf_parser::Face::parse(bytes, 0).expect("Fredoka face");
        [
            parse(include_bytes!("../assets/fonts/Fredoka-500.ttf")),
            parse(include_bytes!("../assets/fonts/Fredoka-600.ttf")),
            parse(include_bytes!("../assets/fonts/Fredoka-700.ttf")),
        ]
    });
    &faces[match weight {
        Weight::Medium => 0,
        Weight::SemiBold => 1,
        Weight::Bold => 2,
    }]
}

/// Width of one line of `value` at `size` px. A little generous, so a fit is never tight.
fn measure(
    value: &str,
    size: f32,
    weight: Weight,
) -> f32 {
    let face = face(weight);
    let unit = size / f32::from(face.units_per_em());
    let advance: u32 = value
        .chars()
        .map(|c| {
            face.glyph_index(c)
                .and_then(|g| face.glyph_hor_advance(g))
                .map_or_else(|| u32::from(face.units_per_em() / 2), u32::from)
        })
        .sum();
    advance as f32 * unit * 1.04 + 1.0
}

/// Cut `value` at a char count and add an ellipsis.
fn ellipsize(
    value: &str,
    width: f32,
    size: f32,
    weight: Weight,
) -> String {
    let chars: Vec<char> = value.trim_end().chars().collect();
    for keep in (0..=chars.len()).rev() {
        let cut: String = chars[..keep].iter().collect();
        let cut = format!("{}\u{2026}", cut.trim_end());
        if measure(&cut, size, weight) <= width {
            return cut;
        }
    }
    String::new()
}

/// Greedy word wrap at `width`; words wider than a line are split.
fn wrap(
    value: &str,
    width: f32,
    size: f32,
    weight: Weight,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for paragraph in value.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let mut word = word.to_string();
            loop {
                let joined = if line.is_empty() {
                    word.clone()
                } else {
                    format!("{line} {word}")
                };
                if measure(&joined, size, weight) <= width {
                    line = joined;
                    break;
                }
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                    continue;
                }
                // One word alone is too wide: split it
                let chars: Vec<char> = word.chars().collect();
                let mut at = 1;
                while at < chars.len()
                    && measure(&chars[..=at].iter().collect::<String>(), size, weight) <= width
                {
                    at += 1;
                }
                lines.push(chars[..at].iter().collect());
                word = chars[at..].iter().collect();
                if word.is_empty() {
                    break;
                }
            }
        }
        lines.push(line);
    }
    lines
}

/// Layout tests switch this on to make every string as long and wide as a string can be.
#[cfg(test)]
pub static STRESS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
fn stressed(value: &str) -> String {
    if STRESS.load(std::sync::atomic::Ordering::Relaxed) {
        format!(
            "{value} WWWWWWWWWWWW MMMMMMMMMMMM Mmmmmmmmmmmm of the Wwwwwwwwwwww Grand Prix Championship Series"
        )
    } else {
        value.to_string()
    }
}

#[cfg(not(test))]
fn stressed(value: &str) -> String {
    value.to_string()
}

/// Text that cannot leave its box: it keeps `width` px (design px) and at most `lines` lines.
/// Too long, it first shrinks (never under `min`), then wraps, then ends in an ellipsis.
/// Change the words with [`Fitted::set`]; never write the `Text` itself.
#[derive(Component, Clone, Debug)]
pub struct Fitted {
    source: String,
    size: f32,
    min: f32,
    weight: Weight,
    width: f32,
    lines: usize,
    shown: String,
    shown_size: f32,
}
impl Fitted {
    /// New words, fitted on the next frame.
    pub fn set(
        &mut self,
        value: impl AsRef<str>,
    ) {
        if self.source != value.as_ref() {
            self.source = value.as_ref().to_string();
            let (shown, size) = self.fit();
            self.shown = shown;
            self.shown_size = size;
        }
    }

    /// The text as shown, with its size, for the current source.
    fn fit(&self) -> (String, f32) {
        let source = stressed(&self.source);
        let mut size = self.size;
        loop {
            let lines = wrap(&source, self.width, size, self.weight);
            let last = size <= self.min + 0.01;
            if lines.len() <= self.lines {
                return (lines.join("\n"), size);
            }
            if last {
                let mut lines = lines;
                lines.truncate(self.lines);
                let tail = lines.pop().unwrap_or_default();
                // The kept last line plus what was cut off, so the ellipsis lands where the text ends
                let rest = wrap(&source, self.width, size, self.weight)
                    .split_off(self.lines - 1)
                    .join(" ");
                lines.push(ellipsize(
                    format!("{tail} {rest}").trim(),
                    self.width,
                    size,
                    self.weight,
                ));
                return (lines.join("\n"), size);
            }
            size = (size - 1.0).max(self.min);
        }
    }
}

/// Text in a box `width` px wide and `lines` lines tall: the one way words reach the screen.
/// `size` is the type token; it never shrinks under the label size (or `size` itself if smaller).
pub fn fit_text(
    value: impl Into<String>,
    size: f32,
    weight: Weight,
    color: Color,
    width: f32,
    lines: usize,
) -> impl Bundle {
    fit_text_min(
        value,
        size,
        Size::LABEL.min(size),
        weight,
        color,
        width,
        lines,
    )
}

/// [`fit_text`] that may shrink down to `min` px, for the cramped corners of a HUD chip.
pub fn fit_text_min(
    value: impl Into<String>,
    size: f32,
    min: f32,
    weight: Weight,
    color: Color,
    width: f32,
    lines: usize,
) -> impl Bundle {
    let mut fitted = Fitted {
        source: value.into(),
        size,
        min,
        weight,
        width,
        lines,
        shown: String::new(),
        shown_size: size,
    };
    let (shown, shown_size) = fitted.fit();
    fitted.shown.clone_from(&shown);
    fitted.shown_size = shown_size;
    (
        Text::new(shown),
        TextFont::from(weight.handle())
            .with_font_size(shown_size)
            .with_font_weight(FontWeight(match weight {
                Weight::Medium => 500,
                Weight::SemiBold => 600,
                Weight::Bold => 700,
            })),
        TextColor(color),
        // Lines are cut by us, so the layout never wraps on its own
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        fitted,
    )
}

/// A crisp shadow for text over the 3D view.
pub const fn shadowed() -> TextShadow {
    TextShadow {
        offset: Vec2::splat(2.0),
        color: Color::srgba(0.1, 0.03, 0.0, 0.9),
    }
}

/// Push the fitted words and size into the text.
fn refit(mut texts: Query<(&Fitted, &mut Text, &mut TextFont), Changed<Fitted>>) {
    for (fitted, mut text, mut font) in &mut texts {
        if text.0 != fitted.shown {
            text.0.clone_from(&fitted.shown);
        }
        font.font_size = fitted.shown_size.into();
    }
}

/// Which row of the screen a widget answers to; it looks focused when the cursor is there.
#[derive(Component, Copy, Clone, Debug)]
pub struct Slot(pub usize);

/// Background `idle`, or `focus` while the cursor is on its [`Slot`].
#[derive(Component, Copy, Clone, Debug)]
pub struct Fill {
    pub idle: Color,
    pub focus: Color,
}

/// Text colour, same idea as [`Fill`].
#[derive(Component, Copy, Clone, Debug)]
pub struct Ink {
    pub idle: Color,
    pub focus: Color,
}

/// The 4 px cream ring shown on focus.
#[derive(Component, Copy, Clone, Debug)]
pub struct Ring;

/// Which slot is focused on this screen; `None` outside menus.
#[derive(Resource, Copy, Clone, Debug, Default)]
pub struct Focus(pub Option<usize>);

fn paint(
    focus: Option<Res<Focus>>,
    mut fills: Query<(&Slot, &Fill, &mut BackgroundColor)>,
    mut inks: Query<(&Slot, &Ink, &mut TextColor)>,
    mut rings: Query<(&Slot, &mut BorderColor), With<Ring>>,
) {
    let at = focus.and_then(|f| f.0);
    for (slot, fill, mut color) in &mut fills {
        let c = if at == Some(slot.0) {
            fill.focus
        } else {
            fill.idle
        };
        if color.0 != c {
            color.0 = c;
        }
    }
    for (slot, ink, mut color) in &mut inks {
        let c = if at == Some(slot.0) {
            ink.focus
        } else {
            ink.idle
        };
        if color.0 != c {
            color.0 = c;
        }
    }
    for (slot, mut border) in &mut rings {
        let c = if at == Some(slot.0) {
            Tone::CREAM_100
        } else {
            Color::NONE
        };
        *border = BorderColor::all(c);
    }
}

/// A fixed-size box at `left`/`top` (spec px), the way the spec lists everything.
pub fn boxed(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(left),
        top: px(top),
        width: px(width),
        height: px(height),
        ..default()
    }
}

/// A rounded rectangle.
pub fn pill(radius: f32) -> BorderRadius {
    BorderRadius::all(px(radius))
}

/// A stat bar (spec 336x32): caption, track and fill. `fill` is set by the caller (0..=200).
pub fn bar(
    parent: &mut ChildSpawnerCommands,
    left: f32,
    top: f32,
    name: &str,
    fill: impl Bundle,
) {
    parent
        .spawn(boxed(left, top, 336.0, 32.0))
        .with_children(|bar| {
            bar.spawn((
                boxed(0.0, 4.0, 108.0, 24.0),
                children![fit_text(
                    name,
                    Size::LABEL,
                    Weight::SemiBold,
                    Tone::CREAM_300,
                    108.0,
                    1
                )],
            ));
            bar.spawn((
                Node {
                    border_radius: pill(8.0),
                    ..boxed(112.0, 8.0, 200.0, 16.0)
                },
                BackgroundColor(Tone::BROWN_700),
            ));
            bar.spawn(fill);
        });
}

/// The hint bar every menu screen ends with: spec 1280x48 at the bottom of the stage.
pub fn hint_bar(
    parent: &mut ChildSpawnerCommands,
    hints: &[(Action, &str)],
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(752),
                width: px(1280),
                height: px(48),
                justify_content: JustifyContent::End,
                align_items: AlignItems::Center,
                column_gap: px(Space::XL),
                padding: UiRect::right(px(48)),
                border: UiRect::top(px(2)),
                ..default()
            },
            BackgroundColor(Tone::BROWN_900.with_alpha(0.9)),
            BorderColor::all(Tone::CREAM_300.with_alpha(0.3)),
        ))
        .with_children(|bar| {
            for (action, label) in hints {
                bar.spawn(prompt(*action, label));
            }
        });
}

fn glyph(
    action: Action,
    seat: Option<usize>,
) -> impl Bundle {
    (
        Glyph(action, seat),
        ImageNode::default(),
        Node {
            width: px(Size::GLYPH),
            height: px(Size::GLYPH),
            flex_shrink: 0.0,
            ..default()
        },
    )
}

fn row() -> Node {
    Node {
        align_items: AlignItems::Center,
        column_gap: px(Space::S),
        ..default()
    }
}

/// A button glyph followed by what it does. The glyph follows the last used device.
pub fn prompt(
    action: Action,
    label_text: &str,
) -> impl Bundle {
    (
        row(),
        children![
            glyph(action, None),
            fit_text(
                label_text,
                Size::LABEL,
                Weight::SemiBold,
                Tone::CREAM_100,
                220.0,
                1
            )
        ],
    )
}

/// Each seat's colour, shared by its HUD badge and its phone page.
pub const fn seat_color(slot: usize) -> Color {
    match slot {
        0 => Tone::PUMPKIN,
        1 => Color::srgb_u8(0x4F, 0xA3, 0xE0),
        2 => Color::srgb_u8(0x6C, 0xC2, 0x4A),
        _ => Tone::CRANBERRY,
    }
}
