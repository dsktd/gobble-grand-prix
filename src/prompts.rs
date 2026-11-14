//! Button glyphs (Kenney's CC0 Input Prompts) that follow the device used last.

use bevy::asset::RenderAssetUsages;
use bevy::image::CompressedImageFormats;
use bevy::image::ImageSampler;
use bevy::image::ImageType;
use bevy::input::gamepad::GamepadEvent;
use bevy::prelude::*;

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
    Brake,
    Steer,
    Drift,
    Item,
    Quit,
}
impl Action {
    const ALL: [Action; 9] = [
        Action::Confirm,
        Action::Back,
        Action::Navigate,
        Action::Gas,
        Action::Brake,
        Action::Steer,
        Action::Drift,
        Action::Item,
        Action::Quit,
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
    const ALL: [Family; 3] = [Family::Keyboard, Family::Xbox, Family::Deck];

    fn pad() -> Self {
        match std::env::var_os("SteamDeck").is_some() {
            true => Family::Deck,
            false => Family::Xbox,
        }
    }

    /// Pads first on the Deck, keyboard first elsewhere.
    fn start() -> Self {
        match Self::pad() {
            Family::Deck => Family::Deck,
            _ => Family::Keyboard,
        }
    }
}

/// An image node showing the glyph for an action.
#[derive(Component, Copy, Clone, Debug)]
pub struct Glyph(pub Action);

#[derive(Resource, Clone, Debug)]
struct Glyphs(Vec<((Family, Action), Handle<Image>)>);

fn bytes(
    family: Family,
    action: Action,
) -> &'static [u8] {
    match (family, action) {
        (Family::Keyboard, Action::Confirm) => include_bytes!("../assets/prompts/keyboard/keyboard_enter.png").as_slice(),
        (Family::Keyboard, Action::Back) => include_bytes!("../assets/prompts/keyboard/keyboard_escape.png").as_slice(),
        (Family::Keyboard, Action::Navigate) => include_bytes!("../assets/prompts/keyboard/keyboard_arrows_all.png").as_slice(),
        (Family::Keyboard, Action::Gas) => include_bytes!("../assets/prompts/keyboard/keyboard_arrow_up.png").as_slice(),
        (Family::Keyboard, Action::Brake) => include_bytes!("../assets/prompts/keyboard/keyboard_arrow_down.png").as_slice(),
        (Family::Keyboard, Action::Steer) => include_bytes!("../assets/prompts/keyboard/keyboard_arrows_horizontal.png").as_slice(),
        (Family::Keyboard, Action::Drift) => include_bytes!("../assets/prompts/keyboard/keyboard_space.png").as_slice(),
        (Family::Keyboard, Action::Item) => include_bytes!("../assets/prompts/keyboard/keyboard_shift.png").as_slice(),
        (Family::Keyboard, Action::Quit) => include_bytes!("../assets/prompts/keyboard/keyboard_escape.png").as_slice(),
        (Family::Xbox, Action::Confirm) => include_bytes!("../assets/prompts/xbox/xbox_button_color_a.png").as_slice(),
        (Family::Xbox, Action::Back) => include_bytes!("../assets/prompts/xbox/xbox_button_color_b.png").as_slice(),
        (Family::Xbox, Action::Navigate) => include_bytes!("../assets/prompts/xbox/xbox_dpad.png").as_slice(),
        (Family::Xbox, Action::Gas) => include_bytes!("../assets/prompts/xbox/xbox_rt.png").as_slice(),
        (Family::Xbox, Action::Brake) => include_bytes!("../assets/prompts/xbox/xbox_lt.png").as_slice(),
        (Family::Xbox, Action::Steer) => include_bytes!("../assets/prompts/xbox/xbox_stick_l.png").as_slice(),
        (Family::Xbox, Action::Drift) => include_bytes!("../assets/prompts/xbox/xbox_rb.png").as_slice(),
        (Family::Xbox, Action::Item) => include_bytes!("../assets/prompts/xbox/xbox_button_color_b.png").as_slice(),
        (Family::Xbox, Action::Quit) => include_bytes!("../assets/prompts/xbox/xbox_button_view.png").as_slice(),
        (Family::Deck, Action::Confirm) => include_bytes!("../assets/prompts/steamdeck/steamdeck_button_a.png").as_slice(),
        (Family::Deck, Action::Back) => include_bytes!("../assets/prompts/steamdeck/steamdeck_button_b.png").as_slice(),
        (Family::Deck, Action::Navigate) => include_bytes!("../assets/prompts/steamdeck/steamdeck_dpad.png").as_slice(),
        (Family::Deck, Action::Gas) => include_bytes!("../assets/prompts/steamdeck/steamdeck_button_r2.png").as_slice(),
        (Family::Deck, Action::Brake) => include_bytes!("../assets/prompts/steamdeck/steamdeck_button_l2.png").as_slice(),
        (Family::Deck, Action::Steer) => include_bytes!("../assets/prompts/steamdeck/steamdeck_stick_l.png").as_slice(),
        (Family::Deck, Action::Drift) => include_bytes!("../assets/prompts/steamdeck/steamdeck_button_r1.png").as_slice(),
        (Family::Deck, Action::Item) => include_bytes!("../assets/prompts/steamdeck/steamdeck_button_b.png").as_slice(),
        (Family::Deck, Action::Quit) => include_bytes!("../assets/prompts/steamdeck/steamdeck_button_view.png").as_slice(),
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
    glyphs: Res<Glyphs>,
    nodes: Query<(Ref<Glyph>, &mut ImageNode)>,
) {
    for (glyph, mut node) in nodes {
        if !family.is_changed() && !glyph.is_added() {
            continue;
        }
        let Some((_, handle)) = glyphs.0.iter().find(|(key, _)| *key == (*family, glyph.0)) else {
            continue;
        };
        node.image = handle.clone();
    }
}
