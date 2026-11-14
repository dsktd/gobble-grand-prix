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
    // A track index, or r<seed> for a generated track
    let arg = args.get(2).map_or("0", String::as_str);
    let (track, seed) = arg.strip_prefix('r').map_or_else(
        || (arg.parse().unwrap_or(0).min(TRACKS.len() - 1), 1),
        |seed| (TRACKS.len(), seed.parse().unwrap_or(1)),
    );
    // Optional 6th argument: "diagonal" for the diagonal two-player split
    app.insert_resource(Options {
        high_quality: args.get(3).map(String::as_str) != Some("low"),
        split: match args.get(6).map(String::as_str) {
            Some("diagonal") => Split::Diagonal,
            Some("side") => Split::SideBySide,
            _ => Split::TopBottom,
        },
        ..default()
    });
    let seats: usize = args
        .get(5)
        .and_then(|a| a.parse().ok())
        .unwrap_or(1)
        .clamp(1, 4);
    if seats > 1 {
        app.insert_resource(Seats(vec![Device::Bot; seats]));
        app.insert_resource(Setup {
            racers: (0..seats).map(|s| s * 2 % ROSTER.len()).collect(),
            track,
            seed,
            last_seed: seed,
        });
    }
    let racer = args
        .get(4)
        .and_then(|a| a.parse().ok())
        .unwrap_or(track * 2 % ROSTER.len());
    app.insert_resource(Tour {
        racer,
        track,
        seed,
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
    seed: u32,
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
            .map_or(0, |d| d.as_secs());
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
    mut whats: ResMut<Whats>,
    mut exit: MessageWriter<AppExit>,
) {
    tour.clock += time.delta_secs();
    let t = tour.clock;
    let name = match TRACKS.get(tour.track) {
        Some(def) => def.name.to_lowercase().replace(' ', "-"),
        None => format!("random-{}", tour.seed),
    };
    match tour.step {
        0 if t > 2.0 => shoot(&mut commands, "menu"),
        1 if t > 2.5 => whats.open = true,
        2 if t > 3.5 => shoot(&mut commands, "news"),
        3 if t > 4.0 => {
            whats.open = false;
            next.set(GameState::Options);
        }
        4 if t > 5.5 => shoot(&mut commands, "options"),
        5 if t > 6.0 => next.set(GameState::Lobby),
        6 if t > 7.0 => shoot(&mut commands, "lobby"),
        7 if t > 7.5 => {
            setup.racers[0] = tour.racer;
            next.set(GameState::PickRacer);
        }
        8 if t > 9.0 => shoot(&mut commands, &format!("racer-{name}")),
        9 if t > 9.5 => {
            setup.track = tour.track;
            next.set(GameState::PickTrack);
        }
        10 if t > 11.0 => shoot(&mut commands, &format!("track-{name}")),
        11 if t > 11.5 => {
            // The track page rolls a fresh seed; race the one asked for
            setup.seed = tour.seed;
            next.set(GameState::Race);
        }
        12 if t > 14.0 => shoot(&mut commands, &format!("{name}-countdown")),
        13 if t > 21.0 => shoot(&mut commands, &format!("{name}-racing")),
        14 if t > 29.0 => {
            shoot(&mut commands, &format!("{name}-pack"));
            virtual_time.set_relative_speed(2.0);
        }
        15 if clock.is_some_and(|c| c.phase == Phase::Done) => {
            virtual_time.set_relative_speed(1.0);
            tour.clock = 1000.0;
        }
        16 if t > 1001.0 => shoot(&mut commands, &format!("{name}-results")),
        17 if t > 1002.5 => {
            exit.write(AppExit::Success);
        }
        _ => return,
    }
    tour.step += 1;
}
