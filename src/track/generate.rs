//! Seeded random tracks: a loop around a centre, bent by a mix of corner kinds.
//!
//! Control points sit at increasing angles around the origin, so the loop starts out
//! free of crossings; hairpins, chicanes, esses, sweepers and straights then push them
//! in, out or onto a line. Every candidate is checked for curves the karts cannot make
//! and for legs of road that come too close, and the next attempt is tried until one
//! passes. The same seed always gives the same track.

use std::f32::consts::TAU;

use bevy::prelude::*;

use super::LOOKS;
use super::Scenery;
use super::Track;
use super::centerline;

/// Tightest centerline radius a CPU kart takes at a sane speed.
pub const MIN_RADIUS: f32 = 13.0;
/// Free space kept between two legs of road beyond their walls.
const GAP: f32 = 4.0;
const LENGTH: std::ops::Range<f32> = 650.0..1300.0;
/// Steepest grade of a hill.
pub const MAX_GRADE: f32 = 0.16;
/// Steepest cross-slope of a banked corner.
pub const MAX_BANK: f32 = 0.22;
/// Most two nearby legs of road may differ in height, so the ground between them stays
/// a slope rather than a cliff.
const STEP_DOWN: f32 = 2.0;

/// splitmix64: tiny, fast and the same on every machine.
struct Dice(u64);
impl Dice {
    const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(
        &mut self,
        range: std::ops::Range<f32>,
    ) -> f32 {
        range.start + self.unit() * (range.end - range.start)
    }

    const fn pick<T: Copy>(
        &mut self,
        from: &[T],
    ) -> T {
        from[(self.next() % from.len() as u64) as usize]
    }
}

#[derive(Copy, Clone, Debug)]
enum Corner {
    Hairpin,
    Chicane,
    Esses,
    Sweeper,
    Straight,
    Plain,
}
impl Corner {
    /// Control points the corner bends.
    const fn span(self) -> usize {
        match self {
            Self::Hairpin | Self::Sweeper => 3,
            Self::Chicane => 2,
            Self::Esses => 4,
            Self::Straight => 4,
            Self::Plain => 1,
        }
    }
}

/// The track for `seed`.
pub fn track(seed: u32) -> Track {
    (0..u64::MAX)
        .find_map(|attempt| attempt_track(seed, attempt))
        .expect("some attempt passes")
}

fn attempt_track(
    seed: u32,
    attempt: u64,
) -> Option<Track> {
    let mut dice = Dice(u64::from(seed) << 20 ^ attempt);
    let theme = dice.pick(&[0, 1, 2]);
    let half_width = dice.range(7.5..9.0);
    // The seed alone picks the kind, so a figure eight keeps trying as one
    let eight = Dice(u64::from(seed)).unit() < 0.3;
    let control = if eight {
        figure_eight(&mut dice)
    } else {
        layout(&mut dice)
    };
    let (points, dists, length) = centerline(&control);
    let barrier = half_width + 5.0;
    if !LENGTH.contains(&length) || !smooth(&points) || (!eight && !clear(&points, &[], barrier)) {
        return None;
    }
    let mut track = Track {
        name: name(&mut dice, LOOKS[theme].scenery),
        points,
        dists,
        length,
        half_width,
        heights: Vec::new(),
        banks: Vec::new(),
        ramps: Vec::new(),
        gaps: Vec::new(),
        decks: Vec::new(),
        pipes: Vec::new(),
        shortcuts: Vec::new(),
        boxes: Vec::new(),
        scenery: LOOKS[theme].scenery,
        theme: LOOKS[theme].theme,
        landmark: None,
    };
    track.heights = hills(&mut dice, &track.dists, length);
    if eight {
        bridge(&mut track)?;
        track.decks = decks(&track);
        if track.decks.is_empty() || !clear(&track.points, &track.heights, barrier) {
            return None;
        }
    }
    let zone = zone(&track);
    track.boxes = boxes(&track, &zone);
    track.landmark = landmark(&track, 22.0);
    track.banks = banks(&track.points, dice.range(0.6..1.0));
    jumps(&mut dice, &mut track, &zone);
    if dice.unit() < 0.5 {
        track.pipes = pipes(&track, &zone);
    }
    level(&track, &zone).then_some(track)
}

/// Control points of a figure eight, its lobes squashed and wobbled at random. Its two
/// legs cross in the middle; [`bridge`] lifts one over the other.
fn figure_eight(dice: &mut Dice) -> Vec<Vec2> {
    let n = dice.range(18.0..26.0) as usize;
    let a = dice.range(140.0..190.0);
    let points: Vec<Vec2> = (0..n)
        .map(|k| {
            let t = (k as f32 + dice.range(-0.1..0.1)) * TAU / n as f32;
            let (s, c) = t.sin_cos();
            let p = Vec2::new(a * c, a * s * c) / (1.0 + s * s);
            // Wobble each point about its lobe's centre, leaving the crossing alone
            let centre = Vec2::new(a * 0.6 * c.signum(), 0.0);
            let wobble = 1.0 + dice.range(-0.12..0.12) * (p.x.abs() / a).min(1.0);
            centre + (p - centre) * wobble
        })
        .collect();
    let stretch = Vec2::new(1.0, dice.range(0.9..1.5));
    let turn = Vec2::from_angle(dice.range(0.0..TAU));
    let points: Vec<Vec2> = points.iter().map(|p| turn.rotate(*p * stretch)).collect();
    start_on_straight(&points)
}

/// Lift the road over the crossing of a figure eight high enough to clear the other leg.
fn bridge(track: &mut Track) -> Option<()> {
    const HALF: f32 = 80.0;
    let n = track.points.len();
    let p = &track.points;
    // The crossing: the closest pair of samples a quarter lap or more apart
    let (i, j) = (0..n)
        .flat_map(|i| (i + n / 4..n.min(i + 3 * n / 4)).map(move |j| (i, j)))
        .min_by(|a, b| {
            p[a.0]
                .distance_squared(p[a.1])
                .total_cmp(&p[b.0].distance_squared(p[b.1]))
        })?;
    let top = Track::CLEAR + 1.5;
    let at = track.dists[i];
    for (k, s) in track.dists.iter().enumerate() {
        let d = (s - at).abs();
        let d = d.min(track.length - d);
        if d < HALF {
            track.heights[k] += top * 0.5 * (1.0 + (std::f32::consts::PI * d / HALF).cos());
        }
    }
    let h = &track.heights;
    let steepest = (0..n)
        .map(|k| (h[(k + 1) % n] - h[k]).abs() / Track::STEP)
        .fold(0.0, f32::max);
    (h[i] - h[j] >= Track::CLEAR + 0.5 && steepest <= MAX_GRADE * 1.3).then_some(())
}

/// Samples of road lifted over another leg close enough that their walls overlap.
fn decks(track: &Track) -> Vec<usize> {
    let n = track.points.len();
    let near = 2.0 * track.barrier() + GAP;
    let same = (near * 2.5 / Track::STEP) as usize;
    let (p, h) = (&track.points, &track.heights);
    (0..n)
        .filter(|&i| {
            (0..n).any(|j| {
                let apart = i.abs_diff(j);
                apart.min(n - apart) >= same
                    && h[i] - h[j] >= Track::CLEAR - 0.5
                    && p[i].distance_squared(p[j]) < near * near
            })
        })
        .collect()
}

/// Samples near a bridge, kept free of crates and jumps and allowed steep ground.
fn zone(track: &Track) -> Vec<bool> {
    let n = track.points.len();
    let reach = (90.0 / Track::STEP) as usize;
    let mut zone = vec![false; n];
    for &d in &track.decks {
        for k in 0..=2 * reach {
            zone[(d + n + k - reach) % n] = true;
        }
    }
    zone
}

/// Control points of one candidate loop.
fn layout(dice: &mut Dice) -> Vec<Vec2> {
    let n = dice.range(14.0..22.0) as usize;
    let base = dice.range(95.0..135.0);
    let mut radius = vec![base; n];
    let mut angle: Vec<f32> = (0..n)
        .map(|i| (i as f32 + dice.range(-0.25..0.25)) * TAU / n as f32)
        .collect();
    let mut straights = Vec::new();

    let mut i = 0;
    while i < n {
        let corner = dice.pick(&[
            Corner::Hairpin,
            Corner::Chicane,
            Corner::Esses,
            Corner::Esses,
            Corner::Sweeper,
            Corner::Straight,
            Corner::Straight,
            Corner::Plain,
        ]);
        if i + corner.span() > n {
            break;
        }
        match corner {
            Corner::Hairpin => {
                // Dive deep towards the centre and back out on close angles
                radius[i + 1] *= dice.range(0.35..0.5);
                let mid = angle[i + 1];
                angle[i] = mid - (mid - angle[i]) * 0.6;
                angle[i + 2] = mid + (angle[i + 2] - mid) * 0.6;
            }
            Corner::Chicane => {
                let kick = dice.range(10.0..16.0) * dice.pick(&[-1.0, 1.0]);
                radius[i] += kick;
                radius[i + 1] -= kick;
            }
            Corner::Esses => {
                let amp = dice.range(0.12..0.2) * dice.pick(&[-1.0, 1.0]);
                for k in 0..4 {
                    radius[i + k] *= 1.0 + amp * (k as f32 * 2.1).sin();
                }
            }
            Corner::Sweeper => {
                let bulge = dice.range(1.1..1.25);
                radius[i + 1] *= bulge;
                radius[i] *= bulge.sqrt();
                radius[i + 2] *= bulge.sqrt();
            }
            Corner::Straight => straights.push(i),
            Corner::Plain => {}
        }
        i += corner.span();
    }

    let stretch = Vec2::new(dice.range(0.75..1.3), 1.0);
    let turn = Vec2::from_angle(dice.range(0.0..TAU));
    let mut points: Vec<Vec2> = (0..n)
        .map(|i| turn.rotate(Vec2::from_angle(angle[i]) * radius[i] * stretch))
        .collect();

    // Line the middle points of each straight up on its ends
    for i in straights {
        let (a, b) = (points[i], points[i + 3]);
        points[i + 1] = a.lerp(b, 1.0 / 3.0);
        points[i + 2] = a.lerp(b, 2.0 / 3.0);
    }
    start_on_straight(&points)
}

/// The same loop starting halfway along its longest leg, so the grid has a straight.
fn start_on_straight(points: &[Vec2]) -> Vec<Vec2> {
    let n = points.len();
    let start = (0..n)
        .max_by(|a, b| {
            let len = |i: &usize| points[*i].distance(points[(*i + 1) % n]);
            len(a).total_cmp(&len(b))
        })
        .unwrap_or(0);
    let mid = points[start].lerp(points[(start + 1) % n], 0.5);
    let mut ordered = vec![mid];
    ordered.extend((1..=n).map(|k| points[(start + k) % n]));
    ordered
}

/// Road heights: a few hills and dips, often a sharp crest, flat around the start line.
fn hills(
    dice: &mut Dice,
    dists: &[f32],
    length: f32,
) -> Vec<f32> {
    // (centre, half width, height)
    let mut bumps: Vec<(f32, f32, f32)> = (0..dice.range(2.0..5.0) as usize)
        .map(|_| {
            let height = dice.range(2.0..9.0) * dice.pick(&[-1.0, 1.0, 1.0]);
            (dice.range(0.0..length), dice.range(50.0..140.0), height)
        })
        .collect();
    if dice.unit() < 0.7 {
        bumps.push((
            dice.range(0.0..length),
            dice.range(30.0..45.0),
            dice.range(2.0..3.5),
        ));
    }
    let mut heights: Vec<f32> = dists
        .iter()
        .map(|s| {
            let raw: f32 = bumps
                .iter()
                .map(|(centre, half, height)| bump(*s, length, *centre, *half, *height))
                .sum();
            // Level ground for the grid and the start line
            let d = s.min(length - s);
            let calm = ((d - 30.0) / 50.0).clamp(0.0, 1.0);
            raw * calm * calm * (3.0 - 2.0 * calm)
        })
        .collect();
    let n = heights.len();
    let steepest = (0..n)
        .map(|i| (heights[(i + 1) % n] - heights[i]).abs() / Track::STEP)
        .fold(0.0, f32::max);
    if steepest > MAX_GRADE {
        let k = MAX_GRADE / steepest;
        for h in &mut heights {
            *h *= k;
        }
    }
    heights
}

/// Height at `s` metres along a lap of `length` of a smooth hill (or dip) of `height`,
/// centred `centre` metres along and reaching `half` metres either side.
pub(super) fn bump(
    s: f32,
    length: f32,
    centre: f32,
    half: f32,
    height: f32,
) -> f32 {
    let d = (s - centre).abs();
    let d = d.min(length - d);
    if d < half {
        height * 0.5 * (1.0 + (std::f32::consts::PI * d / half).cos())
    } else {
        0.0
    }
}

/// Cross-slopes that lean tight corners in towards their inside, `lean` scaling how
/// much, eased in and out along the road.
pub(super) fn banks(
    points: &[Vec2],
    lean: f32,
) -> Vec<f32> {
    let n = points.len();
    let raw: Vec<f32> = (0..n)
        .map(|i| {
            let (a, b, c) = (points[(i + n - 6) % n], points[i], points[(i + 6) % n]);
            // Signed curvature, positive turning left
            let k = (b - a).perp_dot(c - b).signum() / bend(a, b, c);
            // Gentle curves stay flat; tight ones bank up to the cap
            (-(k.abs() - 0.012).max(0.0) * 6.0 * lean).max(-MAX_BANK) * k.signum()
        })
        .collect();
    // Ease over 20 m either side so the road twists smoothly
    let reach = (20.0 / Track::STEP) as usize;
    (0..n)
        .map(|i| {
            let sum: f32 = (0..=2 * reach).map(|k| raw[(i + n + k - reach) % n]).sum();
            sum / (2 * reach + 1) as f32
        })
        .collect()
}

/// One or two jumps on straight road away from the start and the crates: a kicker that
/// throws the kart off its lip, or a ramp over a short gap to a landing ramp.
fn jumps(
    dice: &mut Dice,
    track: &mut Track,
    zone: &[bool],
) {
    let n = track.points.len();
    let p = track.points.clone();
    let straight = |i: usize| bend(p[(i + n - 16) % n], p[i], p[(i + 16) % n]);
    let boxes: Vec<f32> = track.boxes.iter().map(|f| f * track.length).collect();
    let mut sites: Vec<f32> = Vec::new();
    for _ in 0..dice.range(1.0..3.0) as usize {
        let site = (0..n)
            .step_by(4)
            .filter(|i| !zone[*i])
            .map(|i| track.dists[i])
            .filter(|s| {
                let clear = |x: f32, room: f32| {
                    let d = (s - x).abs();
                    d.min(track.length - d) > room
                };
                clear(0.0, 90.0)
                    && boxes.iter().all(|b| clear(*b, 45.0))
                    && sites.iter().all(|o| clear(*o, 150.0))
            })
            .max_by(|a, b| {
                let r = |s: &f32| straight((*s / Track::STEP) as usize % n);
                r(a).total_cmp(&r(b))
            });
        let Some(site) = site else { break };
        // Only fairly straight road; a ramp mid-corner throws karts at the wall
        if straight((site / Track::STEP) as usize % n) < 60.0 {
            break;
        }
        sites.push(site);
        kicker(track, site, dice.unit() < 0.45);
    }
}

/// A kicker whose lip is at `site` metres along the lap, or, with `gap`, a ramp over a
/// short pit to a landing ramp.
pub(super) fn kicker(
    track: &mut Track,
    site: f32,
    gap: bool,
) {
    const UP: f32 = 8.0;
    const LIP: f32 = 1.6;
    const GAP: f32 = 6.0;
    const DOWN: f32 = 10.0;
    let land = site + UP + if gap { GAP } else { 0.0 };
    let end = if gap { land + DOWN } else { site + UP };
    for (i, s) in track.dists.iter().enumerate() {
        let ahead = (s - site).rem_euclid(track.length);
        let into = ahead - UP;
        if ahead < UP {
            track.heights[i] += LIP * ahead / UP;
        } else if gap && into < GAP {
            track.heights[i] += LIP;
        } else if gap && into - GAP < DOWN {
            track.heights[i] += LIP * 0.75 * (1.0 - (into - GAP) / DOWN);
        }
    }
    track.ramps.push((site, site + UP));
    if gap {
        track.gaps.push((site + UP, land));
        track.ramps.push((land, end));
    }
}

/// Half-pipes on up to two sweeping corners, clear of the start, bridges and jumps, eased
/// in and out over 15 m. All zero when no corner fits.
fn pipes(
    track: &Track,
    zone: &[bool],
) -> Vec<f32> {
    let n = track.points.len();
    let p = &track.points;
    let sweep = |i: usize| (25.0..70.0).contains(&bend(p[(i + n - 8) % n], p[i], p[(i + 8) % n]));
    let free = |i: usize| {
        let s = track.dists[i];
        let start = s.min(track.length - s) > 60.0;
        let jumps = track.ramps.iter().all(|(from, to)| {
            let (a, b) = (
                (s - from).rem_euclid(track.length),
                (to - s).rem_euclid(track.length),
            );
            a.min(track.length - a) > 25.0 && b.min(track.length - b) > 25.0
        });
        start && jumps && !zone[i]
    };
    // Runs of at least 40 m of sweeping, free road
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut from = None;
    for i in 0..n {
        match (sweep(i) && free(i), from) {
            (true, None) => from = Some(i),
            (false, Some(f)) => {
                runs.push((f, i));
                from = None;
            }
            _ => {}
        }
    }
    let least = (40.0 / Track::STEP) as usize;
    runs.retain(|(a, b)| b - a >= least);
    runs.sort_by_key(|(a, b)| std::cmp::Reverse(b - a));
    let mut pipes = vec![0.0; n];
    for (a, b) in runs.into_iter().take(2) {
        curl(&mut pipes, a, b);
    }
    pipes
}

/// Curl samples `from..to` into a half-pipe, eased in and out over 15 m.
pub(super) fn curl(
    pipes: &mut [f32],
    from: usize,
    to: usize,
) {
    let ease = 15.0 / Track::STEP;
    for (i, pipe) in pipes.iter_mut().enumerate().take(to).skip(from) {
        *pipe = ((i - from) as f32 / ease)
            .min((to - 1 - i) as f32 / ease)
            .min(1.0);
    }
}

/// Legs of road near each other sit at about the same height.
pub(super) fn level(
    track: &Track,
    zone: &[bool],
) -> bool {
    let n = track.points.len();
    let near = 2.0 * track.barrier() + 2.0 * Track::FALL;
    let p = &track.points;
    let h = &track.heights;
    // Banking lifts a road edge above its centre
    let lift = |i: usize| {
        track.banks[i].abs() * track.barrier() + track.pipes.get(i).unwrap_or(&0.0) * Track::PIPE
    };
    // Samples this close along the lap are one leg climbing its own hill
    let same = (near * 2.0 / Track::STEP) as usize;
    (0..n).step_by(2).all(|i| {
        (0..n).step_by(2).all(|j| {
            let apart = i.abs_diff(j);
            apart.min(n - apart) < same
                || zone[i]
                || zone[j]
                || (h[i] - h[j]).abs() + lift(i) + lift(j) <= STEP_DOWN + 2.0
                || p[i].distance_squared(p[j]) > near * near
        })
    })
}

/// Radius of the circle through three points.
fn bend(
    a: Vec2,
    b: Vec2,
    c: Vec2,
) -> f32 {
    let cross = (b - a).perp_dot(c - a).abs();
    a.distance(b) * b.distance(c) * c.distance(a) / (2.0 * cross).max(1e-6)
}

/// No curve tighter than [`MIN_RADIUS`], measured over 12 m of road.
fn smooth(points: &[Vec2]) -> bool {
    let n = points.len();
    (0..n).all(|i| bend(points[(i + n - 4) % n], points[i], points[(i + 4) % n]) >= MIN_RADIUS)
}

/// Legs of road far apart along the lap also stay apart on the ground, walls included.
/// Legs that pass over each other by at least [`Track::CLEAR`] are fine; `heights` may be
/// empty for a flat track.
fn clear(
    points: &[Vec2],
    heights: &[f32],
    barrier: f32,
) -> bool {
    let over =
        |i: usize, j: usize| !heights.is_empty() && (heights[i] - heights[j]).abs() >= Track::CLEAR;
    let n = points.len();
    let near = 2.0 * barrier + GAP;
    // Samples closer than this along the lap belong to the same leg
    let same = (near * 2.5 / Track::STEP) as usize;
    (0..n).all(|i| {
        (i + same..n)
            .filter(|j| n - j + i >= same)
            .all(|j| over(i, j) || points[i].distance_squared(points[j]) > near * near)
    })
}

/// Three rows of item crates, each on the straightest road of its third of the lap.
fn boxes(
    track: &Track,
    zone: &[bool],
) -> Vec<f32> {
    let n = track.points.len();
    let p = &track.points;
    (0..3)
        .map(|third| {
            let from = n * (2 * third + 1) / 8;
            let to = n * (2 * third + 3) / 8;
            let best = (from..to)
                .filter(|i| !zone[*i])
                .max_by(|a, b| {
                    let r = |i: &usize| bend(p[(*i + n - 6) % n], p[*i], p[(*i + 6) % n]);
                    r(a).total_cmp(&r(b))
                })
                .unwrap_or(from);
            track.dists[best] / track.length
        })
        .collect()
}

/// The open spot farthest from any road, if it leaves `room` beyond the walls.
fn landmark(
    track: &Track,
    room: f32,
) -> Option<Vec2> {
    let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for p in &track.points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let mut best = (0.0, Vec2::ZERO);
    let mut z = lo.y;
    while z < hi.y {
        let mut x = lo.x;
        while x < hi.x {
            let at = Vec2::new(x, z);
            let d = track
                .points
                .iter()
                .step_by(2)
                .map(|p| p.distance_squared(at))
                .fold(f32::MAX, f32::min);
            if d > best.0 {
                best = (d, at);
            }
            x += 8.0;
        }
        z += 8.0;
    }
    (best.0.sqrt() > track.barrier() + room).then_some(best.1)
}

fn name(
    dice: &mut Dice,
    scenery: Scenery,
) -> String {
    let (first, second): (&[&str], &[&str]) = match scenery {
        Scenery::Farm => (
            &["Hayride", "Orchard", "Pumpkin", "Haystack", "Cider"],
            &["Bends", "Lane", "Loop", "Run", "Trail"],
        ),
        Scenery::Table => (
            &["Gravy", "Stuffing", "Cranberry", "Drumstick", "Butter"],
            &["Dash", "Ring", "Sprint", "Spiral", "Circuit"],
        ),
        Scenery::Maze => (
            &["Cornrow", "Husk", "Kernel", "Scarecrow", "Silo"],
            &["Twist", "Tangle", "Rush", "Snake", "Wind"],
        ),
    };
    format!("{} {}", dice.pick(first), dice.pick(second))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_same_seed_same_track() {
        let (a, b) = (track(42), track(42));
        assert_eq!(a.points, b.points);
        assert_eq!(a.name, b.name);
    }

    #[test]
    fn test_seeds_pass_checks() {
        for seed in 0..50 {
            let t = track(seed);
            assert!(
                LENGTH.contains(&t.length),
                "seed {seed}: length {}",
                t.length
            );
            assert!(smooth(&t.points), "seed {seed}");
            assert!(clear(&t.points, &t.heights, t.barrier()), "seed {seed}");
            assert_eq!(t.boxes.len(), 3);
            assert!(level(&t, &zone(&t)), "seed {seed}");
            assert!(t.hilly(), "seed {seed}: flat");
        }
    }

    /// Tight corners bank, in towards their inside, and never past the cap.
    #[test]
    fn test_corners_bank() {
        let steepest = (0..20)
            .flat_map(|seed| track(seed).banks)
            .fold(0.0f32, |m, b| m.max(b.abs()));
        assert!(
            steepest > 0.08 && steepest <= MAX_BANK,
            "steepest bank {steepest}"
        );
    }

    /// Jumps turn up: some tracks get a kicker, some a gap, each gap between ramps.
    #[test]
    fn test_jumps() {
        let tracks: Vec<Track> = (0..30).map(track).collect();
        assert!(tracks.iter().filter(|t| !t.ramps.is_empty()).count() >= 10);
        assert!(tracks.iter().filter(|t| !t.gaps.is_empty()).count() >= 3);
        for t in &tracks {
            for (from, to) in &t.gaps {
                assert!(t.ramps.iter().any(|r| r.1 == *from));
                assert!(t.ramps.iter().any(|r| r.0 == *to));
            }
        }
    }

    /// Some tracks are figure eights with a bridge high over the road it crosses.
    #[test]
    fn test_bridges() {
        let tracks: Vec<Track> = (0..30).map(track).collect();
        let bridged: Vec<&Track> = tracks.iter().filter(|t| !t.decks.is_empty()).collect();
        assert!(bridged.len() >= 4, "{} bridges", bridged.len());
        for t in bridged {
            // A kart on the road, deck or under it, is found on its own leg
            for (i, p) in t.points.iter().enumerate() {
                let along = t.find(*p, t.heights[i]).along;
                let off = (along - t.dists[i]).abs();
                assert!(
                    off.min(t.length - off) < 2.0,
                    "{}: {i} found at {along}",
                    t.name
                );
            }
        }
    }

    /// Some tracks curl the road up into half-pipes on sweeping corners.
    #[test]
    fn test_pipes() {
        let piped = (0..30)
            .map(track)
            .filter(|t| t.pipes.iter().any(|p| *p > 0.99))
            .count();
        assert!(piped >= 5, "{piped} tracks with half-pipes");
    }

    /// Different seeds give different layouts, not one shape over and over.
    #[test]
    fn test_seeds_differ() {
        let lengths: Vec<i32> = (0..10).map(|s| track(s).length as i32).collect();
        let mut unique = lengths.clone();
        unique.sort_unstable();
        unique.dedup();
        assert!(unique.len() >= 8, "{lengths:?}");
    }
}
