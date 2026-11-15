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
            .run_if(on_track),
    );
    app.add_systems(
        Update,
        (drive, bump, pose, sparks).chain().in_set(KartSet::Physics),
    );
    app.add_systems(Update, tears.after(KartSet::Physics).run_if(on_track));
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
    /// Ground height under the kart, kept by the physics step.
    pub ground: f32,
    /// Height of the kart's wheels: the ground's, or above it in the air.
    pub y: f32,
    /// Vertical speed, m/s up.
    pub rise: f32,
    /// Off the ground after a ramp or crest.
    pub air: bool,
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
    /// Yaw rate in rad/s, positive turning left.
    pub yaw_rate: f32,
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
    /// Pull of a slope along the road, m/s² per unit grade: arcade-light, not 9.8.
    pub const GRAVITY: f32 = 7.0;
    /// Fall in the air: heavy, so jumps stay short and snappy.
    pub const AIR_GRAVITY: f32 = 20.0;
    /// Knock-out after dropping into a gap and being set back on the road.
    const FELL: f32 = 0.6;
    pub const ACCEL: f32 = 20.0;
    /// Seconds for the steering input to ramp from centre to full lock.
    pub const STEER_RAMP: f32 = 0.15;
    /// Scales the drift's visual slide. 1.0 was too shallow and 1.15 too much, so it
    /// sits between; it no longer touches the turn rate.
    pub const DRIFT_ANGLE: f32 = 1.05;
    /// Body slide in radians at `DRIFT_ANGLE` 1.
    pub const DRIFT_SLIDE: f32 = 0.4;
    /// A drift starts only above this share of top speed.
    pub const DRIFT_MIN_SPEED: f32 = 0.4;
    /// How long the body takes to straighten after a drift.
    pub const DRIFT_SETTLE: f32 = 0.2;
    /// Fixed physics step, the same on every machine (needed for LAN play).
    pub const STEP: f32 = 1.0 / 120.0;
    /// Bicycle model: distance from the centre of mass to the front and rear axle.
    pub const FRONT_AXLE: f32 = 0.8;
    pub const REAR_AXLE: f32 = 0.8;
    /// Yaw inertia per unit mass.
    pub const INERTIA: f32 = 0.55;
    /// Front wheel lock at a standstill (rad), shrinking with speed past `STEER_FALLOFF`.
    pub const MAX_STEER: f32 = 0.45;
    pub const STEER_FALLOFF: f32 = 14.0;
    /// Tyre cornering stiffness per axle (lateral accel per rad of slip) and its saturation.
    pub const CORNERING: f32 = 90.0;
    pub const AXLE_GRIP: f32 = 17.0;
    /// Below `BLEND_LOW` the kart moves as a pure kinematic bicycle; above `BLEND_HIGH` as a
    /// dynamic one with tyre slip; blended between.
    pub const BLEND_LOW: f32 = 2.0;
    pub const BLEND_HIGH: f32 = 8.0;
    /// Assist: yaw rate is pulled toward what the tyres can hold, per second.
    pub const YAW_ASSIST: f32 = 3.0;
    pub const DRIFT_ASSIST: f32 = 1.2;
    /// Assist: largest body slip angle (rad) gripping and drifting; beyond it the kart
    /// counter-steers itself.
    pub const MAX_SLIP: f32 = 0.2;
    pub const DRIFT_MAX_SLIP: f32 = 0.5;
    /// A drift is a deliberate loss of rear grip.
    pub const DRIFT_REAR_GRIP: f32 = 0.5;
    /// Front wheels aim into the drift by this share of lock with the stick neutral; full
    /// lean adds the rest, counter-steer takes away down to `DRIFT_MIN_AIM`.
    pub const DRIFT_AIM: f32 = 0.55;
    pub const DRIFT_MIN_AIM: f32 = 0.15;
    /// Wall contact: share of along-wall speed lost on impact (scaled by how steep it is),
    /// how steep a hit must be to bounce, and how much of the into-wall speed bounces back.
    pub const WALL_FRICTION: f32 = 0.35;
    /// Speed lost per second while scraping along a wall.
    pub const WALL_DRAG: f32 = 0.6;
    /// How fast (rad/s) a wall turns the nose along itself.
    pub const WALL_ALIGN: f32 = 3.0;
    pub const WALL_STEEP: f32 = 0.7;
    pub const WALL_BOUNCE: f32 = 0.2;
    /// A drift arcs: its turning radius never gets tighter than this (at full lean; the
    /// radius grows as the lean eases off).
    pub const DRIFT_MIN_RADIUS: f32 = 15.0;

    /// A kart standing on the ground of `track`.
    pub fn on(
        racer: usize,
        pos: Vec2,
        yaw: f32,
        track: &Track,
    ) -> Self {
        let mut kart = Self::new(racer, pos, yaw);
        kart.ground = track.ground(pos);
        kart.y = kart.ground;
        kart
    }

    pub const fn new(
        racer: usize,
        pos: Vec2,
        yaw: f32,
    ) -> Self {
        Self {
            racer,
            pos,
            ground: 0.0,
            y: 0.0,
            rise: 0.0,
            air: false,
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
            yaw_rate: 0.0,
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
    mut karts: Query<(&mut Kart, &KartInput)>,
    mut carry: Local<f32>,
) {
    let dt = time.delta_secs().min(0.1);
    for (mut kart, _) in &mut karts {
        kart.fired = 0;
        kart.struck = false;
        kart.boost = (kart.boost - dt).max(0.0);
        kart.coma = (kart.coma - dt).max(0.0);
        kart.pied = (kart.pied - dt).max(0.0);
        kart.hop = (kart.hop - dt).max(0.0);
    }
    // Fixed steps so every machine simulates the same race
    *carry += dt;
    while *carry >= Kart::STEP {
        *carry -= Kart::STEP;
        for (mut kart, input) in &mut karts {
            if kart.frozen {
                continue;
            }
            let spot = track.find(kart.pos, kart.y);
            kart.offroad =
                spot.lateral.abs() > track.half_width + 1.2 && !track.in_shortcut(kart.pos);
            let stats = kart.stats();
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
            climb(&mut kart, &track, spot);
            let before = kart.pos;
            step(&mut kart, input, stats, top, Kart::STEP);

            // Walls just past the shoulders, swept so nothing tunnels through
            let hit = wall(&mut kart, &track, before);
            if hit && !kart.on_wall {
                // Count separate wall contacts, not every step against it
                kart.walls += 1;
            }
            kart.on_wall = hit;
        }
    }
}

/// Gravity: hills slow the climb and speed the run down, banked corners hold the kart
/// in, and where the ground drops away faster than the kart falls it flies until it
/// lands. A kart that drops into a gap is set back on the road past it. Also records the
/// ground height under the kart.
pub fn climb(
    kart: &mut Kart,
    track: &Track,
    spot: Spot,
) {
    let dt = Kart::STEP;
    let ground = track.ground_at(spot);
    kart.ground = ground;
    if kart.air {
        kart.rise -= Kart::AIR_GRAVITY * dt;
        kart.y += kart.rise * dt;
        if kart.y > ground {
            return;
        }
        kart.air = false;
        kart.y = ground;
        kart.rise = 0.0;
        if let Some(end) = track.gap_at(spot) {
            rescue(kart, track, end);
            return;
        }
    } else {
        let rise = ((ground - kart.y) / dt).clamp(-30.0, 30.0);
        let falling = kart.rise - Kart::AIR_GRAVITY * dt;
        if rise < falling - 1.2 && kart.speed > 4.0 {
            kart.air = true;
            kart.rise = falling;
            kart.y += falling * dt;
            return;
        }
        kart.y = ground;
        kart.rise = rise;
    }

    let grade = track.grade(spot.along);
    // Off the road the bank no longer holds the kart
    let bank = if spot.lateral.abs() <= track.barrier() {
        track.tilt(spot.along, spot.lateral) * 2.0
    } else {
        0.0
    };
    if grade != 0.0 || bank != 0.0 {
        let (_, dir) = track.sample(spot.along);
        // Down the slope, so a banked corner pulls towards its inside
        kart.vel -= (dir * grade + dir.perp() * bank) * (Kart::GRAVITY * Kart::STEP);
    }
}

/// Put a kart that fell into a gap back on the road just past it, slowed and dazed.
fn rescue(
    kart: &mut Kart,
    track: &Track,
    end: f32,
) {
    let along = end + 4.0;
    let (pos, dir) = track.sample(along);
    kart.pos = pos;
    kart.yaw = (-dir.x).atan2(-dir.y);
    kart.vel = dir * Kart::TOP_SPEED * 0.3;
    kart.speed = Kart::TOP_SPEED * 0.3;
    kart.yaw_rate = 0.0;
    kart.drift = None;
    kart.spin = Kart::FELL;
    kart.ground = track.height(along);
    kart.y = kart.ground;
}

/// Swept wall contact for one step from `before` to the kart's new position. On a crossing
/// the kart is put back on the wall line, the into-wall velocity is removed (a small bounce on
/// steep hits) and the along-wall velocity kept with mild friction, so it slides. Heading and
/// yaw stay the driver's. Returns whether the kart touched the wall.
pub fn wall(
    kart: &mut Kart,
    track: &Track,
    before: Vec2,
) -> bool {
    let limit = track.barrier();
    let spot = track.find(kart.pos, kart.y);
    if spot.lateral.abs() <= limit || track.in_shortcut(kart.pos) {
        return false;
    }
    let side = spot.lateral.signum();
    // Where along the step the wall line was crossed (the lateral offset is near linear
    // over one short step)
    let start = track.find(before, kart.y);
    let crossed = if start.lateral.abs() < limit && start.lateral.signum() == side {
        ((limit - start.lateral.abs()) / (spot.lateral.abs() - start.lateral.abs())).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let contact = before.lerp(kart.pos, crossed);
    let at = track.find(contact, kart.y).along;
    let (_, dir) = track.sample(at);
    // Outward normal: from the road toward this wall
    let normal = dir.perp() * side;
    let into = kart.vel.dot(normal).max(0.0);
    let along = kart.vel - normal * kart.vel.dot(normal);
    let steep = into / kart.vel.length().max(0.001);
    let bounce = if steep > Kart::WALL_STEEP {
        -into * Kart::WALL_BOUNCE
    } else {
        0.0
    };
    // A one-off loss on impact, scaled by how steep it was; then only light drag while scraping
    let impact = if kart.on_wall {
        0.0
    } else {
        Kart::WALL_FRICTION * steep
    };
    let drag = (-Kart::WALL_DRAG * Kart::STEP).exp();
    kart.vel = along * (1.0 - impact) * drag + normal * bounce;
    // Back onto the wall line, a hair inside, then slide for the rest of the step
    kart.pos = track.offset(at, (limit - 0.01) * side) + kart.vel * Kart::STEP * (1.0 - crossed);
    let settled = track.find(kart.pos, kart.y);
    if settled.lateral.abs() > limit {
        kart.pos = track.offset(settled.along, (limit - 0.01) * side);
    }
    // The wall turns the nose along it, gently, so the kart slides instead of grinding
    if along.length() > 1.0 {
        let tangent = along.normalize();
        let want = (-tangent.x).atan2(-tangent.y);
        let diff = (want - kart.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        let turn = Kart::WALL_ALIGN * Kart::STEP;
        kart.yaw += diff.clamp(-turn, turn);
    }
    kart.speed = kart.vel.dot(kart.forward());
    true
}

fn smoothstep(
    low: f32,
    high: f32,
    x: f32,
) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// One fixed physics step for one kart: throttle, drift rules, then a bicycle model that is
/// kinematic at low speed and has linear, saturating tyres at speed. Pure, for tests.
pub fn step(
    kart: &mut Kart,
    input: &KartInput,
    stats: Stats,
    top: f32,
    dt: f32,
) {
    let spinning = kart.spin > 0.0;
    let fwd = kart.forward();
    let left = Vec2::new(fwd.y, -fwd.x);
    let mut vx = kart.vel.dot(fwd);
    let mut vy = kart.vel.dot(left);
    let mut r = kart.yaw_rate;
    let quiet = KartInput::default();
    let input = if spinning { &quiet } else { input };

    if spinning {
        // Knocked out: no control, bleed speed
        kart.spin = (kart.spin - dt).max(0.0);
        vx *= (-2.5 * dt).exp();
        kart.drift = None;
    } else {
        // Throttle and brakes act on forward speed only
        let accel = Kart::ACCEL * stats.accel;
        if kart.boost > 0.0 {
            vx = approach(vx, top, accel * 3.0 * dt);
        } else if input.throttle > 0.1 {
            let rate = if vx > top {
                25.0
            } else {
                accel * input.throttle * (1.2 - vx / top).max(0.25)
            };
            vx = approach(vx, top, rate * dt);
        } else if input.throttle < -0.1 {
            vx = approach(vx, -top * 0.35, 40.0 * dt);
        } else {
            vx = approach(vx, 0.0, 8.0 * dt);
        }
    }
    kart.speed = vx;

    let fire = drift_control(kart, input, stats, dt);
    if fire > 0 {
        kart.fired = kart.fired.max(fire);
    }

    // Front wheel angle: the eased stick, or the drift's aim
    let lock = Kart::MAX_STEER / (1.0 + vx.abs() / Kart::STEER_FALLOFF);
    let mut share = 1.0;
    let aim = match kart.drift {
        Some(drift) => {
            let lean = kart.steer * drift.dir;
            share = (Kart::DRIFT_AIM + (1.0 - Kart::DRIFT_AIM) * lean).max(Kart::DRIFT_MIN_AIM);
            drift.dir * share
        }
        None => kart.steer,
    };
    let delta = aim * lock * stats.turn;
    let wheelbase = Kart::FRONT_AXLE + Kart::REAR_AXLE;

    // Kinematic bicycle: the wheels roll where they point
    let r_kinematic = vx * delta.tan() / wheelbase;

    // Dynamic bicycle: slip angles, linear tyres that saturate
    let rear_grip = match kart.drift {
        Some(_) => Kart::DRIFT_REAR_GRIP,
        None => 1.0,
    };
    let sx = vx.abs().max(0.5);
    let alpha_front = (vy + Kart::FRONT_AXLE * r).atan2(sx) - delta * vx.signum();
    let alpha_rear = (vy - Kart::REAR_AXLE * r).atan2(sx);
    let grip = Kart::AXLE_GRIP * stats.weight.sqrt();
    let f_front = (-Kart::CORNERING * alpha_front).clamp(-grip, grip);
    let f_rear =
        (-Kart::CORNERING * rear_grip * alpha_rear).clamp(-grip * rear_grip, grip * rear_grip);
    let vy_dynamic = vy + (f_front * delta.cos() + f_rear - vx * r) * dt;
    let r_dynamic = r
        + (Kart::FRONT_AXLE * f_front * delta.cos() - Kart::REAR_AXLE * f_rear) / Kart::INERTIA
            * dt;

    let w = smoothstep(Kart::BLEND_LOW, Kart::BLEND_HIGH, vx.abs());
    r = r_kinematic + (r_dynamic - r_kinematic) * w;
    vy *= 0.0;
    vy += vy_dynamic * w;

    // Assists: settle the yaw toward what the tyres can hold, cap the slip angle
    let hold = (grip * (1.0 + rear_grip)) / sx;
    let settle = r_kinematic.clamp(-hold, hold);
    let assist = match kart.drift {
        Some(_) => Kart::DRIFT_ASSIST,
        None => Kart::YAW_ASSIST,
    };
    if !spinning {
        r += (settle - r) * (1.0 - (-assist * dt).exp());
    }
    let max_slip = match kart.drift {
        Some(_) => Kart::DRIFT_MAX_SLIP,
        None => Kart::MAX_SLIP,
    };
    let slip_limit = sx * max_slip.tan();
    vy = vy.clamp(-slip_limit, slip_limit);
    if let Some(drift) = kart.drift {
        // A drift arcs, never spins: bounded rate, always its own direction, and wider
        // the less the stick leans in
        let cap = vx.abs() * share / Kart::DRIFT_MIN_RADIUS;
        r = (r * drift.dir).clamp(0.0, cap) * drift.dir;
    }

    kart.yaw_rate = r;
    kart.yaw += r * dt;
    let fwd = kart.forward();
    let left = Vec2::new(fwd.y, -fwd.x);
    kart.vel = fwd * vx + left * vy;
    kart.pos += kart.vel * dt;

    // Ease the body into and out of the drift slide
    let target = kart
        .drift
        .map_or(0.0, |d| d.dir * Kart::DRIFT_SLIDE * Kart::DRIFT_ANGLE);
    kart.slide = approach(
        kart.slide,
        target,
        Kart::DRIFT_SLIDE * Kart::DRIFT_ANGLE * dt / Kart::DRIFT_SETTLE,
    );
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
    track: Res<Track>,
    karts: Query<(&Kart, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    // Suspension: the body follows the ground a beat late
    let ride = 1.0 - (-time.delta_secs() * 18.0).exp();
    let hilly = track.hilly();
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
        // Sit on the ground's slope, felt over the kart's wheelbase
        let tilt = if hilly {
            let fwd = kart.forward();
            let left = fwd.perp();
            let ground = |p: Vec2| track.ground_near(p, kart.y);
            let dx = ground(kart.pos + fwd) - ground(kart.pos - fwd);
            let dz = ground(kart.pos + left) - ground(kart.pos - left);
            let normal = (Vec3::Y * 2.0 - flat(fwd, 0.0) * dx - flat(left, 0.0) * dz).normalize();
            Quat::from_rotation_arc(Vec3::Y, normal)
        } else {
            Quat::IDENTITY
        };
        let y = transform.translation.y - hop - rumble;
        // Sprung on the ground, exact in the air
        let y = if kart.air {
            kart.y
        } else {
            y + (kart.y - y) * ride
        };
        transform.translation = flat(kart.pos, y + hop + rumble);
        transform.rotation = tilt * Quat::from_rotation_y(yaw) * Quat::from_rotation_z(doze);
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
        if tear.life <= 0.0 {
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
            let head = flat(kart.pos, kart.y + 1.85) + fwd * 0.6 + side * s * 0.25;
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

/// Stick easing, drift start and release, and mini-turbo charge for one step. Returns the
/// mini-turbo tier fired by a release this step.
fn drift_control(
    kart: &mut Kart,
    input: &KartInput,
    stats: Stats,
    dt: f32,
) -> u8 {
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
    if let Some(drift) = kart.drift.as_mut() {
        let lean = kart.steer * drift.dir;
        drift.charge += dt * (0.7 + 0.6 * lean.max(0.0));
    }
    // Release a drift into a mini-turbo
    let release = !input.drift || kart.speed < top * 0.25;
    if release && let Some(drift) = kart.drift.take() {
        let tier = drift.tier();
        kart.boost = kart.boost.max(match tier {
            2 => 1.1,
            1 => 0.55,
            _ => 0.0,
        });
        return tier;
    }
    0
}

#[cfg(test)]
mod test {
    use super::*;

    const DT: f32 = Kart::STEP;

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

    /// A kart rolling straight at `speed`.
    fn rolling(speed: f32) -> Kart {
        let mut kart = Kart::new(0, Vec2::ZERO, 0.0);
        kart.frozen = false;
        kart.speed = speed;
        kart.vel = kart.forward() * speed;
        kart
    }

    /// Run the inputs at a held speed and return the steady turning radius.
    fn radius(
        speed: f32,
        input: &KartInput,
    ) -> f32 {
        let stats = ROSTER[0].stats;
        let mut kart = rolling(speed);
        for _ in 0..360 {
            step(&mut kart, input, stats, speed, DT);
            kart.hop = (kart.hop - DT).max(0.0);
        }
        kart.vel.length() / kart.yaw_rate.abs().max(1e-6)
    }

    /// Angle between where the kart points and where it goes.
    fn slip(kart: &Kart) -> f32 {
        if kart.vel.length() < 0.1 {
            return 0.0;
        }
        kart.forward().angle_to(kart.vel).abs()
    }

    #[test]
    fn test_standstill_no_sideways() {
        let stats = ROSTER[0].stats;
        let mut kart = rolling(0.0);
        let quiet = KartInput {
            throttle: 0.0,
            steer: 1.0,
            drift: true,
            item: false,
        };
        for _ in 0..120 {
            step(&mut kart, &quiet, stats, Kart::TOP_SPEED, DT);
        }
        assert!(kart.pos.length() < 1e-4, "moved {:?}", kart.pos);
        assert!(kart.yaw.abs() < 1e-4, "turned {}", kart.yaw);
    }

    #[test]
    fn test_low_speed_bicycle_radius() {
        let speed = 1.5;
        let stats = ROSTER[0].stats;
        let measured = radius(speed, &held(1.0, false));
        let delta = Kart::MAX_STEER / (1.0 + speed / Kart::STEER_FALLOFF) * stats.turn;
        let formula = (Kart::FRONT_AXLE + Kart::REAR_AXLE) / delta.tan();
        assert!(
            (measured - formula).abs() / formula < 0.05,
            "radius {measured} vs bicycle {formula}"
        );
    }

    #[test]
    fn test_drift_arcs_not_spins() {
        let stats = ROSTER[0].stats;
        let speed = Kart::TOP_SPEED * 0.9;
        let mut kart = rolling(speed);
        for _ in 0..360 {
            step(&mut kart, &held(1.0, true), stats, speed, DT);
            assert!(slip(&kart) < 1.0, "spinning: slip {}", slip(&kart));
        }
        assert!(kart.drift.is_some());
        let r = kart.vel.length() / kart.yaw_rate.abs();
        assert!(r >= Kart::DRIFT_MIN_RADIUS * 0.95, "radius {r}");
    }

    #[test]
    fn test_drift_no_donuts() {
        let drift = radius(Kart::TOP_SPEED * 0.6, &held(1.0, true));
        assert!(drift > 14.0, "drift radius {drift}");
    }

    #[test]
    fn test_drift_needs_speed() {
        let stats = ROSTER[0].stats;
        let mut kart = rolling(Kart::TOP_SPEED * 0.3);
        step(&mut kart, &held(1.0, true), stats, Kart::TOP_SPEED, DT);
        assert!(kart.drift.is_none());
    }

    #[test]
    fn test_drift_counter_steer_widens_not_flips() {
        let stats = ROSTER[0].stats;
        let speed = Kart::TOP_SPEED * 0.8;
        let mut kart = rolling(speed);
        for _ in 0..120 {
            step(&mut kart, &held(1.0, true), stats, speed, DT);
        }
        let into = kart.vel.length() / kart.yaw_rate.abs();
        for _ in 0..240 {
            step(&mut kart, &held(-1.0, true), stats, speed, DT);
        }
        assert!(
            kart.yaw_rate > 0.0,
            "counter-steer flipped the drift: {}",
            kart.yaw_rate
        );
        let counter = kart.vel.length() / kart.yaw_rate;
        assert!(counter > into, "counter radius {counter} vs into {into}");
    }

    #[test]
    fn test_drift_release_recovers() {
        let stats = ROSTER[0].stats;
        let speed = Kart::TOP_SPEED * 0.8;
        let mut kart = rolling(speed);
        for _ in 0..240 {
            step(&mut kart, &held(1.0, true), stats, speed, DT);
        }
        let steps = (0.5 / DT) as usize;
        for _ in 0..steps {
            step(&mut kart, &held(0.0, false), stats, speed, DT);
        }
        assert!(slip(&kart) < 0.09, "slip {} after 0.5 s", slip(&kart));
        assert!(kart.slide.abs() < 1e-3, "body slide {}", kart.slide);
    }

    /// Drive at top boost speed into the left wall of the first straight of seed 1 at
    /// `angle` (rad from the wall line) and check the kart never passes the wall.
    fn into_wall(angle: f32) -> (Kart, f32) {
        let track = crate::track::generate::track(1);
        let stats = ROSTER[0].stats;
        let along = 20.0;
        let (_, dir) = track.sample(along);
        let left = dir.perp();
        let heading = dir * angle.cos() + left * angle.sin();
        let mut kart = Kart::new(
            0,
            track.offset(along, track.half_width),
            (-heading.x).atan2(-heading.y),
        );
        kart.frozen = false;
        let speed = Kart::TOP_SPEED * 1.4;
        kart.speed = speed;
        kart.vel = heading * speed;
        let tangential = kart.vel.dot(dir);
        for _ in 0..120 {
            let before = kart.pos;
            step(&mut kart, &held(0.0, false), stats, speed, DT);
            let hit = wall(&mut kart, &track, before);
            kart.on_wall = hit;
            let lateral = track.locate(kart.pos).lateral;
            assert!(
                lateral <= track.barrier() + 1e-3,
                "through the wall: {lateral}"
            );
        }
        (kart, tangential)
    }

    #[test]
    fn test_wall_shallow_slides() {
        let (kart, tangential) = into_wall(0.25);
        let track = crate::track::generate::track(1);
        let (_, dir) = track.sample(track.locate(kart.pos).along);
        assert!(
            kart.vel.dot(dir) > tangential * 0.6,
            "lost too much speed along the wall: {} of {tangential}",
            kart.vel.dot(dir)
        );
    }

    #[test]
    fn test_wall_steep_stops_not_tunnels() {
        let (kart, _) = into_wall(1.3);
        assert!(kart.vel.length().is_finite());
    }

    #[test]
    fn test_drift_makes_typical_corners() {
        // A drift held at racing speed must be able to follow a typical corner of every
        // track: tighter than the median curve of the bends
        let drift = radius(Kart::TOP_SPEED * 0.8, &held(1.0, true));
        for seed in 0..50 {
            let track = crate::track::generate::track(seed);
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
            assert!(
                drift < typical + track.half_width,
                "seed {seed} ({}): drift radius {drift}, typical bend {typical}",
                track.name
            );
        }
    }
}
