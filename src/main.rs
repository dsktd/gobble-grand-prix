use bevy::prelude::*;
use bevy::window::MonitorSelection;
use bevy::window::PresentMode;
use bevy::window::WindowMode;

fn main() {
    // Steam sets SteamDeck=1 in Game Mode; there the game fills the 1280x800 screen
    let fullscreen = std::env::var_os("SteamDeck").is_some()
        || std::env::args().any(|a| a == "--fullscreen");
    let mode = match fullscreen {
        true => WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        false => WindowMode::Windowed,
    };
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Gobble Grand Prix".to_string(),
                    resolution: (1280, 800).into(),
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
