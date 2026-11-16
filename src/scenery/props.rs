//! Props every look shares: balls, cues, chalk, racks and stools, each in the look's colours.

use std::f32::consts::FRAC_PI_2;
use std::f32::consts::PI;

use bevy::prelude::*;
use rand::Rng;

use super::Dress;
use crate::prelude::*;

/// A rack of fifteen balls of radius `r` in a triangle with its frame, centred on `at`.
pub(super) fn rack(
    d: &mut Dress,
    at: Vec2,
    r: f32,
    frame: Color,
) {
    let m = r + 0.4;
    let turn = Quat::from_rotation_y(d.rng.random_range(0.0..PI * 2.0));
    let wood = d.shiny(Cuboid::new(1.0, 1.0, 1.0), frame, 0.4);
    // Fifteen balls, the eight in the middle of row three
    let mut numbers: Vec<usize> = (1..16).filter(|n| *n != 8).collect();
    let spot = |row: usize, k: usize| (row * (row + 1) / 2 + k, row, k);
    for row in 0..5 {
        for k in 0..=row {
            let (index, row, k) = spot(row, k);
            let number = if index == 7 {
                8
            } else if numbers.is_empty() {
                0
            } else {
                let pick = d.rng.random_range(0..numbers.len());
                numbers.swap_remove(pick)
            };
            let local = Vec3::new(
                row as f32 * 3.0f32.sqrt() * r,
                0.0,
                (k as f32 - row as f32 / 2.0) * 2.0 * r,
            );
            let p = at + (turn * local).xz();
            let spin = d.tumble();
            d.ball(number, p, r, spin);
        }
    }
    // The frame: two slanted sides and the base
    let base = 4.0 * 3.0f32.sqrt() * r + m;
    let across = 4.0 * r + 3.0f32.sqrt() * m;
    let side = across * 2.0;
    for sign in [-1.0f32, 1.0] {
        let mid = Vec2::new(f32::midpoint(-2.0 * m, base), sign * across / 2.0);
        let p = at + (turn * Vec3::new(mid.x, 0.0, mid.y)).xz();
        let dir = Vec2::new(3.0f32.sqrt(), -sign).normalize();
        d.put(
            &wood,
            Transform::from_translation(flat(p, 1.3))
                .with_rotation(turn * Quat::from_rotation_y((-dir.y).atan2(dir.x)))
                .with_scale(Vec3::new(side, 2.6, 0.9)),
        );
    }
    let p = at + (turn * Vec3::new(base, 0.0, 0.0)).xz();
    d.put(
        &wood,
        Transform::from_translation(flat(p, 1.3))
            .with_rotation(turn * Quat::from_rotation_y(FRAC_PI_2))
            .with_scale(Vec3::new(across * 2.0, 2.6, 0.9)),
    );
}

pub(super) fn balls(
    d: &mut Dress,
    n: usize,
) {
    for place in d.spots(n, 6.0, 52.0, 6.0) {
        let (number, r) = (d.rng.random_range(0..16), d.rng.random_range(2.4..4.2));
        let spin = d.tumble();
        d.ball(number, place.p, r, spin);
    }
}

/// Cues lying about and stood in racks.
pub(super) fn cues(
    d: &mut Dress,
    lying: usize,
    racks: usize,
    wood: Color,
) {
    let kits = d.cue_kits();
    for place in d.spots(lying, 5.0, 40.0, 10.0) {
        let jitter = Vec2::from_angle(d.rng.random_range(-0.5..0.5)).rotate(place.dir);
        d.lay_cue(&kits, place.p, jitter);
    }
    let frame = d.shiny(Cuboid::new(1.0, 1.0, 1.0), wood, 0.4);
    for place in d.spots(racks, 8.0, 30.0, 16.0) {
        let face = place.facing();
        let local = |x: f32, y: f32, z: f32| {
            flat(place.p, 0.0) + face * Vec3::new(x, y, z) + Vec3::Y * y * 0.0
        };
        // Back board, base and top rail; the cues lean on the board
        d.put(
            &frame,
            Transform::from_translation(local(0.0, 22.0, -2.2))
                .with_rotation(face)
                .with_scale(Vec3::new(26.0, 44.0, 0.8)),
        );
        d.put(
            &frame,
            Transform::from_translation(local(0.0, 1.0, 0.0))
                .with_rotation(face)
                .with_scale(Vec3::new(28.0, 2.0, 7.0)),
        );
        d.put(
            &frame,
            Transform::from_translation(local(0.0, 40.0, -1.0))
                .with_rotation(face)
                .with_scale(Vec3::new(27.0, 1.2, 2.4)),
        );
        for k in 0..6 {
            let base = local((k as f32 - 2.5) * 3.8, 2.0, 0.6);
            let lean = Quat::from_rotation_x(0.05)
                * Quat::from_rotation_z(d.rng.random_range(-0.03..0.03));
            let at = Vec2::new(base.x, base.z);
            let y = d.track.ground(at) + 2.0;
            d.cue(&kits, flat(at, y), face * lean);
        }
    }
}

/// Big blue chalk cubes.
pub(super) fn chalk(
    d: &mut Dress,
    n: usize,
) {
    let cube = d.kit(Cuboid::new(5.0, 5.0, 5.0), Color::srgb(0.18, 0.42, 0.82));
    let pit = d.kit(Cuboid::new(3.6, 0.3, 3.6), Color::srgb(0.07, 0.16, 0.42));
    let dust = d.kit(Cuboid::new(5.2, 0.15, 5.2), Color::srgb(0.75, 0.85, 0.98));
    for place in d.spots(n, 5.0, 44.0, 5.0) {
        let yaw = Quat::from_rotation_y(d.rng.random_range(0.0..PI * 2.0));
        d.put(
            &cube,
            Transform::from_translation(flat(place.p, 2.5)).with_rotation(yaw),
        );
        d.put(
            &pit,
            Transform::from_translation(flat(place.p, 5.0)).with_rotation(yaw),
        );
        d.put(
            &dust,
            Transform::from_translation(flat(place.p, 0.08)).with_rotation(yaw),
        );
    }
}

pub(super) fn stools(
    d: &mut Dress,
    n: usize,
    seat: Color,
    metal: Color,
) {
    let leather = d.shiny(Cylinder::new(4.5, 2.0), seat, 0.35);
    let post = d.shiny(Cylinder::new(0.7, 12.0), metal, 0.2);
    let ring = d.shiny(Torus::new(2.7, 3.3), metal, 0.2);
    let foot = d.shiny(Cylinder::new(4.2, 0.6), Color::srgb(0.55, 0.55, 0.6), 0.3);
    for place in d.spots(n, 6.0, 46.0, 7.0) {
        let tip = d.rng.random_bool(0.15);
        // Most stand; a few lie knocked over
        let turn = if tip {
            Quat::from_rotation_y(d.rng.random_range(0.0..PI * 2.0))
                * Quat::from_rotation_z(FRAC_PI_2)
        } else {
            Quat::IDENTITY
        };
        let lift = if tip { 4.5 } else { 0.0 };
        for (kit, y) in [(&leather, 14.0), (&post, 7.0), (&ring, 6.0), (&foot, 0.3)] {
            let local = turn * Vec3::Y * (y + 0.0);
            d.put(
                kit,
                Transform::from_translation(flat(place.p, lift) + local).with_rotation(turn),
            );
        }
    }
}
