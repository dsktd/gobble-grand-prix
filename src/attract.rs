//! Attract mode: CPU karts race a real track behind the menus, filmed like a TV broadcast.

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use rand::Rng;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Attract;
    pub use super::on_track;
}

pub(super) fn plugin(app: &mut App) {
    app.init_schedule(Load);
    app.add_systems(
        Load,
        (
            spawn_track,
            crate::scenery::spawn,
            crate::items::spawn,
            crate::race::spawn,
        )
            .chain(),
    );
    app.add_systems(OnEnter(InMenus), (start, cameras).chain());
    app.add_systems(OnExit(InMenus), stop);
    app.add_systems(
        Update,
        (rotate, direct, film)
            .chain()
            .after(KartSet::Race)
            .run_if(resource_exists::<Attract>),
    );
}

/// Present while the menus show a race behind them.
#[derive(Resource, Copy, Clone, Debug)]
pub struct Attract {
    track: usize,
    /// Seconds on this track.
    time: f32,
}
impl Attract {
    /// Race sound volume behind the menus.
    pub const DUCK: f32 = 0.15;
    /// Laps the leader runs before the next track.
    const LAPS: u32 = 2;
    /// Longest stay on one track, in case the pack is slow.
    const STAY: f32 = 120.0;
}

/// Karts move: in a race, or behind the menus.
pub fn on_track(
    state: Res<State<GameState>>,
    attract: Option<Res<Attract>>,
) -> bool {
    *state.get() == GameState::Race || attract.is_some()
}

/// Spawns one track's race with the systems the real race uses.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Load;

fn start(world: &mut World) {
    let setup = world.resource::<Setup>();
    let (track, seed) = (setup.track, setup.seed);
    load(world, track, seed);
}

/// Swap the race behind the menus for a fresh all-CPU race on `track`, already rolling.
fn load(
    world: &mut World,
    track: usize,
    seed: u32,
) {
    let old: Vec<Entity> = world
        .query::<(Entity, &DespawnOnExit<GameState>)>()
        .iter(world)
        .filter(|(_, scope)| scope.0 == GameState::Race)
        .map(|(id, _)| id)
        .collect();
    for id in old {
        // Children of earlier entities in the list are already gone
        let _ = world.try_despawn(id);
    }

    // The spawners read the player's setup; hand them an empty grid on our track
    let saved = world.resource::<Setup>().clone();
    *world.resource_mut::<Setup>() = Setup {
        racers: Vec::new(),
        track,
        seed,
        last_seed: seed,
    };
    world.run_schedule(Load);
    *world.resource_mut::<Setup>() = saved;

    // Skip the countdown and its beeps
    let mut clock = world.resource_mut::<RaceClock>();
    clock.phase = Phase::Running;
    clock.countdown = 0.0;
    for mut kart in world.query::<&mut Kart>().iter_mut(world) {
        kart.frozen = false;
    }
    world.insert_resource(Attract { track, time: 0.0 });
}

fn stop(world: &mut World) {
    world.remove_resource::<Attract>();
    let old: Vec<Entity> = world
        .query::<(Entity, &DespawnOnExit<GameState>)>()
        .iter(world)
        .filter(|(_, scope)| scope.0 == GameState::Race)
        .map(|(id, _)| id)
        .collect();
    for id in old {
        let _ = world.try_despawn(id);
    }
}

/// Next track once the leader has run its laps.
fn rotate(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    let mut attract = world.resource_mut::<Attract>();
    attract.time += dt;
    let (track, time) = (attract.track, attract.time);
    let laps = world
        .query::<&Progress>()
        .iter(world)
        .map(|p| p.lap)
        .max()
        .unwrap_or(0);
    if laps > Attract::LAPS || time > Attract::STAY {
        // Generated tracks take their turn too, a new one each time
        load(world, (track + 1) % (TRACKS.len() + 1), rand::random());
        for mut director in world.query::<&mut Director>().iter_mut(world) {
            director.left = 0.0;
        }
    }
}

/// Camera moves, one per cut.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Shot {
    /// Behind and above the leader.
    Chase,
    /// Circling one kart.
    Orbit,
    /// A fixed camera by the barrier, panning as the pack passes.
    Trackside,
    /// Tracking alongside a kart from beyond the barrier.
    Dolly,
    /// Low by a drifting or boosting kart.
    Low,
}
impl Shot {
    const CALM: [Self; 4] = [Self::Chase, Self::Orbit, Self::Trackside, Self::Dolly];
}

/// The attract camera's script.
#[derive(Component, Clone, Debug)]
struct Director {
    shot: Shot,
    subject: Option<Entity>,
    /// Seconds until the next cut.
    left: f32,
    /// Seconds into this shot.
    age: f32,
    /// Which side of the kart or road the camera sits on.
    side: f32,
    /// Trackside camera position.
    spot: Vec3,
    /// Smoothed look target.
    aim: Vec3,
}

fn cameras(
    mut commands: Commands,
    track: Res<Track>,
    options: Res<Options>,
) {
    let scope = DespawnOnExit(InMenus);
    // The menu draws on top of both 3D views
    commands.spawn((
        Camera2d,
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
        scope.clone(),
    ));
    let (start, dir) = track.sample(track.length - 20.0);
    let mut camera = commands.spawn((
        Director {
            shot: Shot::Chase,
            subject: None,
            left: 0.0,
            age: 0.0,
            side: 1.0,
            spot: Vec3::ZERO,
            aim: track.lift(start, 1.0),
        },
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 50f32.to_radians(),
            ..default()
        }),
        Transform::from_translation(track.lift(start - dir * 10.0, 4.0))
            .looking_at(track.lift(start, 1.0), Vec3::Y),
        scope,
    ));
    if options.high_quality {
        camera.insert(fog(&track));
    } else {
        camera.insert(bevy::render::view::Msaa::Off);
    }
}

fn fog(track: &Track) -> DistanceFog {
    let theme = track.theme;
    DistanceFog {
        color: theme.fog,
        directional_light_color: Color::NONE,
        directional_light_exponent: 8.0,
        falloff: FogFalloff::Linear {
            start: theme.fog_end * 0.3,
            end: theme.fog_end * 0.8,
        },
    }
}

/// Cut to a new shot every 6-8 s; drifts and boosts get the low angle.
fn direct(
    time: Res<Time>,
    track: Res<Track>,
    directors: Query<(&mut Director, &mut Transform, Option<&mut DistanceFog>)>,
    karts: Query<(Entity, &Kart, &Progress)>,
) {
    let mut rng = rand::rng();
    let dt = time.delta_secs();
    for (mut director, mut transform, fogged) in directors {
        director.age += dt;
        director.left -= dt;
        let gone = director.subject.is_none_or(|id| !karts.contains(id));
        if director.left > 0.0 && !gone {
            continue;
        }
        if let Some(mut f) = fogged
            && track.is_changed()
        {
            *f = fog(&track);
        }
        let leader = karts.iter().min_by_key(|(_, _, p)| p.place);
        let Some((leader, _, lead)) = leader else {
            continue;
        };
        let action = karts
            .iter()
            .filter(|(_, k, p)| p.place <= 4 && (k.drift.is_some() || k.boost > 0.0))
            .map(|(id, _, _)| id)
            .next();
        let (shot, subject) = match action {
            Some(id) if director.shot != Shot::Low && rng.random_bool(0.6) => (Shot::Low, id),
            _ => {
                let calm: Vec<Shot> = Shot::CALM
                    .into_iter()
                    .filter(|s| *s != director.shot)
                    .collect();
                let shot = calm[rng.random_range(0..calm.len())];
                // Orbits and dollies pick from the front of the pack, the rest follow the leader
                let pick = rng.random_range(1..=3);
                let subject = match shot {
                    Shot::Orbit | Shot::Dolly => karts
                        .iter()
                        .find(|(_, _, p)| p.place == pick)
                        .map_or(leader, |(id, _, _)| id),
                    _ => leader,
                };
                (shot, subject)
            }
        };
        director.shot = shot;
        director.subject = Some(subject);
        director.left = rng.random_range(6.0..8.0);
        director.age = 0.0;
        director.side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        let ahead = lead.along + rng.random_range(30.0..45.0);
        director.spot = track.lift(
            track.offset(ahead, director.side * (track.barrier() + 4.0)),
            1.2,
        );
        // Hard cut: jump straight to the new framing
        if let Ok((_, kart, _)) = karts.get(subject) {
            let (eye, aim) = framing(&director, kart, &track);
            director.aim = aim;
            *transform = Transform::from_translation(eye).looking_at(aim, Vec3::Y);
        }
    }
}

/// Where the camera wants to be and look for the current shot.
fn framing(
    director: &Director,
    kart: &Kart,
    track: &Track,
) -> (Vec3, Vec3) {
    let pos = flat(kart.pos, kart.y);
    let fwd = flat(kart.forward(), 0.0);
    let side = Vec3::new(fwd.z, 0.0, -fwd.x) * director.side;
    match director.shot {
        Shot::Chase => (
            pos - fwd * 8.0 + side * 1.5 + Vec3::Y * 2.8,
            pos + fwd * 5.0 + Vec3::Y * 0.8,
        ),
        Shot::Orbit => {
            let angle = director.age * 0.35 * director.side + 1.0;
            let round = fwd * angle.cos() + side * angle.sin();
            (pos + round * 6.5 + Vec3::Y * 1.8, pos + Vec3::Y * 0.7)
        }
        Shot::Trackside => (director.spot, pos + Vec3::Y * 0.8),
        Shot::Dolly => {
            let along = track.locate(kart.pos).along;
            let rail = track.offset(along - 3.0, director.side * (track.barrier() + 2.5));
            (track.lift(rail, 2.2), pos + Vec3::Y * 0.8)
        }
        Shot::Low => (
            pos + side * 3.2 - fwd * 1.5 + Vec3::Y * 0.35,
            pos + fwd * 2.0 + Vec3::Y * 0.7,
        ),
    }
}

/// Glide toward the shot's framing; the eye lags a little more than the aim.
fn film(
    time: Res<Time>,
    track: Res<Track>,
    directors: Query<(&mut Director, &mut Transform)>,
    karts: Query<&Kart>,
) {
    let dt = time.delta_secs();
    for (mut director, mut transform) in directors {
        let Some(kart) = director.subject.and_then(|id| karts.get(id).ok()) else {
            continue;
        };
        let (eye, aim) = framing(&director, kart, &track);
        let (eye_rate, aim_rate) = match director.shot {
            Shot::Chase => (3.0, 5.0),
            Shot::Trackside => (0.0, 3.0),
            _ => (6.0, 8.0),
        };
        let k = 1.0 - (-dt * eye_rate).exp();
        let a = 1.0 - (-dt * aim_rate).exp();
        director.aim = director.aim.lerp(aim, a);
        transform.translation = match director.shot {
            Shot::Trackside => eye,
            _ => transform.translation.lerp(eye, k),
        };
        let look = director.aim;
        transform.look_at(look, Vec3::Y);
    }
}
