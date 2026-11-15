//! Frame-time benchmark: `SLOP_BENCH=<file> gobble shots ...` runs the screenshot tour without
//! screenshots or fast-forward and writes frame-time stats per game state to `<file>` at exit.
//! `SLOP_BENCH_SECS=<secs>` ends the run early.
//! `SLOP_BENCH_HOLD=<secs>` stays on the title screen (attract race) that much longer.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Instant;

use bevy::prelude::*;

use crate::prelude::*;

/// Where to write the stats, if benchmarking.
pub(super) fn path() -> Option<String> {
    std::env::var("SLOP_BENCH").ok()
}

/// Extra seconds on the title screen before the tour moves on.
pub(super) fn hold() -> f32 {
    std::env::var("SLOP_BENCH_HOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

pub(super) fn plugin(app: &mut App) {
    if path().is_some() {
        app.init_resource::<Frames>()
            .add_systems(Last, (stop, record, write).chain());
    }
}

/// Ends the run after `SLOP_BENCH_SECS` of wall time, if set.
fn stop(
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    let secs = std::env::var("SLOP_BENCH_SECS")
        .ok()
        .and_then(|s| s.parse().ok());
    if secs.is_some_and(|s: f32| time.elapsed_secs() > s) {
        exit.write(AppExit::Success);
    }
}

#[derive(Resource, Default)]
struct Frames {
    last: Option<Instant>,
    state: Option<GameState>,
    since: f32,
    times: BTreeMap<String, Vec<f32>>,
}

/// Frame-to-frame wall time, skipping the first second after each state change (loading).
fn record(
    mut frames: ResMut<Frames>,
    state: Res<State<GameState>>,
) {
    let now = Instant::now();
    let dt = frames.last.map(|l| (now - l).as_secs_f32());
    frames.last = Some(now);
    if frames.state != Some(*state.get()) {
        frames.state = Some(*state.get());
        frames.since = 0.0;
        return;
    }
    let Some(dt) = dt else { return };
    frames.since += dt;
    if frames.since > 1.0 {
        let key = format!("{:?}", state.get());
        frames.times.entry(key).or_default().push(dt * 1000.0);
    }
}

fn write(
    frames: Res<Frames>,
    mut exit: MessageReader<AppExit>,
) {
    if exit.read().next().is_none() {
        return;
    }
    let mut out = String::from("state frames fps mean_ms p50_ms p95_ms p99_ms max_ms\n");
    for (key, times) in &frames.times {
        let mut sorted = times.clone();
        sorted.sort_by(f32::total_cmp);
        let at = |q: f32| sorted[((sorted.len() - 1) as f32 * q) as usize];
        let mean = sorted.iter().sum::<f32>() / sorted.len() as f32;
        let _ = writeln!(
            out,
            "{key} {} {:.1} {mean:.2} {:.2} {:.2} {:.2} {:.2}",
            sorted.len(),
            1000.0 / mean,
            at(0.5),
            at(0.95),
            at(0.99),
            sorted[sorted.len() - 1],
        );
    }
    if let Some(path) = path() {
        let _ = std::fs::write(path, out);
    }
}
