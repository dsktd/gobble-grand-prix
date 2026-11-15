//! The corner bar: green felt under pendant lamps, a racked triangle of giant balls, cue
//! racks, chalk, stools, bottles and chalkboards.

use std::f32::consts::FRAC_PI_2;
use std::f32::consts::PI;

use bevy::prelude::*;
use rand::Rng;

use super::CEILING;
use super::Dress;
use super::props;
use super::textures;
use crate::prelude::*;

pub(super) fn dress(d: &mut Dress) {
    lamps(d);
    if let Some(at) = d.track.landmark {
        props::rack(d, at, 3.4, Color::srgb(0.45, 0.25, 0.10));
    }
    props::balls(d, 16);
    props::cues(d, 9, 3, Color::srgb(0.32, 0.16, 0.07));
    props::chalk(d, 11);
    props::stools(
        d,
        12,
        Color::srgb(0.62, 0.08, 0.10),
        Color::srgb(0.75, 0.75, 0.78),
    );
    drinks(d);
    chalkboards(d);
    spills(d);
}

/// Bottles and glasses, each with a puddle where it leaked.
fn drinks(d: &mut Dress) {
    let amber = Color::srgb(0.45, 0.22, 0.05);
    let body = d.shiny(Cylinder::new(1.7, 8.0), amber, 0.1);
    let neck = d.shiny(Cylinder::new(0.7, 5.0), amber, 0.1);
    let label = d.kit(Cylinder::new(1.75, 3.0), Color::srgb(0.95, 0.88, 0.6));
    let cap = d.shiny(Cylinder::new(0.85, 0.5), Color::srgb(0.85, 0.7, 0.2), 0.3);
    let glass = d.glass(
        Cylinder::new(2.3, 6.5),
        Color::srgba(0.85, 0.55, 0.12, 0.55),
    );
    let foam = d.kit(Cylinder::new(2.35, 1.1), Color::srgb(0.97, 0.94, 0.85));
    let spill = d.mark(
        textures::puddle(),
        Vec2::splat(11.0),
        Color::srgb(0.75, 0.42, 0.08),
        0.0,
    );
    for place in d.spots(14, 5.0, 46.0, 7.0) {
        let yaw = d.rng.random_range(0.0..PI * 2.0);
        d.stain(&spill, place.p + Vec2::from_angle(yaw) * 3.0, yaw);
        if d.rng.random_bool(0.6) {
            // A bottle, sometimes fallen on its side
            let fallen = d.rng.random_bool(0.3);
            let turn = Quat::from_rotation_y(yaw)
                * if fallen {
                    Quat::from_rotation_z(FRAC_PI_2)
                } else {
                    Quat::IDENTITY
                };
            let lift = if fallen { 1.7 } else { 0.0 };
            for (kit, y) in [(&body, 4.0), (&label, 3.8), (&neck, 10.5), (&cap, 13.25)] {
                d.put(
                    kit,
                    Transform::from_translation(flat(place.p, lift) + turn * Vec3::Y * y)
                        .with_rotation(turn),
                );
            }
        } else {
            d.put(&glass, Transform::from_translation(flat(place.p, 3.25)));
            d.put(&foam, Transform::from_translation(flat(place.p, 6.9)));
        }
    }
}

/// Chalkboard scoreboards with tallies, on a pair of posts.
fn chalkboards(d: &mut Dress) {
    let wood = d.kit(Cuboid::new(1.0, 1.0, 1.0), Color::srgb(0.35, 0.18, 0.08));
    let slate = d.kit(Cuboid::new(1.0, 1.0, 1.0), Color::srgb(0.04, 0.11, 0.08));
    let stroke = d.kit(Cuboid::new(0.28, 3.4, 0.12), Color::srgb(0.95, 0.95, 0.92));
    let slash = d.kit(Cuboid::new(0.28, 5.4, 0.12), Color::srgb(0.95, 0.95, 0.92));
    for place in d.spots(4, 10.0, 34.0, 12.0) {
        let face = place.facing();
        let at = |x: f32, y: f32, z: f32| flat(place.p, y) + face * Vec3::new(x, 0.0, z);
        for x in [-8.5f32, 8.5] {
            d.put(
                &wood,
                Transform::from_translation(at(x, 7.0, 0.0))
                    .with_rotation(face)
                    .with_scale(Vec3::new(1.2, 14.0, 1.2)),
            );
        }
        d.put(
            &wood,
            Transform::from_translation(at(0.0, 10.5, 0.0))
                .with_rotation(face)
                .with_scale(Vec3::new(18.4, 11.4, 0.5)),
        );
        d.put(
            &slate,
            Transform::from_translation(at(0.0, 10.5, 0.3))
                .with_rotation(face)
                .with_scale(Vec3::new(17.0, 10.0, 0.4)),
        );
        for row in 0..2 {
            for group in 0..3 {
                let x0 = -6.5 + group as f32 * 5.2;
                let y = 13.0 - row as f32 * 5.0;
                let marks = d.rng.random_range(1..=5);
                for k in 0..marks.min(4) {
                    d.put(
                        &stroke,
                        Transform::from_translation(at(x0 + k as f32 * 0.9, y, 0.58))
                            .with_rotation(face),
                    );
                }
                if marks == 5 {
                    d.put(
                        &slash,
                        Transform::from_translation(at(x0 + 1.35, y, 0.66))
                            .with_rotation(face * Quat::from_rotation_z(FRAC_PI_2 * 0.0 + 1.25)),
                    );
                }
            }
        }
    }
}

/// Spilled drinks on the cloth, a slick to look at.
fn spills(d: &mut Dress) {
    let spill = d.mark(
        textures::puddle(),
        Vec2::new(8.0, 6.0),
        Color::srgb(0.75, 0.42, 0.08),
        0.0,
    );
    let lap = d.track.length;
    let hw = d.track.half_width;
    for _ in 0..8 {
        let along = d.rng.random_range(0.0..lap);
        if d.track
            .gaps
            .iter()
            .any(|(from, to)| (*from - 6.0..*to + 6.0).contains(&along))
        {
            continue;
        }
        let lateral = d.rng.random_range(-hw * 0.75..hw * 0.75);
        let yaw = d.rng.random_range(0.0..PI);
        d.decal(&spill, along, lateral, yaw);
    }
}

/// Green-shaded pendant lamps over the felt; the real light only in high quality.
fn lamps(d: &mut Dress) {
    let shade = d.shiny(Cone::new(8.0, 6.0), Color::srgb(0.08, 0.35, 0.18), 0.3);
    let bulb = d.glow(Sphere::new(2.0), Color::srgb(1.0, 0.85, 0.5), 9.0);
    let lap = d.track.length;
    for k in 0..5 {
        let along = lap * (0.06 + k as f32 * 0.2);
        let (p, _) = d.track.sample(along);
        let y = d.track.height(along) + 36.0;
        let chain = d.kit(
            Cylinder::new(0.25, CEILING - y - 6.0),
            Color::srgb(0.1, 0.08, 0.05),
        );
        d.put_abs(&shade, Transform::from_translation(flat(p, y + 3.0)));
        d.put_abs(&bulb, Transform::from_translation(flat(p, y - 0.5)));
        d.put_abs(
            &chain,
            Transform::from_translation(flat(p, y + 6.0 + (CEILING - y - 6.0) / 2.0)),
        );
        if d.high {
            d.light(Color::srgb(1.0, 0.82, 0.5), 6.0e7, 150.0, flat(p, y - 3.0));
        }
    }
}
