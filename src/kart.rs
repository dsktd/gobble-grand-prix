//! Arcade kart physics: throttle, steering, drifting with mini-turbos, and knockbacks.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Drift;
    pub use super::Driver;
    pub use super::Kart;
    pub use super::KartInput;
    pub use super::KartSet;
    pub use super::Sparks;
}

pub(super) fn plugin(app: &mut App) {
    app.configure_sets(
        Update,
        (KartSet::Input, KartSet::Physics, KartSet::Race)
            .chain()
            .run_if(in_state(GameState::Race)),
    );
    app.add_systems(
        Update,
        (drive, bump, pose, sparks).chain().in_set(KartSet::Physics),
    );
    app.add_systems(
        Update,
        tears
            .after(KartSet::Physics)
            .run_if(in_state(GameState::Race)),
    );
}

#[derive(SystemSet, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum KartSet {
    Input,
    Physics,
    Race,
}

/// What the driver wants this frame, from the keyboard, a pad, or the CPU.
#[derive(Component, Clone, Debug, Default)]
pub struct KartInput {
    pub throttle: f32,
    pub steer: f32,
    pub drift: bool,
    pub item: bool,
}

/// Who steers a kart. Remote drivers slot in here once LAN play lands.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq)]
pub enum Driver {
    /// A player on this machine, by local slot (0 is P1).
    Local(usize),
    Cpu,
}

#[derive(Copy, Clone, Debug)]
pub struct Drift {
    /// -1 drifting right, 1 drifting left.
    pub dir: f32,
    pub charge: f32,
}
impl Drift {
    pub const BLUE: f32 = 0.9;
    pub const ORANGE: f32 = 2.0;

    pub fn tier(&self) -> u8 {
        match self.charge {
            c if c >= Self::ORANGE => 2,
            c if c >= Self::BLUE => 1,
            _ => 0,
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct Kart {
    pub racer: usize,
    pub pos: Vec2,
    pub yaw: f32,
    pub speed: f32,
    pub vel: Vec2,
    pub drift: Option<Drift>,
    pub boost: f32,
    pub spin: f32,
    pub coma: f32,
    pub pied: f32,
    pub hop: f32,
    /// Rubber band multiplier on top speed, set by the CPU driver.
    pub pace: f32,
    pub frozen: bool,
    pub offroad: bool,
    pub item: Option<Item>,
    /// Steering after the input ramp, -1..1.
    pub steer: f32,
    /// Visual body slide in radians, eased in and out of drifts.
    pub slide: f32,
    /// Wall contacts this race, for tuning logs.
    pub walls: u32,
    on_wall: bool,
    /// Boost the last release earned, read by sound and HUD.
    pub fired: u8,
    /// Knocked into a spin this frame.
    pub struck: bool,
}
impl Kart {
    pub const TOP_SPEED: f32 = 34.0;
    pub const ACCEL: f32 = 20.0;
    /// Base steering rate in rad/s. 1.68 is 20% under the first 2.1, after Cal found
    /// turning too quick.
    pub const TURN: f32 = 1.68;
    /// Seconds for the steering input to ramp from centre to full lock.
    pub const STEER_RAMP: f32 = 0.15;
    /// Share of steering lost at top speed, so fast karts turn gentler.
    pub const HIGH_SPEED_CUT: f32 = 0.3;
    /// Scales the drift's visual slide. 1.0 was too shallow and 1.15 too much, so it
    /// sits between; it no longer touches the turn rate.
    pub const DRIFT_ANGLE: f32 = 1.05;
    /// Body slide in radians at `DRIFT_ANGLE` 1.
    pub const DRIFT_SLIDE: f32 = 0.4;
    /// Drift turn rate as a share of normal steering, before the lean adds to it. Kept
    /// under 1 so a drift arcs wider than steering and cannot spin into a donut.
    pub const DRIFT_TURN: f32 = 0.55;
    /// Extra share of normal steering when the stick leans into the drift.
    pub const DRIFT_LEAN: f32 = 0.25;
    /// Share when counter-steering: the arc widens, it never flips.
    pub const DRIFT_COUNTER: f32 = 0.3;
    /// A drift starts only above this share of top speed.
    pub const DRIFT_MIN_SPEED: f32 = 0.4;
    /// How long the body takes to straighten after a drift.
    pub const DRIFT_SETTLE: f32 = 0.2;
    /// Velocity follows the heading this fast while drifting (normal is about 10).
    pub const DRIFT_GRIP: f32 = 6.0;

    pub const fn new(
        racer: usize,
        pos: Vec2,
        yaw: f32,
    ) -> Self {
        Self {
            racer,
            pos,
            yaw,
            speed: 0.0,
            vel: Vec2::ZERO,
            drift: None,
            boost: 0.0,
            spin: 0.0,
            coma: 0.0,
            pied: 0.0,
            hop: 0.0,
            pace: 1.0,
            frozen: true,
            offroad: false,
            item: None,
            steer: 0.0,
            slide: 0.0,
            walls: 0,
            on_wall: false,
            fired: 0,
            struck: false,
        }
    }

    pub fn forward(&self) -> Vec2 {
        Vec2::new(-self.yaw.sin(), -self.yaw.cos())
    }

    pub const fn stats(&self) -> Stats {
        ROSTER[self.racer].stats
    }

    /// Knock the kart into a spin unless it is boosting through.
    pub const fn hit(
        &mut self,
        time: f32,
    ) {
        self.spin = self.spin.max(time);
        self.struck = true;
        self.drift = None;
        self.boost = 0.0;
    }
}

/// Small glowing balls behind the rear wheels while drifting.
#[derive(Component, Copy, Clone, Debug)]
pub struct Sparks(pub Entity);

fn approach(
    value: f32,
    target: f32,
    rate: f32,
) -> f32 {
    if value < target {
        (value + rate).min(target)
    } else {
        (value - rate).max(target)
    }
}

fn drive(
    time: Res<Time>,
    track: Res<Track>,
    karts: Query<(&mut Kart, &KartInput)>,
) {
    let dt = time.delta_secs().min(1.0 / 20.0);
    for (mut kart, input) in karts {
        kart.fired = 0;
        kart.struck = false;
        kart.boost = (kart.boost - dt).max(0.0);
        kart.coma = (kart.coma - dt).max(0.0);
        kart.pied = (kart.pied - dt).max(0.0);
        kart.hop = (kart.hop - dt).max(0.0);
        if kart.frozen {
            continue;
        }
        let stats = kart.stats();
        let spot = track.locate(kart.pos);
        kart.offroad = spot.lateral.abs() > track.half_width + 1.2 && !track.in_shortcut(kart.pos);

        let mut top = Kart::TOP_SPEED * stats.top_speed * kart.pace;
        if kart.offroad && kart.boost <= 0.0 {
            top *= 0.5;
        }
        if kart.coma > 0.0 {
            top *= 0.55;
        }
        if kart.boost > 0.0 {
            top *= 1.4;
        }

        if kart.spin > 0.0 {
            // Spinning out: no control, bleed speed
            kart.spin = (kart.spin - dt).max(0.0);
            kart.speed *= (-2.5 * dt).exp();
            kart.drift = None;
        } else {
            // Throttle and brakes
            let accel = Kart::ACCEL * stats.accel;
            if kart.boost > 0.0 {
                kart.speed = approach(kart.speed, top, accel * 3.0 * dt);
            } else if input.throttle > 0.1 {
                let rate = if kart.speed > top {
                    25.0
                } else {
                    accel * input.throttle * (1.2 - kart.speed / top).max(0.25)
                };
                kart.speed = approach(kart.speed, top, rate * dt);
            } else if input.throttle < -0.1 {
                kart.speed = approach(kart.speed, -top * 0.35, 40.0 * dt);
            } else {
                kart.speed = approach(kart.speed, 0.0, 8.0 * dt);
            }

            let rate = steer_rate(&mut kart, input, stats, dt);
            // Ease the body into and out of the slide
            let target = kart
                .drift
                .map_or(0.0, |d| d.dir * Kart::DRIFT_SLIDE * Kart::DRIFT_ANGLE);
            kart.slide = approach(
                kart.slide,
                target,
                Kart::DRIFT_SLIDE * Kart::DRIFT_ANGLE * dt / Kart::DRIFT_SETTLE,
            );
            kart.yaw += rate * dt;

            // Release a drift into a mini-turbo
            let release = !input.drift || kart.speed < Kart::TOP_SPEED * stats.top_speed * 0.25;
            if release && let Some(drift) = kart.drift.take() {
                let tier = drift.tier();
                kart.boost = kart.boost.max(match tier {
                    2 => 1.1,
                    1 => 0.55,
                    _ => 0.0,
                });
                kart.fired = tier;
            }
        }

        // Velocity slides toward the heading; drifting lets it slide more
        let heading = kart.forward() * kart.speed;
        let grip = match (kart.drift.is_some(), kart.spin > 0.0) {
            (_, true) => 1.5,
            (true, _) => Kart::DRIFT_GRIP,
            _ => 10.0 * stats.weight.sqrt(),
        };
        kart.vel = kart.vel.lerp(heading, (grip * dt).min(1.0));
        let step = kart.vel * dt;
        kart.pos += step;

        // Invisible walls just past the shoulders
        let spot = track.locate(kart.pos);
        if spot.lateral.abs() > track.barrier() && !track.in_shortcut(kart.pos) {
            let edge = track.offset(spot.along, track.barrier() * spot.lateral.signum());
            kart.pos = edge;
            kart.speed *= 0.6;
            kart.vel *= 0.4;
            // Count separate wall contacts, not every frame against it
            if !kart.on_wall {
                kart.walls += 1;
            }
            kart.on_wall = true;
        } else {
            kart.on_wall = false;
        }
    }
}

/// Push overlapping karts apart, heavier karts win.
fn bump(mut karts: Query<&mut Kart>) {
    let mut combos = karts.iter_combinations_mut();
    while let Some([mut a, mut b]) = combos.fetch_next() {
        let delta = b.pos - a.pos;
        let dist = delta.length();
        if dist > 2.4 || dist < 0.001 {
            continue;
        }
        let push = delta / dist * (2.4 - dist);
        let wa = a.stats().weight;
        let wb = b.stats().weight;
        a.pos -= push * wb / (wa + wb);
        b.pos += push * wa / (wa + wb);
    }
}

/// Copy physics state onto the model transform, with drift lean and spin.
fn pose(
    time: Res<Time>,
    karts: Query<(&Kart, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (kart, mut transform) in karts {
        let mut yaw = kart.yaw + kart.slide;
        yaw += kart.spin * 14.0;
        let hop = (kart.hop / 0.25 * std::f32::consts::PI).sin().max(0.0) * 0.6;
        let doze = if kart.coma > 0.0 {
            (t * 3.0).sin() * 0.12
        } else {
            0.0
        };
        let rumble = if kart.offroad && kart.speed.abs() > 4.0 {
            (t * 40.0).sin() * 0.04
        } else {
            0.0
        };
        transform.translation = flat(kart.pos, hop + rumble);
        transform.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_z(doze);
    }
}

fn sparks(
    karts: Query<(&Kart, &Sparks)>,
    mut lights: Query<(
        &mut Visibility,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for (kart, sparks) in karts {
        let Ok((mut visibility, mut transform, material)) = lights.get_mut(sparks.0) else {
            continue;
        };
        let Some(drift) = kart.drift else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let color = match drift.tier() {
            0 => Color::srgb(1.0, 0.95, 0.8),
            1 => Color::srgb(0.3, 0.6, 1.0),
            _ => Color::srgb(1.0, 0.45, 0.1),
        };
        *visibility = Visibility::Inherited;
        let flicker = 0.8 + (time.elapsed_secs() * 50.0).sin() * 0.3;
        transform.scale = Vec3::splat(flicker * (1.0 + f32::from(drift.tier()) * 0.4));
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.base_color = color;
            material.emissive = color.to_linear() * 12.0;
        }
    }
}

/// A tear flying off a crying racer.
#[derive(Component, Copy, Clone, Debug)]
struct Tear {
    vel: Vec3,
    life: f32,
}

/// Fast criers stream tears backwards.
fn tears(
    mut commands: Commands,
    time: Res<Time>,
    options: Res<Options>,
    karts: Query<&Kart>,
    drops: Query<(Entity, &mut Tear, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut look: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    mut carry: Local<f32>,
) {
    let dt = time.delta_secs();
    for (id, mut tear, mut transform) in drops {
        tear.life -= dt;
        tear.vel.y -= 12.0 * dt;
        transform.translation += tear.vel * dt;
        if tear.life <= 0.0 || transform.translation.y < 0.0 {
            commands.entity(id).despawn();
        }
    }

    let (mesh, material) = look
        .get_or_insert_with(|| {
            (
                meshes.add(Sphere::new(0.07)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.6, 0.85, 1.0),
                    emissive: LinearRgba::rgb(0.2, 0.4, 0.6),
                    perceptual_roughness: 0.1,
                    ..default()
                }),
            )
        })
        .clone();
    let rate = if options.high_quality { 40.0 } else { 15.0 };
    *carry += dt * rate;
    let count = *carry as u32;
    *carry -= count as f32;
    for kart in karts {
        if ROSTER[kart.racer].body != Body::Cat || kart.speed < 20.0 {
            continue;
        }
        let fwd = flat(kart.forward(), 0.0);
        let side = Vec3::new(fwd.z, 0.0, -fwd.x);
        for i in 0..count {
            let s = match i % 2 {
                0 => 1.0,
                _ => -1.0,
            };
            let head = flat(kart.pos, 1.85) + fwd * 0.6 + side * s * 0.25;
            commands.spawn((
                Tear {
                    vel: -fwd * kart.speed * 0.25 + side * s * 2.0 + Vec3::Y * 2.0,
                    life: 0.6,
                },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(head),
                DespawnOnExit(GameState::Race),
            ));
        }
    }
}

/// Steering rate in rad/s for this tick: input ramp, drift start, drift turning and charge.
/// Pure so the drift feel can be tested without the ECS.
pub fn steer_rate(
    kart: &mut Kart,
    input: &KartInput,
    stats: Stats,
    dt: f32,
) -> f32 {
    let top = Kart::TOP_SPEED * stats.top_speed;
    // Start a drift with a little hop: fast enough and steering held
    if kart.drift.is_none()
        && input.drift
        && input.steer.abs() > 0.3
        && kart.speed > top * Kart::DRIFT_MIN_SPEED
        && kart.hop <= 0.0
    {
        kart.drift = Some(Drift {
            dir: input.steer.signum(),
            charge: 0.0,
        });
        kart.hop = 0.25;
    }

    // Ease the stick so taps do not jerk the kart
    kart.steer = approach(kart.steer, input.steer, dt / Kart::STEER_RAMP);
    let steer = kart.steer;
    let fast = (kart.speed.abs() / top).min(1.0);
    let grip = (kart.speed.abs() / 8.0).min(1.0)
        * kart.speed.signum()
        * (1.0 - Kart::HIGH_SPEED_CUT * fast);
    let turn = Kart::TURN * stats.turn;
    match kart.drift.as_mut() {
        Some(drift) => {
            let lean = steer * drift.dir;
            drift.charge += dt * (0.7 + 0.6 * lean.max(0.0));
            // Leaning in tightens up to DRIFT_TURN + DRIFT_LEAN, counter-steer widens to
            // DRIFT_COUNTER; always the drift's own direction
            let share = if lean >= 0.0 {
                Kart::DRIFT_TURN + Kart::DRIFT_LEAN * lean
            } else {
                Kart::DRIFT_TURN + (Kart::DRIFT_TURN - Kart::DRIFT_COUNTER) * lean
            };
            drift.dir * turn * share * grip.abs()
        }
        None => steer * turn * grip,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Turning radius after holding the inputs for a few seconds at a steady speed.
    fn radius(
        speed: f32,
        input: &KartInput,
    ) -> f32 {
        let stats = ROSTER[0].stats;
        let mut kart = Kart::new(0, Vec2::ZERO, 0.0);
        kart.speed = speed;
        let dt = 1.0 / 60.0;
        let mut rate = 0.0;
        for _ in 0..180 {
            rate = steer_rate(&mut kart, input, stats, dt);
            kart.hop = (kart.hop - dt).max(0.0);
        }
        speed / rate.abs().max(1e-6)
    }

    fn held(
        steer: f32,
        drift: bool,
    ) -> KartInput {
        KartInput {
            throttle: 1.0,
            steer,
            drift,
            item: false,
        }
    }

    #[test]
    fn test_drift_arcs_wider_than_steering() {
        let speed = Kart::TOP_SPEED * 0.9;
        let steer = radius(speed, &held(1.0, false));
        let drift = radius(speed, &held(1.0, true));
        assert!(drift > steer, "drift radius {drift} vs steer {steer}");
    }

    #[test]
    fn test_drift_no_donuts() {
        // Even full lock into the drift keeps a wide arc at racing speed
        let drift = radius(Kart::TOP_SPEED * 0.6, &held(1.0, true));
        assert!(drift > 14.0, "drift radius {drift}");
    }

    #[test]
    fn test_drift_needs_speed() {
        let stats = ROSTER[0].stats;
        let mut kart = Kart::new(0, Vec2::ZERO, 0.0);
        kart.speed = Kart::TOP_SPEED * 0.3;
        steer_rate(&mut kart, &held(1.0, true), stats, 1.0 / 60.0);
        assert!(kart.drift.is_none());
    }

    #[test]
    fn test_drift_counter_steer_widens_not_flips() {
        let stats = ROSTER[0].stats;
        let mut kart = Kart::new(0, Vec2::ZERO, 0.0);
        kart.speed = Kart::TOP_SPEED * 0.8;
        steer_rate(&mut kart, &held(1.0, true), stats, 1.0 / 60.0);
        let mut rate = 0.0;
        for _ in 0..60 {
            rate = steer_rate(&mut kart, &held(-1.0, true), stats, 1.0 / 60.0);
        }
        assert!(rate > 0.0, "counter-steer flipped the drift: {rate}");
        let into = radius(kart.speed, &held(1.0, true));
        assert!(kart.speed / rate > into);
    }

    #[test]
    fn test_drift_makes_typical_corners() {
        // A drift held at racing speed must be able to follow a typical corner of every
        // track: tighter than the median curve of the bends
        let drift = radius(Kart::TOP_SPEED * 0.8, &held(1.0, true));
        for (index, def) in TRACKS.iter().enumerate() {
            let track = Track::new(index);
            let n = track.points.len();
            let mut bends: Vec<f32> = (0..n)
                .filter_map(|i| {
                    let a = track.points[(i + n - 4) % n];
                    let b = track.points[i];
                    let c = track.points[(i + 4) % n];
                    let turn = (b - a).angle_to(c - b).abs();
                    let r = a.distance(c) / turn.max(1e-4);
                    (r < 150.0).then_some(r)
                })
                .collect();
            bends.sort_by(f32::total_cmp);
            let typical = bends[bends.len() / 2];
            // Up to half the road width of drift either side still stays on the road
            assert!(
                drift < typical + def.half_width,
                "{}: drift radius {drift}, typical bend {typical}",
                def.name
            );
        }
    }
}
