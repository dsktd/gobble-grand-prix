//! The tournament hall: blue felt under bright even light, banners, light banks, bead
//! counters, a trophy podium and a crowd in the stands.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use rand::Rng;

use super::CEILING;
use super::Dress;
use super::Kit;
use super::Place;
use super::props;
use crate::prelude::*;

pub(super) fn dress(d: &mut Dress) {
    podium(d);
    props::balls(d, 14);
    props::cues(d, 7, 2, Color::srgb(0.06, 0.06, 0.08));
    props::chalk(d, 8);
    props::stools(
        d,
        8,
        Color::srgb(0.08, 0.08, 0.1),
        Color::srgb(0.8, 0.8, 0.85),
    );
    if d.rng.random_bool(0.6) {
        let spots = d.spots(1, 18.0, 40.0, 26.0);
        for place in spots {
            props::rack(d, place.p, 3.4, Color::srgb(0.06, 0.06, 0.08));
        }
    }
    banners(d);
    light_banks(d);
    counters(d);
    stands(d);
}

/// A trophy podium with the eight ball and cue ball on it, at the landmark.
fn podium(d: &mut Dress) {
    let Some(at) = d.track.landmark else { return };
    let step = d.kit(Cylinder::new(22.0, 2.5), Color::srgb(0.07, 0.07, 0.09));
    let gold = d.shiny(Torus::new(21.0, 22.8), Color::srgb(0.9, 0.7, 0.2), 0.25);
    d.put(&step, Transform::from_translation(flat(at, 1.25)));
    d.put(&gold, Transform::from_translation(flat(at, 2.6)));
    d.ball_on(
        8,
        at + Vec2::new(-6.0, 2.0),
        12.0,
        2.5,
        Quat::from_rotation_y(0.4),
    );
    d.ball_on(
        0,
        at + Vec2::new(11.0, -8.0),
        8.0,
        2.5,
        Quat::from_rotation_y(2.0),
    );
}

/// Tall banners on poles along both walls.
fn banners(d: &mut Dress) {
    let pole = d.shiny(Cylinder::new(0.5, 34.0), Color::srgb(0.12, 0.12, 0.15), 0.3);
    let arm = d.shiny(
        Cuboid::new(0.5, 0.5, 11.0),
        Color::srgb(0.12, 0.12, 0.15),
        0.3,
    );
    let colors = [
        Color::srgb(0.08, 0.20, 0.75),
        Color::srgb(0.92, 0.92, 0.95),
        Color::srgb(0.85, 0.68, 0.18),
        Color::srgb(0.75, 0.10, 0.15),
    ];
    let flags: Vec<Kit> = colors
        .iter()
        .map(|c| d.kit(Cuboid::new(0.25, 20.0, 9.0), *c))
        .collect();
    let bands: Vec<Kit> = colors
        .iter()
        .rev()
        .map(|c| d.kit(Cuboid::new(0.32, 3.0, 9.1), *c))
        .collect();
    let lap = d.track.length;
    let edge = d.track.barrier() + 1.5;
    for k in 0..18 {
        let side = if k % 2 == 0 { 1.0 } else { -1.0 };
        let along = lap * (k as f32 + 0.5) / 18.0;
        let p = d.track.offset(along, side * (edge + 5.0));
        if !d.free(p, 4.0) {
            continue;
        }
        d.claim(p, 4.0);
        let place = Place {
            p,
            dir: d.track.sample(along).1,
            side,
        };
        let face = place.facing();
        let at = |x: f32, y: f32, z: f32| flat(p, y) + face * Vec3::new(x, 0.0, z);
        d.put(&pole, Transform::from_translation(at(0.0, 17.0, 0.0)));
        d.put(
            &arm,
            Transform::from_translation(at(0.0, 33.5, 5.5)).with_rotation(face),
        );
        d.put(
            &flags[k % 4],
            Transform::from_translation(at(0.0, 23.0, 5.5)).with_rotation(face),
        );
        d.put(
            &bands[k % 4],
            Transform::from_translation(at(0.0, 16.5, 5.5)).with_rotation(face),
        );
    }
}

/// Banks of bright panels hanging over the felt, with real light under a few.
fn light_banks(d: &mut Dress) {
    let panel = d.glow(
        Cuboid::new(40.0, 1.0, 14.0),
        Color::srgb(1.0, 1.0, 0.96),
        4.0,
    );
    let frame = d.kit(Cuboid::new(42.0, 1.4, 16.0), Color::srgb(0.08, 0.08, 0.1));
    let rod = d.kit(
        Cylinder::new(0.3, CEILING - 70.0),
        Color::srgb(0.1, 0.1, 0.12),
    );
    let lap = d.track.length;
    for k in 0..6 {
        let along = lap * (k as f32 + 0.3) / 6.0;
        let (p, dir) = d.track.sample(along);
        let turn = Quat::from_rotation_y((-dir.y).atan2(dir.x));
        let y = d.track.height(along) + 62.0;
        d.put_abs(
            &frame,
            Transform::from_translation(flat(p, y)).with_rotation(turn),
        );
        d.put_abs(
            &panel,
            Transform::from_translation(flat(p, y - 0.5)).with_rotation(turn),
        );
        for (x, z) in [(-17.0, -5.0), (17.0, 5.0)] {
            let at = p + (turn * Vec3::new(x, 0.0, z)).xz();
            d.put_abs(
                &rod,
                Transform::from_translation(flat(at, y + (CEILING - y) / 2.0)),
            );
        }
        if d.high && k % 2 == 0 {
            d.light(Color::srgb(0.95, 0.97, 1.0), 2.5e8, 170.0, flat(p, y - 4.0));
        }
    }
}

/// Bead counters on posts, the sort that keep a tournament's score.
fn counters(d: &mut Dress) {
    let wood = d.shiny(
        Cuboid::new(1.0, 1.0, 1.0),
        Color::srgb(0.07, 0.07, 0.09),
        0.4,
    );
    let rod = d.shiny(Cylinder::new(0.22, 18.0), Color::srgb(0.8, 0.8, 0.85), 0.2);
    let reds = d.shiny(Sphere::new(0.85), Color::srgb(0.85, 0.12, 0.12), 0.25);
    let whites = d.shiny(Sphere::new(0.85), Color::srgb(0.95, 0.95, 0.9), 0.25);
    for place in d.spots(3, 10.0, 34.0, 14.0) {
        let face = place.facing();
        let at = |x: f32, y: f32, z: f32| flat(place.p, y) + face * Vec3::new(x, 0.0, z);
        for x in [-10.0f32, 10.0] {
            d.put(
                &wood,
                Transform::from_translation(at(x, 9.0, 0.0))
                    .with_rotation(face)
                    .with_scale(Vec3::new(1.4, 18.0, 1.4)),
            );
        }
        d.put(
            &wood,
            Transform::from_translation(at(0.0, 18.2, 0.0))
                .with_rotation(face)
                .with_scale(Vec3::new(22.0, 1.6, 1.8)),
        );
        d.put(
            &wood,
            Transform::from_translation(at(0.0, 1.0, 0.0))
                .with_rotation(face)
                .with_scale(Vec3::new(23.0, 1.6, 1.8)),
        );
        for row in 0..4 {
            let y = 4.0 + row as f32 * 3.6;
            d.put(
                &rod,
                Transform::from_translation(at(0.0, y, 0.0))
                    .with_rotation(face * Quat::from_rotation_z(FRAC_PI_2)),
            );
            let slid = d.rng.random_range(1..9);
            for bead in 0..10 {
                let x = if bead < slid {
                    -9.0 + bead as f32 * 1.7
                } else {
                    9.0 - (9 - bead) as f32 * 1.7
                };
                let kit = if row % 2 == 0 { &reds } else { &whites };
                d.put(kit, Transform::from_translation(at(x, y, 0.0)));
            }
        }
    }
}

/// Tiered stands around the table with a crowd of silhouettes in them.
fn stands(d: &mut Dress) {
    let (lo, hi) = d.track.table();
    let outer = 19.0 + 14.0;
    let (lo, hi) = (lo - Vec2::splat(outer), hi + Vec2::splat(outer));
    let step = d.kit(Cuboid::new(1.0, 1.0, 1.0), Color::srgb(0.10, 0.12, 0.22));
    let tops = [4.0, 14.0, 24.0];
    for (tier, top) in tops.iter().enumerate() {
        let inset = tier as f32 * 12.0;
        let height = top + Track::DROP;
        let (a, b) = (
            lo - Vec2::splat(inset + 12.0),
            hi + Vec2::splat(inset + 12.0),
        );
        for side in [-1.0f32, 1.0] {
            let z = if side < 0.0 { a.y } else { b.y };
            d.put_abs(
                &step,
                Transform::from_translation(Vec3::new(
                    f32::midpoint(a.x, b.x),
                    top - height / 2.0,
                    z + side * 6.0,
                ))
                .with_scale(Vec3::new(b.x - a.x + 24.0, height, 12.0)),
            );
            let x = if side < 0.0 { a.x } else { b.x };
            d.put_abs(
                &step,
                Transform::from_translation(Vec3::new(
                    x + side * 6.0,
                    top - height / 2.0,
                    f32::midpoint(a.y, b.y),
                ))
                .with_scale(Vec3::new(12.0, height, b.y - a.y)),
            );
        }
    }
    // Spectators, near-black against the bright hall, with the odd phone light
    let shirts: Vec<Kit> = [(0.05, 0.06, 0.12), (0.10, 0.05, 0.07), (0.04, 0.08, 0.08)]
        .iter()
        .map(|c| d.kit(Capsule3d::new(1.3, 3.0), Color::srgb(c.0, c.1, c.2)))
        .collect();
    let head = d.kit(Sphere::new(1.25), Color::srgb(0.12, 0.09, 0.08));
    let phone = d.glow(Cuboid::new(0.2, 0.9, 0.5), Color::srgb(0.9, 0.95, 1.0), 6.0);
    let gap = if d.high { 9.0 } else { 18.0 };
    for (tier, top) in tops.iter().enumerate() {
        if !d.high && tier == 0 {
            continue;
        }
        let inset = tier as f32 * 12.0 + 6.0;
        let (a, b) = (
            lo - Vec2::splat(inset + 12.0),
            hi + Vec2::splat(inset + 12.0),
        );
        let walk = |from: Vec2, to: Vec2, d: &mut Dress| {
            let len = from.distance(to);
            let steps = (len / gap) as usize;
            for i in 0..steps {
                let jitter = d.rng.random_range(-1.5..1.5);
                let p = from.lerp(to, (i as f32 + 0.5) / steps as f32) + Vec2::splat(jitter);
                let tall = d.rng.random_range(0.9..1.15);
                let kit = d.rng.random_range(0..shirts.len());
                d.put_abs(
                    &shirts[kit],
                    Transform::from_translation(flat(p, top + 3.0 * tall))
                        .with_scale(Vec3::new(1.0, tall, 1.0)),
                );
                d.put_abs(
                    &head,
                    Transform::from_translation(flat(p, top + 6.6 * tall)),
                );
                if d.rng.random_bool(0.04) {
                    d.put_abs(
                        &phone,
                        Transform::from_translation(
                            flat(p, top + 7.0 * tall) + Vec3::new(0.0, 0.0, 1.2),
                        ),
                    );
                }
            }
        };
        walk(Vec2::new(a.x, a.y), Vec2::new(b.x, a.y), d);
        walk(Vec2::new(a.x, b.y), Vec2::new(b.x, b.y), d);
        walk(Vec2::new(a.x, a.y), Vec2::new(a.x, b.y), d);
        walk(Vec2::new(b.x, a.y), Vec2::new(b.x, b.y), d);
    }
}
