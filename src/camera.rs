//! One chase camera per human seat.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::ChaseCam;
    pub use super::seat_rect;
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn.after(spawn_track));
    app.add_systems(
        Update,
        (chase.after(KartSet::Physics), viewports).run_if(in_state(GameState::Race)),
    );
}

/// Follows the kart of the human in this seat.
#[derive(Component, Copy, Clone, Debug)]
pub struct ChaseCam(pub usize);

/// Smoothed camera state, critically damped for motion comfort.
#[derive(Component, Clone, Debug, Default)]
struct Rig {
    yaw: f32,
    yaw_vel: f32,
    focus: Vec2,
    focus_vel: Vec2,
    height: f32,
    height_vel: f32,
    fov: f32,
    fov_vel: f32,
    shake: f32,
    ready: bool,
}

/// Camera tuning, chosen for players prone to motion sickness.
struct Tune;
impl Tune {
    const DISTANCE: f32 = 8.0;
    const HEIGHT: f32 = 3.6;
    /// How far ahead the fixed-pitch look target sits.
    const LOOK_AHEAD: f32 = 12.0;
    const LOOK_HEIGHT: f32 = 1.2;
    /// Yaw lag in seconds at smoothing 0 and the step per smoothing notch (5 gives 0.30 s).
    const YAW_LAG: f32 = 0.15;
    const YAW_LAG_STEP: f32 = 0.03;
    /// At most 90 degrees a second.
    const YAW_RATE: f32 = std::f32::consts::FRAC_PI_2;
    const FOCUS_LAG: f32 = 0.12;
    const HEIGHT_LAG: f32 = 0.4;
    const FOV: f32 = 70.0;
    /// Boost widens the view by at most this many degrees, eased in over about 0.5 s.
    const BOOST_FOV: f32 = 4.0;
    const FOV_LAG: f32 = 0.17;
    const SHAKE: f32 = 0.12;
}

/// Critically damped step toward a target (the classic smooth-damp), frame-rate independent.
fn smooth_damp(
    current: f32,
    target: f32,
    velocity: &mut f32,
    lag: f32,
    max_rate: f32,
    dt: f32,
) -> f32 {
    let omega = 2.0 / lag.max(0.001);
    let x = omega * dt;
    let decay = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let limit = max_rate * lag;
    let change = (current - target).clamp(-limit, limit);
    let goal = current - change;
    let temp = (*velocity + omega * change) * dt;
    *velocity = (*velocity - omega * temp) * decay;
    let mut out = goal + (change + temp) * decay;
    // Never overshoot
    if (target - current > 0.0) == (out > target) {
        out = target;
        *velocity = (out - target) / dt.max(0.0001);
    }
    out
}

fn wrap(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn spawn(
    mut commands: Commands,
    setup: Res<Setup>,
    track: Res<Track>,
    options: Res<Options>,
) {
    let theme = track.def().theme;
    let seats = setup.racers.len();
    // One full-window camera draws every seat's HUD on top of the viewports
    commands.spawn((
        Camera2d,
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
        DespawnOnExit(GameState::Race),
    ));
    let (start, dir) = track.sample(track.length - 20.0);
    for slot in 0..setup.racers.len() {
        let mut camera = commands.spawn((
            ChaseCam(slot),
            Rig::default(),
            Camera3d::default(),
            Camera {
                order: slot as isize,
                // Only the first view clears; later ones would wipe the views drawn before
                clear_color: match slot {
                    0 => ClearColorConfig::Default,
                    _ => ClearColorConfig::None,
                },
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 70f32.to_radians(),
                // Low quality with several views draws less far
                far: match (options.high_quality, seats) {
                    (false, 2..) => 160.0,
                    _ => 1000.0,
                },
                ..default()
            }),
            Transform::from_translation(flat(start - dir * 10.0, 4.0))
                .looking_at(flat(start, 1.0), Vec3::Y),
            DespawnOnExit(GameState::Race),
        ));
        if !options.high_quality {
            camera.insert(bevy::render::view::Msaa::Off);
        }
        if options.high_quality {
            camera.insert(DistanceFog {
                color: theme.fog,
                directional_light_color: Color::NONE,
                directional_light_exponent: 8.0,
                falloff: FogFalloff::Linear {
                    start: theme.fog_end * 0.4,
                    end: theme.fog_end,
                },
            });
        }
    }
}

fn chase(
    time: Res<Time>,
    options: Res<Options>,
    karts: Query<(&Kart, &Human)>,
    cameras: Query<(&ChaseCam, &mut Rig, &mut Transform, &mut Projection)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let yaw_lag = Tune::YAW_LAG + Tune::YAW_LAG_STEP * options.smoothing as f32;
    for (cam, mut rig, mut transform, mut projection) in cameras {
        let Some((kart, _)) = karts.iter().find(|(_, human)| human.0 == cam.0) else {
            continue;
        };
        // Follow where the kart travels, not where it points, so drifts and spins stay calm
        let heading = match kart.vel.length() > 3.0 {
            true => (-kart.vel.x).atan2(-kart.vel.y),
            false => kart.yaw,
        };
        if !rig.ready {
            *rig = Rig {
                yaw: heading,
                focus: kart.pos,
                height: 0.0,
                fov: Tune::FOV,
                ready: true,
                ..default()
            };
        }

        // Yaw: unwrap the target next to the current angle, then damp with a rate cap
        let target = rig.yaw + wrap(heading - rig.yaw);
        let mut yaw_vel = rig.yaw_vel;
        rig.yaw = smooth_damp(rig.yaw, target, &mut yaw_vel, yaw_lag, Tune::YAW_RATE, dt);
        rig.yaw_vel = yaw_vel;

        let mut vx = rig.focus_vel.x;
        let mut vy = rig.focus_vel.y;
        let fx = smooth_damp(rig.focus.x, kart.pos.x, &mut vx, Tune::FOCUS_LAG, 500.0, dt);
        let fy = smooth_damp(rig.focus.y, kart.pos.y, &mut vy, Tune::FOCUS_LAG, 500.0, dt);
        rig.focus = Vec2::new(fx, fy);
        rig.focus_vel = Vec2::new(vx, vy);

        // Ground height ignores hops and bumps (the track is flat; this keeps it smooth if not)
        let mut hv = rig.height_vel;
        rig.height = smooth_damp(rig.height, 0.0, &mut hv, Tune::HEIGHT_LAG, 20.0, dt);
        rig.height_vel = hv;

        let dir = Vec2::new(-rig.yaw.sin(), -rig.yaw.cos());
        let mut eye = flat(rig.focus - dir * Tune::DISTANCE, rig.height + Tune::HEIGHT);
        let look = flat(rig.focus + dir * Tune::LOOK_AHEAD, rig.height + Tune::LOOK_HEIGHT);

        rig.shake = (rig.shake - dt).max(0.0);
        if options.shake && kart.struck {
            rig.shake = 0.3;
        }
        if rig.shake > 0.0 {
            let t = time.elapsed_secs() * 40.0;
            eye += Vec3::new(t.sin(), (t * 1.3).cos(), 0.0) * Tune::SHAKE * rig.shake / 0.3;
        }
        // Looking at a point at a fixed height keeps pitch steady and the horizon level
        *transform = Transform::from_translation(eye).looking_at(look, Vec3::Y);

        let want = match kart.boost > 0.0 {
            true => Tune::FOV + Tune::BOOST_FOV,
            false => Tune::FOV,
        };
        let mut fv = rig.fov_vel;
        rig.fov = smooth_damp(rig.fov, want, &mut fv, Tune::FOV_LAG, 30.0, dt);
        rig.fov_vel = fv;
        if let Projection::Perspective(perspective) = projection.as_mut() {
            perspective.fov = rig.fov.to_radians();
        }
    }
}

/// Where a seat's view sits in the window, as fractions from the top left.
pub fn seat_rect(
    slot: usize,
    seats: usize,
    side_by_side: bool,
) -> Rect {
    match (seats, side_by_side) {
        (0 | 1, _) => Rect::new(0.0, 0.0, 1.0, 1.0),
        (2, true) => Rect::new(slot as f32 * 0.5, 0.0, slot as f32 * 0.5 + 0.5, 1.0),
        (2, false) => Rect::new(0.0, slot as f32 * 0.5, 1.0, slot as f32 * 0.5 + 0.5),
        _ => {
            let x = (slot % 2) as f32 * 0.5;
            let y = (slot / 2) as f32 * 0.5;
            Rect::new(x, y, x + 0.5, y + 0.5)
        }
    }
}

/// Keep each seat's viewport matched to the window size.
fn viewports(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    options: Res<Options>,
    setup: Res<Setup>,
    cameras: Query<(&ChaseCam, &mut Camera)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let size = window.physical_size().as_vec2();
    let seats = setup.racers.len();
    for (cam, mut camera) in cameras {
        if seats <= 1 {
            camera.viewport = None;
            continue;
        }
        let rect = seat_rect(cam.0, seats, options.side_by_side);
        let position = (rect.min * size).as_uvec2();
        let extent = (rect.size() * size).as_uvec2().max(UVec2::ONE);
        camera.viewport = Some(bevy::camera::Viewport {
            physical_position: position,
            physical_size: extent,
            ..default()
        });
    }
}
