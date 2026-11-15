//! CPU drivers: follow the racing line, rubber band toward the players, use items.

use bevy::prelude::*;
use rand::Rng;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Cpu;
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, drive.in_set(KartSet::Input));
}

/// Brain of a CPU kart. Finished players get one too.
#[derive(Component, Clone, Debug)]
pub struct Cpu {
    /// Preferred offset from the centerline.
    pub lane: f32,
    /// Base pace before rubber banding.
    pub skill: f32,
    pub item_wait: f32,
}
impl Cpu {
    pub fn new(skill: f32) -> Self {
        let mut rng = rand::rng();
        Self {
            lane: rng.random_range(-3.0..3.0),
            skill: skill * Pilot::PACE,
            item_wait: 0.0,
        }
    }
}

/// Controller tuning for CPU drivers.
struct Pilot;
impl Pilot {
    /// Look-ahead distance: base plus seconds of travel.
    const LOOK: f32 = 6.0;
    const LOOK_TIME: f32 = 0.3;
    /// Extra steering over pure pursuit, to beat tyre understeer.
    const GAIN: f32 = 1.4;
    /// Lift off when the needed lateral grip exceeds this share of what the tyres hold.
    const LIFT: f32 = 0.8;
    const BRAKE: f32 = 1.3;
    /// Yaw-rate damping so the line does not oscillate.
    const DAMP: f32 = 0.15;
    /// The bicycle-model pilot holds its line better than the old one, so it runs a touch
    /// slower to keep CPU lap times (and difficulty) where they were.
    const PACE: f32 = 0.95;
}

/// Pure pursuit on the racing line through the kart's velocity heading, with a lift/brake
/// rule from the grip the bicycle model can hold. Returns (steer, throttle).
pub fn pursue(
    kart: &Kart,
    track: &Track,
    along: f32,
    lane: f32,
) -> (f32, f32) {
    let speed = kart.vel.length().max(kart.speed.abs());
    let look = Pilot::LOOK + speed * Pilot::LOOK_TIME;
    let target = track.offset(along + look, lane);
    let to = target - kart.pos;
    let heading = if kart.vel.length() > 3.0 {
        kart.vel.normalize()
    } else {
        kart.forward()
    };
    // Signed angle to the target, positive to the left
    let alpha = -heading.angle_to(to.normalize_or_zero());
    let curvature = 2.0 * alpha.sin() / to.length().max(1.0);
    let wheelbase = Kart::FRONT_AXLE + Kart::REAR_AXLE;
    let lock = Kart::MAX_STEER / (1.0 + speed / Kart::STEER_FALLOFF) * kart.stats().turn;
    let delta = (curvature * wheelbase).atan();
    let steer = (delta / lock * Pilot::GAIN - kart.yaw_rate * Pilot::DAMP).clamp(-1.0, 1.0);

    // Grip check on the bend ahead, judged from further down the line
    let far = track.offset(along + look * 2.0, lane);
    let bend = heading.angle_to((far - kart.pos).normalize_or_zero()).abs();
    let need = speed * speed * 2.0 * bend.sin() / (far - kart.pos).length().max(1.0);
    let hold = Kart::AXLE_GRIP * 2.0 * kart.stats().weight.sqrt();
    let throttle = if need > hold * Pilot::BRAKE {
        -0.4
    } else if need > hold * Pilot::LIFT {
        0.0
    } else {
        1.0
    };
    (steer, throttle)
}

fn drive(
    time: Res<Time>,
    track: Res<Track>,
    karts: Query<(
        &mut Kart,
        &mut KartInput,
        &mut Cpu,
        &Progress,
        Has<Finished>,
    )>,
    players: Query<&Progress, With<Human>>,
) {
    let dt = time.delta_secs();
    let lead = players.iter().map(|p| p.total).fold(f32::MIN, f32::max);
    let mut rng = rand::rng();
    for (mut kart, mut input, mut cpu, progress, finished) in karts {
        let (steer, throttle) = pursue(&kart, &track, progress.along, cpu.lane);
        input.steer = steer;
        input.throttle = throttle;
        input.drift = false;

        // Rubber band: catch up when behind the best human, ease off when ahead
        kart.pace = match (lead > f32::MIN, finished) {
            (true, false) => {
                let gap = lead - progress.total;
                cpu.skill * (1.0 + gap * 0.0015).clamp(0.88, 1.14)
            }
            _ => cpu.skill,
        };

        // Wander the lane a little so the pack spreads out
        cpu.lane = (cpu.lane + rng.random_range(-1.0..1.0) * dt * 2.0).clamp(-4.0, 4.0);

        input.item = false;
        if kart.item.is_some() {
            cpu.item_wait += dt;
            if cpu.item_wait > 1.5 && rng.random_bool(f64::from((dt * 0.8).min(1.0))) {
                input.item = true;
                cpu.item_wait = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Drive one CPU kart alone for up to `laps` laps or `limit` s with the real physics
    /// step and walls; lap times and wall contacts.
    fn solo(
        track: &Track,
        laps: usize,
        limit: f32,
    ) -> (Vec<f32>, u32) {
        let start = track.length - 6.0;
        let pos = track.offset(start, 0.0);
        let dir = track.sample(start).1;
        let mut kart = Kart::new(0, pos, (-dir.x).atan2(-dir.y));
        kart.frozen = false;
        let stats = kart.stats();
        let top = Kart::TOP_SPEED * stats.top_speed;
        let mut along = start;
        let mut walls = 0;
        let mut on_wall = false;
        let mut t = 0.0;
        let mut lap_times = Vec::new();
        let mut last = 0.0;
        while lap_times.len() < laps && t < limit {
            let (steer, throttle) = pursue(&kart, track, along, 0.0);
            let input = KartInput {
                throttle,
                steer,
                drift: false,
                item: false,
            };
            let spot = track.find(kart.pos, kart.y);
            crate::kart::climb(&mut kart, track, spot);
            crate::kart::step(&mut kart, &input, stats, top, Kart::STEP);
            let spot = track.find(kart.pos, kart.y);
            if spot.lateral.abs() > track.barrier() {
                kart.pos = track.offset(spot.along, track.barrier() * spot.lateral.signum());
                kart.speed *= 0.6;
                kart.vel *= 0.4;
                kart.yaw_rate *= 0.5;
                if !on_wall {
                    walls += 1;
                }
                on_wall = true;
            } else {
                on_wall = false;
            }
            if spot.along < along - track.length / 2.0 {
                lap_times.push(t - last);
                last = t;
            }
            along = spot.along;
            t += Kart::STEP;
        }
        (lap_times, walls)
    }

    /// A CPU kart gets round every generated track at a sane pace: no corner it cannot
    /// make, no spot it gets stuck in.
    #[test]
    fn test_generated_tracks_lap() {
        for seed in 0..50 {
            let track = crate::track::generate::track(seed);
            // The first lap starts from a standstill; time the second
            let (laps, walls) = solo(&track, 2, 240.0);
            assert_eq!(
                laps.len(),
                2,
                "seed {seed} ({}): stuck after {laps:?}",
                track.name
            );
            // Mean speed over the lap: at least a third of top speed
            let pace = track.length / laps[1];
            assert!(
                pace > Kart::TOP_SPEED / 3.0,
                "seed {seed} ({}): {pace:.1} m/s over {:.0} m, {walls} wall hits",
                track.name,
                track.length
            );
        }
    }
}
