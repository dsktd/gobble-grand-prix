#![allow(clippy::type_complexity)]

mod audio;
mod camera;
mod cpu;
mod hud;
mod input;
mod items;
mod kart;
mod menu;
mod race;
mod roster;
mod prompts;
mod scenery;
mod shots;
mod style;
mod track;

use bevy::prelude::*;

pub(crate) mod prelude {
    pub use super::GameState;
    pub use super::Options;
    pub use super::Setup;
    pub use super::audio::prelude::*;
    pub use super::camera::prelude::*;
    pub use super::cpu::prelude::*;
    pub use super::input::prelude::*;
    pub use super::items::prelude::*;
    pub use super::kart::prelude::*;
    pub use super::prompts::prelude::*;
    pub use super::race::prelude::*;
    pub use super::roster::prelude::*;
    pub use super::style::prelude::*;
    pub use super::track::prelude::*;
}

/// Top level screens.
#[derive(States, Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Menu,
    Options,
    PickRacer,
    PickTrack,
    Race,
}

/// What the player picked on the title screen.
#[derive(Resource, Clone, Debug)]
pub struct Setup {
    /// Racer per local player slot. One slot until split screen lands.
    pub racers: Vec<usize>,
    pub track: usize,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            racers: vec![0],
            track: 0,
        }
    }
}

/// Player-chosen settings from the options menu.
#[derive(Resource, Copy, Clone, Debug)]
pub struct Options {
    /// 0 to 10.
    pub volume: u8,
    pub high_quality: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            volume: 7,
            high_quality: true,
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_state::<GameState>();
    app.init_resource::<Setup>();
    app.init_resource::<Options>();
    app.add_plugins((
        audio::plugin,
        camera::plugin,
        cpu::plugin,
        hud::plugin,
        input::plugin,
        items::plugin,
        kart::plugin,
        menu::plugin,
        race::plugin,
        roster::plugin,
        scenery::plugin,
        prompts::plugin,
        shots::plugin,
        style::plugin,
        track::plugin,
    ));
}
