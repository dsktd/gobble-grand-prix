use bevy::prelude::*;
use bevy::window::MonitorSelection;
use bevy::window::PresentMode;
use bevy::window::WindowMode;
use bevy::window::WindowResolution;

fn main() {
    // Steam sets SteamDeck=1 in Game Mode; there the game fills the 1280x800 screen
    let fullscreen =
        std::env::var_os("SteamDeck").is_some() || std::env::args().any(|a| a == "--fullscreen");
    let mode = match fullscreen {
        true => WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        false => WindowMode::Windowed,
    };
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Gobble Grand Prix".to_string(),
                    resolution: match fullscreen {
                        // Gamescope reports a scale of 2 on the Deck, which doubles the UI
                        true => WindowResolution::new(1280, 800).with_scale_factor_override(1.0),
                        false => (1280, 800).into(),
                    },
                    mode,
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }),
            gobble::plugin,
        ))
        .run();
}
