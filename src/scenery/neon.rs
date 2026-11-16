//! The neon hall after midnight: dark felt, glowing tubes and signs on poles, a jukebox,
//! cigarette machines glowing in the haze, and a few regulars.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use rand::Rng;

use super::Dress;
use super::Kit;
use super::props;
use crate::prelude::*;

const CYAN: Color = Color::srgb(0.12, 0.95, 0.92);
const PINK: Color = Color::srgb(0.98, 0.15, 0.68);
const GOLD: Color = Color::srgb(1.0, 0.78, 0.2);
const LIME: Color = Color::srgb(0.55, 1.0, 0.25);

pub(super) fn dress(d: &mut Dress) {
    tubes(d);
    jukebox(d);
    signs(d);
    machines(d);
    regulars(d);
    props::balls(d, 10);
    props::cues(d, 6, 1, Color::srgb(0.08, 0.04, 0.12));
    props::chalk(d, 6);
    props::stools(
        d,
        10,
        Color::srgb(0.45, 0.10, 0.55),
        Color::srgb(0.7, 0.75, 0.85),
    );
}

/// A rotation that turns local +Z from `from` towards `to` on the ground plane.
fn turn_to(
    from: Vec2,
    to: Vec2,
) -> Quat {
    let v = to - from;
    Quat::from_rotation_y(v.x.atan2(v.y))
}

/// Rows of glowing tubes hem the road in, cyan and pink in turn.
fn tubes(d: &mut Dress) {
    let cyan = d.glow(Cylinder::new(0.18, 5.5), CYAN, 6.0);
    let pink = d.glow(Cylinder::new(0.18, 5.5), PINK, 6.0);
    let caps = [
        d.glow(Cone::new(0.35, 0.8), CYAN, 9.0),
        d.glow(Cone::new(0.35, 0.8), PINK, 9.0),
    ];
    let rows = if d.high { 3 } else { 2 };
    let edge = d.track.barrier() + 1.5;
    let lap = d.track.length;
    let mut along = 0.0;
    let mut n = 0;
    while along < lap {
        for side in [-1.0f32, 1.0] {
            for row in 0..rows {
                let jitter = d.rng.random_range(-0.8..0.8);
                let p = d
                    .track
                    .offset(along + jitter, side * (edge + 1.0 + row as f32 * 2.2));
                if !d.free(p, 0.0) {
                    continue;
                }
                let h = d.rng.random_range(0.8..1.25);
                let pick = (n + row) % 2;
                let kit = if pick == 0 { &cyan } else { &pink };
                d.put(
                    kit,
                    Transform::from_translation(flat(p, 2.75 * h))
                        .with_scale(Vec3::new(1.0, h, 1.0)),
                );
                d.put(
                    &caps[pick],
                    Transform::from_translation(flat(p, 5.5 * h + 0.4)),
                );
            }
        }
        n += 1;
        along += 2.6;
    }
}

/// A jukebox with an arched top and glowing tubes, facing the road, at the landmark.
fn jukebox(d: &mut Dress) {
    let Some(at) = d.track.landmark else { return };
    let near = d.track.locate(at);
    let face = turn_to(at, d.track.sample(near.along).0);
    let k = 1.7;
    let body = d.shiny(
        Cuboid::new(16.0 * k, 13.0 * k, 10.0 * k),
        Color::srgb(0.30, 0.04, 0.22),
        0.25,
    );
    let dome = d.shiny(
        Cylinder::new(8.0 * k, 10.0 * k),
        Color::srgb(0.30, 0.04, 0.22),
        0.25,
    );
    let screen = d.glow(Cuboid::new(11.0 * k, 7.0 * k, 0.3 * k), GOLD, 4.0);
    let arch = d.glow(Torus::new(6.6 * k, 7.4 * k), PINK, 7.0);
    let tube_a = d.glow(Cuboid::new(0.8 * k, 15.0 * k, 0.6 * k), CYAN, 7.0);
    let tube_b = d.glow(Cuboid::new(0.8 * k, 15.0 * k, 0.6 * k), LIME, 7.0);
    let at3 = |x: f32, y: f32, z: f32| flat(at, y * k) + face * Vec3::new(x * k, 0.0, z * k);
    d.put(
        &body,
        Transform::from_translation(at3(0.0, 6.5, 0.0)).with_rotation(face),
    );
    d.put(
        &dome,
        Transform::from_translation(at3(0.0, 13.0, 0.0))
            .with_rotation(face * Quat::from_rotation_x(FRAC_PI_2)),
    );
    d.put(
        &screen,
        Transform::from_translation(at3(0.0, 8.0, 5.1)).with_rotation(face),
    );
    d.put(
        &arch,
        Transform::from_translation(at3(0.0, 13.0, 5.0))
            .with_rotation(face * Quat::from_rotation_x(FRAC_PI_2))
            .with_scale(Vec3::new(1.0, 0.4, 1.0)),
    );
    d.put(
        &tube_a,
        Transform::from_translation(at3(-8.2, 7.5, 3.0)).with_rotation(face),
    );
    d.put(
        &tube_b,
        Transform::from_translation(at3(8.2, 7.5, 3.0)).with_rotation(face),
    );
    if d.high {
        d.light(
            PINK,
            3.0e7,
            90.0,
            flat(at, 16.0 * k) + face * Vec3::Z * 12.0,
        );
    }
}

/// Neon signs on poles: rings, an eight, an arrow, bars and a cue, with coloured light and haze.
fn signs(d: &mut Dress) {
    let pole = d.shiny(Cylinder::new(0.4, 24.0), Color::srgb(0.08, 0.06, 0.12), 0.4);
    let haze = Kit {
        mesh: d.meshes.add(Cone::new(11.0, 26.0)),
        material: d.materials.add(StandardMaterial {
            base_color: Color::srgba(0.7, 0.2, 0.9, 0.035),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            cull_mode: None,
            double_sided: true,
            ..default()
        }),
    };
    let ring = |d: &mut Dress, r: f32, c: Color| d.glow(Torus::new(r - 0.3, r + 0.3), c, 7.0);
    let bar = |d: &mut Dress, len: f32, c: Color| d.glow(Cuboid::new(len, 0.8, 0.8), c, 7.0);
    let big = ring(d, 5.5, CYAN);
    let small = ring(d, 3.0, PINK);
    let eight = ring(d, 3.0, GOLD);
    let long = bar(d, 12.0, CYAN);
    let mid = bar(d, 12.0, PINK);
    let short = bar(d, 12.0, GOLD);
    let shaft = bar(d, 12.0, LIME);
    let head = d.glow(Cone::new(2.2, 4.0), LIME, 7.0);
    let flat_ring = Quat::from_rotation_x(FRAC_PI_2);
    let colors = [CYAN, PINK, GOLD, LIME, CYAN, PINK];
    for (k, place) in d.spots(10, 12.0, 42.0, 16.0).into_iter().enumerate() {
        let face = turn_to(place.p, d.track.sample(d.track.locate(place.p).along).0);
        let at = |x: f32, y: f32| flat(place.p, y) + face * Vec3::new(x, 0.0, 0.0);
        d.put(&pole, Transform::from_translation(at(0.0, 12.0)));
        let y = 29.0;
        match k % 5 {
            0 => {
                d.put(
                    &big,
                    Transform::from_translation(at(0.0, y)).with_rotation(face * flat_ring),
                );
                d.put(
                    &small,
                    Transform::from_translation(at(0.0, y)).with_rotation(face * flat_ring),
                );
            }
            1 => {
                d.put(
                    &eight,
                    Transform::from_translation(at(0.0, y + 3.2)).with_rotation(face * flat_ring),
                );
                d.put(
                    &eight,
                    Transform::from_translation(at(0.0, y - 3.2)).with_rotation(face * flat_ring),
                );
            }
            2 => {
                d.put(
                    &shaft,
                    Transform::from_translation(at(0.0, y)).with_rotation(face),
                );
                d.put(
                    &head,
                    Transform::from_translation(at(0.0, y) + face * Vec3::X * 7.0)
                        .with_rotation(face * Quat::from_rotation_z(-FRAC_PI_2)),
                );
            }
            3 => {
                for (kit, dy) in [(&long, 3.0), (&mid, 0.0), (&short, -3.0)] {
                    d.put(
                        kit,
                        Transform::from_translation(at(0.0, y + dy)).with_rotation(face),
                    );
                }
            }
            _ => {
                d.put(
                    &long,
                    Transform::from_translation(at(0.0, y))
                        .with_rotation(face * Quat::from_rotation_z(0.5)),
                );
                d.put(
                    &small,
                    Transform::from_translation(at(5.0, y + 2.9)).with_rotation(face * flat_ring),
                );
            }
        }
        d.put(&haze, Transform::from_translation(flat(place.p, 13.0)));
        if d.high && k < 6 {
            d.light(
                colors[k],
                2.0e7,
                80.0,
                flat(place.p, y - 2.0) + face * Vec3::Z * 6.0,
            );
        }
    }
}

/// Cigarette machines glowing in the haze.
fn machines(d: &mut Dress) {
    let body = d.shiny(
        Cuboid::new(7.0, 14.0, 5.0),
        Color::srgb(0.12, 0.14, 0.2),
        0.3,
    );
    let window = d.glow(Cuboid::new(5.2, 6.0, 0.3), Color::srgb(0.6, 0.95, 1.0), 3.5);
    let packs: Vec<Kit> = [PINK, GOLD, LIME]
        .iter()
        .map(|c| d.glow(Cuboid::new(1.2, 1.6, 0.4), *c, 5.0))
        .collect();
    let slot = d.glow(Cuboid::new(1.0, 1.0, 0.3), Color::srgb(1.0, 0.1, 0.1), 6.0);
    for place in d.spots(6, 6.0, 30.0, 8.0) {
        let face = turn_to(place.p, d.track.sample(d.track.locate(place.p).along).0);
        let at = |x: f32, y: f32, z: f32| flat(place.p, y) + face * Vec3::new(x, 0.0, z);
        d.put(
            &body,
            Transform::from_translation(at(0.0, 7.0, 0.0)).with_rotation(face),
        );
        d.put(
            &window,
            Transform::from_translation(at(0.0, 9.5, 2.6)).with_rotation(face),
        );
        for (i, kit) in packs.iter().enumerate() {
            for row in 0..3 {
                d.put(
                    kit,
                    Transform::from_translation(at(
                        -1.8 + i as f32 * 1.8,
                        7.6 + row as f32 * 1.9,
                        2.8,
                    ))
                    .with_rotation(face),
                );
            }
        }
        d.put(
            &slot,
            Transform::from_translation(at(0.0, 3.0, 2.6)).with_rotation(face),
        );
    }
}

/// Silhouettes of regulars, hats and all, watching the road.
fn regulars(d: &mut Dress) {
    let shirt = d.kit(Capsule3d::new(1.3, 3.0), Color::srgb(0.05, 0.03, 0.09));
    let head = d.kit(Sphere::new(1.0), Color::srgb(0.06, 0.04, 0.08));
    let hat = d.kit(Cone::new(1.6, 1.4), Color::srgb(0.04, 0.03, 0.07));
    for place in d.spots(8, 3.0, 12.0, 5.0) {
        d.put(&shirt, Transform::from_translation(flat(place.p, 3.2)));
        d.put(&head, Transform::from_translation(flat(place.p, 6.0)));
        d.put(&hat, Transform::from_translation(flat(place.p, 7.4)));
    }
}
