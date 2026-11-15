#![allow(clippy::type_complexity)]

mod attract;
mod audio;
mod bench;
mod camera;
mod cpu;
mod hud;
mod input;
mod items;
mod kart;
#[cfg(test)]
mod layout_test;
mod menu;
mod minimap;
mod music;
mod names;
mod options_file;
mod phone;
mod phone_guard;
mod prompts;
mod race;
mod roster;
mod scenery;
mod shots;
mod style;
mod track;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub(crate) mod prelude {
    pub use super::GameState;
    pub use super::Options;
    pub use super::Setup;
    pub use super::Split;
    pub use super::attract::prelude::*;
    pub use super::audio::prelude::*;
    pub use super::camera::prelude::*;
    pub use super::cpu::prelude::*;
    pub use super::input::prelude::*;
    pub use super::items::prelude::*;
    pub use super::kart::prelude::*;
    pub use super::menu::prelude::*;
    pub use super::names::prelude::*;
    pub use super::phone::prelude::*;
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
    /// Players join seats with their own pad or the keyboard.
    Lobby,
    PickRacer,
    PickTrack,
    Race,
}

/// What the player picked on the title screen.
#[derive(Resource, Clone, Debug)]
pub struct Setup {
    /// Racer per local player slot. One slot until split screen lands.
    pub racers: Vec<usize>,
    /// Seed of the random track to race.
    pub seed: u32,
    /// The seed raced before this one, to replay a good track.
    pub last_seed: u32,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            racers: vec![0],
            seed: 1,
            last_seed: 1,
        }
    }
}

/// Player-chosen settings from the options menu.
#[derive(Resource, Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// 0 to 10.
    pub volume: u8,
    pub high_quality: bool,
    /// Camera smoothing, 0 (snappy) to 10 (floaty).
    pub smoothing: u8,
    /// Small camera shake on hits. Off by default for motion comfort.
    pub shake: bool,
    /// Music volume, 0 to 10, on top of the main volume.
    pub music: u8,
    /// How two players share the screen.
    pub split: Split,
}

/// How two seats share the window. Three and four seats always use the 2x2 grid.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Split {
    #[default]
    TopBottom,
    SideBySide,
    /// Cut corner to corner, top left to bottom right: P1 top right, P2 bottom left.
    Diagonal,
}
impl Split {
    /// The next mode, wrapping, in `step` direction.
    #[must_use]
    pub fn cycle(
        self,
        step: i32,
    ) -> Self {
        const ALL: [Split; 3] = [Split::TopBottom, Split::SideBySide, Split::Diagonal];
        let at = ALL.iter().position(|s| *s == self).unwrap_or(0) as i32;
        ALL[(at + step).rem_euclid(3) as usize]
    }
}
impl Default for Options {
    fn default() -> Self {
        Self {
            volume: 7,
            high_quality: true,
            smoothing: 5,
            shake: false,
            music: 6,
            split: Split::TopBottom,
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_state::<GameState>();
    app.init_resource::<Setup>();
    app.insert_resource(options_file::load());
    app.add_systems(Update, options_file::autosave);
    app.add_plugins((
        attract::plugin,
        audio::plugin,
        camera::plugin,
        cpu::plugin,
        hud::plugin,
        input::plugin,
        items::plugin,
        kart::plugin,
        menu::plugin,
        minimap::plugin,
        music::plugin,
        names::plugin,
        phone::plugin,
    ));
    app.add_plugins((
        race::plugin,
        roster::plugin,
        scenery::plugin,
        prompts::plugin,
        shots::plugin,
        bench::plugin,
        style::plugin,
        track::plugin,
    ));
}
