//! UI look in one place: sizes, colours and the small widgets every screen is built from.
//!
//! Sizes are logical pixels for a 1280x800 screen at scale 1, the Steam Deck's 7" panel.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Size;
    pub use super::Tone;
    pub use super::label;
    pub use super::panel;
    pub use super::prompt;
    pub use super::prompt_either;
}

pub(super) fn plugin(_app: &mut App) {
    //
}

/// Text sizes.
pub struct Size;
impl Size {
    pub const TITLE: f32 = 44.0;
    pub const HEADING: f32 = 40.0;
    pub const BODY: f32 = 24.0;
    pub const SMALL: f32 = 20.0;
    pub const HUD: f32 = 32.0;
    pub const PLACE: f32 = 44.0;
    pub const BANNER: f32 = 96.0;
    pub const GLYPH: f32 = 32.0;
    pub const PAD: f32 = 24.0;
    pub const GAP: f32 = 10.0;
}

/// Colours.
pub struct Tone;
impl Tone {
    pub const GOLD: Color = Color::srgb(1.0, 0.82, 0.35);
    pub const CREAM: Color = Color::srgb(0.98, 0.93, 0.82);
    pub const DIM: Color = Color::srgba(0.98, 0.93, 0.82, 0.6);
    pub const PANEL: Color = Color::srgba(0.25, 0.10, 0.04, 0.85);
    pub const MENU_BG: Color = Color::srgb(0.55, 0.22, 0.10);
}

/// A line of text with a soft shadow.
pub fn label(
    value: impl Into<String>,
    size: f32,
    color: Color,
) -> impl Bundle {
    (
        Text::new(value),
        TextFont::from_font_size(size),
        TextColor(color),
        TextShadow {
            offset: Vec2::splat(2.0),
            color: Color::srgba(0.1, 0.03, 0.0, 0.9),
        },
    )
}

/// A dark rounded box for boards and hint bars.
pub fn panel() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(Size::PAD)),
            row_gap: px(Size::GAP),
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BackgroundColor(Tone::PANEL),
    )
}

fn glyph(action: Action) -> impl Bundle {
    (
        Glyph(action),
        ImageNode::default(),
        Node {
            width: px(Size::GLYPH),
            height: px(Size::GLYPH),
            ..default()
        },
    )
}

/// A button glyph followed by what it does. The glyph follows the last used device.
pub fn prompt(
    action: Action,
    text: &str,
) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        },
        children![glyph(action), label(text, Size::SMALL, Tone::CREAM)],
    )
}

/// Two glyphs that do the same thing, then what they do.
pub fn prompt_either(
    action: Action,
    alternative: Action,
    text: &str,
) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        },
        children![
            glyph(action),
            glyph(alternative),
            label(text, Size::SMALL, Tone::CREAM)
        ],
    )
}
