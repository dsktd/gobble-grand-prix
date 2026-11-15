//! Seeded random tracks: a loop around a centre, bent by a mix of corner kinds.
//!
//! Control points sit at increasing angles around the origin, so the loop starts out
//! free of crossings; hairpins, chicanes, esses, sweepers and straights then push them
//! in, out or onto a line. Every candidate is checked for curves the karts cannot make
//! and for legs of road that come too close, and the next attempt is tried until one
//! passes. The same seed always gives the same track.

use std::f32::consts::TAU;

use bevy::prelude::*;

use super::Scenery;
use super::TRACKS;
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
    let control = layout(&mut dice);
    let (points, dists, length) = centerline(&control);
    let barrier = half_width + 5.0;
    if !LENGTH.contains(&length) || !smooth(&points) || !clear(&points, barrier) {
        return None;
    }
    let mut track = Track {
        name: name(&mut dice, TRACKS[theme].scenery),
        points,
        dists,
        length,
        half_width,
        heights: Vec::new(),
        banks: Vec::new(),
        ramps: Vec::new(),
        gaps: Vec::new(),
        shortcuts: Vec::new(),
        boxes: Vec::new(),
        scenery: TRACKS[theme].scenery,
        theme: TRACKS[theme].theme,
        landmark: None,
    };
    track.boxes = boxes(&track);
    track.landmark = landmark(&track, 22.0);
    track.heights = hills(&mut dice, &track.dists, length);
    track.banks = banks(&track.points, dice.range(0.6..1.0));
    jumps(&mut dice, &mut track);
    level(&track).then_some(track)
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
    // The start line wants a straight run, so begin halfway along the longest leg
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
                .map(|(centre, half, height)| {
                    let d = (s - centre).abs();
                    let d = d.min(length - d);
                    if d < *half {
                        height * 0.5 * (1.0 + (std::f32::consts::PI * d / half).cos())
                    } else {
                        0.0
                    }
                })
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

/// Cross-slopes that lean tight corners in towards their inside, `lean` scaling how
/// much, eased in and out along the road.
fn banks(
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
) {
    const UP: f32 = 8.0;
    const LIP: f32 = 1.6;
    const GAP: f32 = 6.0;
    const DOWN: f32 = 10.0;
    let n = track.points.len();
    let p = &track.points;
    let straight = |i: usize| bend(p[(i + n - 16) % n], p[i], p[(i + 16) % n]);
    let boxes: Vec<f32> = track.boxes.iter().map(|f| f * track.length).collect();
    let mut sites: Vec<f32> = Vec::new();
    for _ in 0..dice.range(1.0..3.0) as usize {
        let site = (0..n)
            .step_by(4)
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
        let gap = dice.unit() < 0.45;
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
}

/// Legs of road near each other sit at about the same height.
fn level(track: &Track) -> bool {
    let n = track.points.len();
    let near = 2.0 * track.barrier() + 2.0 * Track::FALL;
    let p = &track.points;
    let h = &track.heights;
    // Banking lifts a road edge above its centre
    let lift = |i: usize| track.banks[i].abs() * track.barrier();
    // Samples this close along the lap are one leg climbing its own hill
    let same = (near * 2.0 / Track::STEP) as usize;
    (0..n).step_by(2).all(|i| {
        (0..n).step_by(2).all(|j| {
            let apart = i.abs_diff(j);
            apart.min(n - apart) < same
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
fn clear(
    points: &[Vec2],
    barrier: f32,
) -> bool {
    let n = points.len();
    let near = 2.0 * barrier + GAP;
    // Samples closer than this along the lap belong to the same leg
    let same = (near * 2.5 / Track::STEP) as usize;
    (0..n).all(|i| {
        (i + same..n)
            .filter(|j| n - j + i >= same)
            .all(|j| points[i].distance_squared(points[j]) > near * near)
    })
}

/// Three rows of item crates, each on the straightest road of its third of the lap.
fn boxes(track: &Track) -> Vec<f32> {
    let n = track.points.len();
    let p = &track.points;
    (0..3)
        .map(|third| {
            let from = n * (2 * third + 1) / 8;
            let to = n * (2 * third + 3) / 8;
            let best = (from..to)
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
            assert!(clear(&t.points, t.barrier()), "seed {seed}");
            assert_eq!(t.boxes.len(), 3);
            assert!(level(&t), "seed {seed}");
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
