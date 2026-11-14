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
            skill,
            item_wait: 0.0,
        }
    }
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
        let look = 7.0 + kart.speed.max(0.0) * 0.35;
        let target = track.offset(progress.along + look, cpu.lane);
        let to = target - kart.pos;
        let want = (-to.x).atan2(-to.y);
        let diff = (want - kart.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        let sharp = diff.abs() > 0.6 && kart.speed > 20.0;

        input.steer = (diff * 2.5).clamp(-1.0, 1.0);
        input.throttle = if sharp { 0.3 } else { 1.0 };
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
