//! Hand-authored elevation, banking and features for the hand-made tracks, built from
//! the same pieces the generator uses for random ones.

use super::Track;
use super::generate;

/// Hills as (centre, half width, height) in metres along
/// the lap; level ground around the start line.
fn hills(
    track: &mut Track,
    bumps: &[(f32, f32, f32)],
) {
    let length = track.length;
    for (s, h) in track.dists.iter().zip(&mut track.heights) {
        let raw: f32 = bumps
            .iter()
            .map(|(centre, half, height)| generate::bump(*s, length, *centre, *half, *height))
            .sum();
        let d = s.min(length - s);
        let calm = ((d - 30.0) / 50.0).clamp(0.0, 1.0);
        *h = raw * calm * calm * (3.0 - 2.0 * calm);
    }
}

/// A table top: the road `height` above its surroundings from `from` to `to` metres along
/// the lap, rising and falling over `ease` metres either side.
fn level_up(
    track: &mut Track,
    from: f32,
    to: f32,
    height: f32,
    ease: f32,
) {
    let centre = f32::midpoint(from, to);
    let half = (to - from) / 2.0 + ease;
    for (s, h) in track.dists.iter().zip(&mut track.heights) {
        let d = (s - centre).abs();
        let up = ((half - d) / ease).clamp(0.0, 1.0);
        *h += height * up * up * (3.0 - 2.0 * up);
    }
}

/// Give the hand-made track `index` its hills, banking and features.
pub(super) fn profile(
    index: usize,
    track: &mut Track,
) {
    match index {
        0 => harvest(track),
        1 => feast(track),
        _ => {}
    }
}

/// Rolling farm hills, gentle banking and a hay-bale kicker on the first bend's approach.
fn harvest(track: &mut Track) {
    hills(
        track,
        &[
            (170.0, 70.0, 3.0),
            (300.0, 60.0, -2.5),
            (420.0, 80.0, 4.0),
            (545.0, 60.0, 2.5),
        ],
    );
    track.banks = generate::banks(&track.points, 0.5);
    generate::kicker(track, 80.0, false);
}

/// Table-top levels either side of the roast with a gap to jump on the start straight,
/// and a half-pipe curled round the far plate's rim.
fn feast(track: &mut Track) {
    level_up(track, 150.0, 240.0, 3.5, 35.0);
    level_up(track, 440.0, 520.0, 2.0, 25.0);
    track.banks = generate::banks(&track.points, 1.0);
    generate::kicker(track, 92.0, true);
    track.pipes = vec![0.0; track.points.len()];
    generate::curl(&mut track.pipes, 313, 350);
}

#[cfg(test)]
mod test {
    use super::*;

    /// Each hand-made track keeps level ground at the start, and nearby legs of road stay
    /// at about the same height.
    #[test]
    fn test_profiles_are_sound() {
        for index in 0..2 {
            let t = Track::new(index);
            let n = t.points.len();
            assert!(t.hilly(), "{}: flat", t.name);
            assert_eq!(t.heights.len(), n);
            assert_eq!(t.banks.len(), n);
            for along in [0.0, 10.0, t.length - 10.0] {
                assert!(t.height(along).abs() < 0.05, "{}: start not level", t.name);
            }
            assert!(
                generate::level(&t, &vec![false; n]),
                "{}: legs at odds",
                t.name
            );
        }
    }

    /// The hay-bale kicker on Harvest Lane is a ramp, not a gap.
    #[test]
    fn test_harvest_kicker() {
        let t = Track::new(0);
        assert_eq!(t.ramps.len(), 1);
        assert_eq!(t.gaps.len(), 0);
    }

    /// The Feast Table has a gap between dishes, each side a ramp, and a half-pipe.
    #[test]
    fn test_feast_features() {
        let t = Track::new(1);
        assert_eq!(t.gaps.len(), 1);
        let (from, to) = t.gaps[0];
        assert!(t.ramps.iter().any(|r| r.1 == from) && t.ramps.iter().any(|r| r.0 == to));
        assert!(t.pipes.iter().any(|p| *p > 0.99));
        // The top of the table stands above the start
        assert!(t.height(200.0) > 3.0);
    }
}
