//! Button glyphs (Kenney's CC0 Input Prompts) that follow the device used last.

use bevy::asset::RenderAssetUsages;
use bevy::image::CompressedImageFormats;
use bevy::image::ImageSampler;
use bevy::image::ImageType;
use bevy::input::gamepad::GamepadEvent;
use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Action;
    pub use super::Glyph;
}

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(Family::start());
    app.add_systems(Startup, load);
    app.add_systems(PreUpdate, detect.after(bevy::input::InputSystems));
    app.add_systems(Update, show);
}

/// Something a prompt asks the player to press.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Action {
    Confirm,
    Back,
    Navigate,
    Gas,
    GasButton,
    Brake,
    BrakeButton,
    Steer,
    Drift,
    Item,
    Quit,
    /// The spare face button: Y on pads, T on the keyboard.
    Alt,
}
impl Action {
    const ALL: [Self; 12] = [
        Self::Alt,
        Self::GasButton,
        Self::BrakeButton,
        Self::Confirm,
        Self::Back,
        Self::Navigate,
        Self::Gas,
        Self::Brake,
        Self::Steer,
        Self::Drift,
        Self::Item,
        Self::Quit,
    ];
}

/// Which glyph set to draw.
#[derive(Resource, Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum Family {
    Keyboard,
    Xbox,
    Deck,
}
impl Family {
    const ALL: [Self; 3] = [Self::Keyboard, Self::Xbox, Self::Deck];

    fn pad() -> Self {
        if std::env::var_os("SteamDeck").is_some() {
            Self::Deck
        } else {
            Self::Xbox
        }
    }

    /// Pads first on the Deck, keyboard first elsewhere.
    fn start() -> Self {
        match Self::pad() {
            Self::Deck => Self::Deck,
            _ => Self::Keyboard,
        }
    }
}

/// An image node showing the glyph for an action, for one seat's device when given.
#[derive(Component, Copy, Clone, Debug)]
pub struct Glyph(pub Action, pub Option<usize>);

#[derive(Resource, Clone, Debug)]
struct Glyphs(Vec<((Family, Action), Handle<Image>)>);

const fn bytes(
    family: Family,
    action: Action,
) -> &'static [u8] {
    match (family, action) {
        (Family::Keyboard, Action::Confirm) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_enter.png").as_slice()
        }
        (Family::Keyboard, Action::Back) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_escape.png").as_slice()
        }
        (Family::Keyboard, Action::Navigate) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_arrows_all.png").as_slice()
        }
        (Family::Keyboard, Action::Gas) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_arrow_up.png").as_slice()
        }
        (Family::Keyboard, Action::GasButton) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_w.png").as_slice()
        }
        (Family::Keyboard, Action::BrakeButton) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_s.png").as_slice()
        }
        (Family::Keyboard, Action::Alt) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_t.png").as_slice()
        }
        (Family::Keyboard, Action::Brake) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_arrow_down.png").as_slice()
        }
        (Family::Keyboard, Action::Steer) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_arrows_horizontal.png").as_slice()
        }
        (Family::Keyboard, Action::Drift) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_space.png").as_slice()
        }
        (Family::Keyboard, Action::Item) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_shift.png").as_slice()
        }
        (Family::Keyboard, Action::Quit) => {
            include_bytes!("../assets/prompts/keyboard/keyboard_escape.png").as_slice()
        }
        (Family::Xbox, Action::Confirm) => {
            include_bytes!("../assets/prompts/xbox/xbox_button_color_a.png").as_slice()
        }
        (Family::Xbox, Action::Back) => {
            include_bytes!("../assets/prompts/xbox/xbox_button_color_b.png").as_slice()
        }
        (Family::Xbox, Action::Navigate) => {
            include_bytes!("../assets/prompts/xbox/xbox_dpad.png").as_slice()
        }
        (Family::Xbox, Action::Gas) => {
            include_bytes!("../assets/prompts/xbox/xbox_rt.png").as_slice()
        }
        (Family::Xbox, Action::GasButton) => {
            include_bytes!("../assets/prompts/xbox/xbox_button_color_a.png").as_slice()
        }
        (Family::Xbox, Action::BrakeButton) => {
            include_bytes!("../assets/prompts/xbox/xbox_button_color_x.png").as_slice()
        }
        (Family::Xbox, Action::Alt) => {
            include_bytes!("../assets/prompts/xbox/xbox_button_color_y.png").as_slice()
        }
        (Family::Xbox, Action::Brake) => {
            include_bytes!("../assets/prompts/xbox/xbox_lt.png").as_slice()
        }
        (Family::Xbox, Action::Steer) => {
            include_bytes!("../assets/prompts/xbox/xbox_stick_l.png").as_slice()
        }
        (Family::Xbox, Action::Drift) => {
            include_bytes!("../assets/prompts/xbox/xbox_rb.png").as_slice()
        }
        (Family::Xbox, Action::Item) => {
            include_bytes!("../assets/prompts/xbox/xbox_button_color_b.png").as_slice()
        }
        (Family::Xbox, Action::Quit) => {
            include_bytes!("../assets/prompts/xbox/xbox_button_view.png").as_slice()
        }
        (Family::Deck, Action::Confirm) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_a.png").as_slice()
        }
        (Family::Deck, Action::Back) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_b.png").as_slice()
        }
        (Family::Deck, Action::Navigate) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_dpad.png").as_slice()
        }
        (Family::Deck, Action::Gas) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_r2.png").as_slice()
        }
        (Family::Deck, Action::GasButton) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_a.png").as_slice()
        }
        (Family::Deck, Action::BrakeButton) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_x.png").as_slice()
        }
        (Family::Deck, Action::Alt) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_y.png").as_slice()
        }
        (Family::Deck, Action::Brake) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_l2.png").as_slice()
        }
        (Family::Deck, Action::Steer) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_stick_l.png").as_slice()
        }
        (Family::Deck, Action::Drift) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_r1.png").as_slice()
        }
        (Family::Deck, Action::Item) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_b.png").as_slice()
        }
        (Family::Deck, Action::Quit) => {
            include_bytes!("../assets/prompts/steamdeck/steamdeck_button_view.png").as_slice()
        }
    }
}

fn load(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let mut glyphs = Vec::new();
    for family in Family::ALL {
        for action in Action::ALL {
            let image = Image::from_buffer(
                bytes(family, action),
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                ImageSampler::linear(),
                RenderAssetUsages::RENDER_WORLD,
            )
            .unwrap();
            glyphs.push(((family, action), images.add(image)));
        }
    }
    commands.insert_resource(Glyphs(glyphs));
}

/// Switch glyph sets when another kind of device is used.
fn detect(
    keys: Res<ButtonInput<KeyCode>>,
    mut pads: MessageReader<GamepadEvent>,
    mut family: ResMut<Family>,
) {
    let pad = pads.read().any(|event| match event {
        GamepadEvent::Button(_) => true,
        GamepadEvent::Axis(axis) => axis.value.abs() > 0.5,
        GamepadEvent::Connection(_) => false,
    });
    let next = match (pad, keys.get_just_pressed().next().is_some()) {
        (true, _) => Family::pad(),
        (false, true) => Family::Keyboard,
        _ => return,
    };
    if *family != next {
        *family = next;
    }
}

fn show(
    family: Res<Family>,
    seats: Res<Seats>,
    glyphs: Res<Glyphs>,
    nodes: Query<(Ref<Glyph>, &mut ImageNode)>,
) {
    for (glyph, mut node) in nodes {
        if !family.is_changed() && !seats.is_changed() && !glyph.is_added() {
            continue;
        }
        // A seat's own device decides its glyphs; shared prompts follow the last input
        let set = match glyph.1.and_then(|slot| seats.0.get(slot)) {
            Some(Device::Keyboard) => Family::Keyboard,
            Some(Device::Gamepad(_)) => Family::pad(),
            _ => *family,
        };
        let Some((_, handle)) = glyphs.0.iter().find(|(key, _)| *key == (set, glyph.0)) else {
            continue;
        };
        node.image = handle.clone();
    }
}
