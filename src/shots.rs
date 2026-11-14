//! Screenshots: F12 any time, plus a scripted tour for `gobble shots <track>`.

use bevy::prelude::*;
use bevy::render::view::screenshot::Screenshot;
use bevy::render::view::screenshot::save_to_disk;

use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, key);
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("shots") {
        return;
    }
    let track = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(0);
    if args.get(3).map(String::as_str) == Some("low") {
        app.insert_resource(Options {
            high_quality: false,
            ..default()
        });
    }
    let racer = args
        .get(4)
        .and_then(|a| a.parse().ok())
        .unwrap_or(track * 2 % ROSTER.len());
    app.insert_resource(Tour {
        racer,
        track,
        clock: 0.0,
        step: 0,
    });
    app.add_plugins((
        bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
        bevy::diagnostic::LogDiagnosticsPlugin::default(),
    ));
    app.add_systems(Update, (tour, autopilot));
}

#[derive(Resource, Clone, Debug)]
struct Tour {
    racer: usize,
    track: usize,
    clock: f32,
    step: usize,
}

fn shoot(
    commands: &mut Commands,
    name: &str,
) {
    let _ = std::fs::create_dir_all("screenshots");
    let path = format!("screenshots/{name}.png");
    info!("Screenshot {path}");
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

fn key(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut count: Local<u32>,
) {
    if keys.just_pressed(KeyCode::F12) {
        *count += 1;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        shoot(&mut commands, &format!("shot-{stamp}-{}", *count));
    }
}

/// Seat 0 drives itself during the tour.
fn autopilot(
    mut commands: Commands,
    humans: Query<Entity, Added<Human>>,
) {
    for id in &humans {
        commands.entity(id).insert((Driver::Cpu, Cpu::new(1.0)));
    }
}

fn tour(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut tour: ResMut<Tour>,
    mut setup: ResMut<Setup>,
    mut next: ResMut<NextState<GameState>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    clock: Option<Res<RaceClock>>,
    mut exit: MessageWriter<AppExit>,
) {
    tour.clock += time.delta_secs();
    let t = tour.clock;
    let name = TRACKS[tour.track].name.to_lowercase().replace(' ', "-");
    match tour.step {
        0 if t > 2.0 => shoot(&mut commands, "menu"),
        1 if t > 2.5 => next.set(GameState::Options),
        2 if t > 4.0 => shoot(&mut commands, "options"),
        3 if t > 4.5 => {
            setup.racers[0] = tour.racer;
            next.set(GameState::PickRacer);
        }
        4 if t > 6.0 => shoot(&mut commands, &format!("racer-{name}")),
        5 if t > 6.5 => {
            setup.track = tour.track;
            next.set(GameState::Race);
        }
        6 if t > 9.0 => shoot(&mut commands, &format!("{name}-countdown")),
        7 if t > 16.0 => shoot(&mut commands, &format!("{name}-racing")),
        8 if t > 24.0 => {
            shoot(&mut commands, &format!("{name}-pack"));
            virtual_time.set_relative_speed(4.0);
        }
        9 if clock.is_some_and(|c| c.phase == Phase::Done) => {
            virtual_time.set_relative_speed(1.0);
            tour.clock = 1000.0;
        }
        10 if t > 1001.0 => shoot(&mut commands, &format!("{name}-results")),
        11 if t > 1002.5 => {
            exit.write(AppExit::Success);
        }
        _ => return,
    }
    tour.step += 1;
}
