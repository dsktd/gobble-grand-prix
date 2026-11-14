//! The grid, the countdown, laps, positions and the finish.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Finished;
    pub use super::Human;
    pub use super::LAPS;
    pub use super::Phase;
    pub use super::Progress;
    pub use super::RaceClock;
}

pub const LAPS: u32 = 3;
const KARTS: usize = 6;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn.after(spawn_track));
    app.add_systems(
        Update,
        (clock, progress, places, finish, leave)
            .chain()
            .in_set(KartSet::Race),
    );
    app.add_systems(Update, pause.run_if(in_state(GameState::Race)));
    app.add_systems(
        OnExit(GameState::Race),
        |mut time: ResMut<Time<Virtual>>| time.unpause(),
    );
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Phase {
    Countdown,
    Running,
    /// Every human crossed the line, results are showing.
    Done,
}

#[derive(Resource, Clone, Debug)]
pub struct RaceClock {
    pub phase: Phase,
    /// Seconds until the start, counting down from 3.
    pub countdown: f32,
    pub time: f32,
    /// Time since the last human finished.
    pub after: f32,
}

/// A kart driven by a person, on this machine or another.
#[derive(Component, Copy, Clone, Debug)]
pub struct Human(pub usize);

#[derive(Component, Clone, Debug)]
pub struct Progress {
    pub lap: u32,
    pub along: f32,
    /// Laps and distance folded into one number for sorting.
    pub total: f32,
    /// Passed the halfway mark this lap.
    pub halfway: bool,
    pub place: usize,
    pub lap_start: f32,
    pub best_lap: Option<f32>,
}

#[derive(Component, Copy, Clone, Debug)]
pub struct Finished {
    pub time: f32,
    pub place: usize,
}

fn spawn(
    mut commands: Commands,
    setup: Res<Setup>,
    seats: Res<Seats>,
    track: Res<Track>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(RaceClock {
        phase: Phase::Countdown,
        countdown: 3.999,
        time: 0.0,
        after: 0.0,
    });

    // Humans start at the back of the grid, CPUs take the other racers
    let humans = setup.racers.len();
    let mut cpus = (0..ROSTER.len()).filter(|r| !setup.racers.contains(r));
    let mut parts = Parts {
        meshes: &mut meshes,
        materials: &mut materials,
    };
    for grid in 0..KARTS {
        let human = (KARTS - grid - 1) < humans;
        let racer = if human {
            setup.racers[KARTS - grid - 1]
        } else {
            cpus.next().unwrap_or(grid % ROSTER.len())
        };
        let row = grid / 2;
        let side = match grid % 2 {
            0 => 1.0,
            _ => -1.0,
        };
        let along = track.length - 6.0 - row as f32 * 6.0 - (grid % 2) as f32 * 2.0;
        let pos = track.offset(along, side * track.half_width * 0.45);
        let dir = track.sample(along).1;
        let yaw = (-dir.x).atan2(-dir.y);

        let spark = commands
            .spawn((
                Mesh3d(parts.meshes.add(Sphere::new(0.22))),
                MeshMaterial3d(parts.glow(Color::WHITE, 10.0)),
                Transform::from_xyz(0.0, 0.25, 1.15),
                Visibility::Hidden,
            ))
            .id();
        let mut kart = commands.spawn((
            Kart::new(racer, pos, yaw),
            KartInput::default(),
            Progress {
                lap: 0,
                along,
                total: along,
                halfway: true,
                place: grid + 1,
                lap_start: 0.0,
                best_lap: None,
            },
            Sparks(spark),
            Transform::from_translation(flat(pos, 0.0)),
            Visibility::default(),
            DespawnOnExit(GameState::Race),
        ));
        kart.add_child(spark);
        if human {
            let slot = KARTS - grid - 1;
            match seats.0.get(slot) {
                Some(Device::Bot) => kart.insert((Driver::Cpu, Cpu::new(0.95), Human(slot))),
                _ => kart.insert((Driver::Local(slot), Human(slot))),
            };
        } else {
            kart.insert((Driver::Cpu, Cpu::new(0.86 + grid as f32 * 0.018)));
        }
        let root = kart.id();
        spawn_model(&mut commands, &mut parts, root, &ROSTER[racer]);
    }
}

fn clock(
    time: Res<Time>,
    mut clock: ResMut<RaceClock>,
    mut karts: Query<&mut Kart>,
    mut sfx: MessageWriter<Sfx>,
) {
    let dt = time.delta_secs();
    match clock.phase {
        Phase::Countdown => {
            let before = clock.countdown.ceil();
            clock.countdown -= dt;
            if clock.countdown.ceil() != before && clock.countdown > 0.0 {
                sfx.write(Sfx::Beep);
            }
            if clock.countdown <= 0.0 {
                clock.phase = Phase::Running;
                sfx.write(Sfx::Go);
                for mut kart in &mut karts {
                    kart.frozen = false;
                }
            }
        }
        Phase::Running | Phase::Done => {
            clock.time += dt;
        }
    }
}

fn progress(
    track: Res<Track>,
    clock: Res<RaceClock>,
    karts: Query<(&Kart, &mut Progress)>,
) {
    let len = track.length;
    for (kart, mut progress) in karts {
        let along = track.locate(kart.pos).along;
        let delta = along - progress.along;
        if delta < -len / 2.0 && progress.halfway {
            // Crossed the line going forward
            progress.lap += 1;
            progress.halfway = false;
            if progress.lap > 1 {
                let lap = clock.time - progress.lap_start;
                progress.best_lap = Some(progress.best_lap.map_or(lap, |b: f32| b.min(lap)));
            }
            progress.lap_start = clock.time;
        } else if delta > len / 2.0 {
            // Backed over the line
            progress.lap = progress.lap.saturating_sub(1);
            progress.halfway = true;
        }
        if (along - len / 2.0).abs() < len * 0.1 {
            progress.halfway = true;
        }
        progress.along = along;
        progress.total = progress.lap as f32 * len + along;
    }
}

fn places(karts: Query<(Entity, &mut Progress, Option<&Finished>)>) {
    let mut order: Vec<(Entity, f32, Option<usize>)> = karts
        .iter()
        .map(|(id, progress, finished)| (id, progress.total, finished.map(|f| f.place)))
        .collect();
    order.sort_by(|a, b| match (a.2, b.2) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => b.1.total_cmp(&a.1),
    });
    let mut karts = karts;
    for (place, (id, _, _)) in order.iter().enumerate() {
        if let Ok((_, mut progress, _)) = karts.get_mut(*id) {
            progress.place = place + 1;
        }
    }
}

fn finish(
    mut commands: Commands,
    time: Res<Time>,
    mut clock: ResMut<RaceClock>,
    karts: Query<(Entity, &Kart, &Progress, Has<Human>), Without<Finished>>,
    finished: Query<&Finished>,
    humans: Query<Has<Finished>, With<Human>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let mut place = finished.iter().count();
    for (id, kart, progress, human) in karts {
        if progress.lap <= LAPS {
            continue;
        }
        place += 1;
        info!(
            "Finished {} place {place} time {:.2} walls {}",
            ROSTER[kart.racer].name, clock.time, kart.walls
        );
        commands.entity(id).insert(Finished {
            time: clock.time,
            place,
        });
        if human {
            // Hand the wheel to the CPU for a victory lap
            commands.entity(id).insert((Driver::Cpu, Cpu::new(0.8)));
            sfx.write(Sfx::Fanfare);
        }
    }
    if clock.phase == Phase::Running && humans.iter().all(|done| done) {
        clock.after += time.delta_secs();
        if clock.after > 2.5 {
            clock.phase = Phase::Done;
        }
    }
}

fn leave(
    clock: Res<RaceClock>,
    menu: Res<MenuInput>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut next: ResMut<NextState<GameState>>,
) {
    let quit = keys.just_pressed(KeyCode::Escape)
        || gamepads
            .iter()
            .any(|g| g.just_pressed(GamepadButton::Select));
    let done = clock.phase == Phase::Done;
    let change = keys.just_pressed(KeyCode::KeyT)
        || gamepads
            .iter()
            .any(|g| g.just_pressed(GamepadButton::North));
    if quit || (done && menu.back) {
        next.set(GameState::Menu);
    } else if done && menu.confirm {
        // Same track, same racers, fresh race
        next.set(GameState::Race);
    } else if done && change {
        next.set(GameState::PickTrack);
    }
}

/// Start, P, or the phone's pause button freeze the race.
fn pause(
    menu: Res<MenuInput>,
    mut time: ResMut<Time<Virtual>>,
) {
    if !menu.pause {
        return;
    }
    if time.is_paused() {
        time.unpause();
    } else {
        time.pause();
    }
}
