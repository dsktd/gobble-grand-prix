use bevy::prelude::*;
use bevy::window::MonitorSelection;
use bevy::window::PresentMode;
use bevy::window::WindowMode;
use bevy::window::WindowResolution;

fn main() {
    // Steam sets SteamDeck=1 in Game Mode; there the game fills the 1280x800 screen
    let fullscreen =
        std::env::var_os("SteamDeck").is_some() || std::env::args().any(|a| a == "--fullscreen");
    let mode = if fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
    } else {
        WindowMode::Windowed
    };
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Super Slop Kart".to_string(),
                    resolution: if fullscreen {
                        WindowResolution::new(1280, 800).with_scale_factor_override(1.0)
                    } else {
                        (1280, 800).into()
                    },
                    mode,
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            }),
            super_slop_kart::plugin,
        ))
        .run();
}
