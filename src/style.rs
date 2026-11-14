//! UI look in one place: the design tokens from the UI spec (colours, type, spacing) and the
//! small widgets every screen is built from.
//!
//! Sizes are px on a 1280x800 screen; [`UiScale`] maps them onto any window.

use bevy::prelude::*;
use bevy::text::FontWeight;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Size;
    pub use super::Space;
    pub use super::Tone;
    pub use super::Weight;
    pub use super::label;
    pub use super::panel;
    pub use super::prompt;
    pub use super::prompt_either;
    pub use super::text;
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, fonts);
    app.add_systems(Update, scale);
}

/// Colour tokens.
pub struct Tone;
impl Tone {
    pub const BROWN_900: Color = Color::srgb_u8(0x2A, 0x17, 0x0E);
    pub const BROWN_800: Color = Color::srgb_u8(0x3E, 0x23, 0x16);
    pub const BROWN_700: Color = Color::srgb_u8(0x5B, 0x34, 0x21);
    pub const CRANBERRY: Color = Color::srgb_u8(0xB3, 0x20, 0x3C);
    pub const PUMPKIN: Color = Color::srgb_u8(0xF2, 0x8C, 0x28);
    pub const CREAM_100: Color = Color::srgb_u8(0xFF, 0xF4, 0xE0);
    pub const CREAM_300: Color = Color::srgb_u8(0xD9, 0xC3, 0xA0);
    /// HUD chip background: brown/800 at 0.85.
    pub const CHIP: Color = Color::srgba(0x3E as f32 / 255.0, 0x23 as f32 / 255.0, 0x16 as f32 / 255.0, 0.85);

    // Older names, kept while screens move over to the spec
    pub const GOLD: Color = Tone::PUMPKIN;
    pub const CREAM: Color = Tone::CREAM_100;
    pub const DIM: Color = Tone::CREAM_300;
    pub const PANEL: Color = Tone::CHIP;
    pub const MENU_BG: Color = Tone::BROWN_900;
}

/// Type scale tokens (px).
pub struct Size;
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
    pub const GLYPH: f32 = 36.0;

    // Older names
    pub const TITLE: f32 = Size::H1;
    pub const HEADING: f32 = Size::H1;
    pub const SMALL: f32 = Size::LABEL;
    pub const HUD: f32 = Size::HUD_NUM;
    pub const PLACE: f32 = Size::HUD_PLACE;
    pub const PAD: f32 = Space::L;
    pub const GAP: f32 = Space::S;
}

/// Spacing and radius tokens (px).
pub struct Space;
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
    const SEMI_BOLD: Handle<Font> = bevy::asset::uuid_handle!("237d8670-f8dc-4997-b3af-1749191f422e");
    const BOLD: Handle<Font> = bevy::asset::uuid_handle!("b0eb91dd-b953-4bb7-bbf7-74aa3e0d1d35");

    fn handle(self) -> Handle<Font> {
        match self {
            Weight::Medium => Self::MEDIUM,
            Weight::SemiBold => Self::SEMI_BOLD,
            Weight::Bold => Self::BOLD,
        }
    }

    /// The spec's weight for a type size.
    fn of(size: f32) -> Self {
        match size {
            s if s >= Size::H2 => Weight::Bold,
            s if s <= Size::LABEL => Weight::SemiBold,
            _ => Weight::Medium,
        }
    }
}

fn fonts(mut fonts: ResMut<Assets<Font>>) {
    let faces: [(Weight, &[u8], u16); 3] = [
        (Weight::Medium, include_bytes!("../assets/fonts/Fredoka-500.ttf"), 500),
        (Weight::SemiBold, include_bytes!("../assets/fonts/Fredoka-600.ttf"), 600),
        (Weight::Bold, include_bytes!("../assets/fonts/Fredoka-700.ttf"), 700),
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
    let fit = (window.width() / 1280.0).min(window.height() / 800.0).max(0.25);
    if (ui.0 - fit).abs() > 0.001 {
        ui.0 = fit;
    }
}

/// Text in a given size, weight and colour, no shadow.
pub fn text(
    value: impl Into<String>,
    size: f32,
    weight: Weight,
    color: Color,
) -> impl Bundle {
    (
        Text::new(value),
        TextFont::from(weight.handle())
            .with_font_size(size)
            .with_font_weight(FontWeight(match weight {
                Weight::Medium => 500,
                Weight::SemiBold => 600,
                Weight::Bold => 700,
            })),
        TextColor(color),
    )
}

/// Text with the spec weight for its size and a crisp shadow, for text over the 3D view.
pub fn label(
    value: impl Into<String>,
    size: f32,
    color: Color,
) -> impl Bundle {
    (
        text(value, size, Weight::of(size), color),
        TextShadow {
            offset: Vec2::splat(2.0),
            color: Color::srgba(0.1, 0.03, 0.0, 0.9),
        },
    )
}

/// A rounded brown/800 panel.
pub fn panel() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(Space::L)),
            row_gap: px(Space::S),
            border_radius: BorderRadius::all(px(Space::RADIUS_M)),
            ..default()
        },
        BackgroundColor(Tone::BROWN_800),
    )
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
        children![glyph(action, None), text(label_text, Size::LABEL, Weight::SemiBold, Tone::CREAM_100)],
    )
}

/// Two glyphs that do the same thing, then what they do.
pub fn prompt_either(
    action: Action,
    alternative: Action,
    label_text: &str,
) -> impl Bundle {
    (
        row(),
        children![
            glyph(action, None),
            glyph(alternative, None),
            text(label_text, Size::LABEL, Weight::SemiBold, Tone::CREAM_100)
        ],
    )
}
