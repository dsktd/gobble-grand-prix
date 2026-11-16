//! Procedural textures. Everything is painted from maths at load, so there are no image files
//! to license or ship.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageAddressMode;
use bevy::image::ImageSampler;
use bevy::image::ImageSamplerDescriptor;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;

/// A pseudo-random number in 0..1 from an integer lattice point.
fn hash(
    x: i32,
    y: i32,
    salt: u32,
) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x85eb_ca6b)
        ^ (y as u32).wrapping_mul(0xc2b2_ae35)
        ^ salt.wrapping_mul(0x27d4_eb2f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0
}

/// Smooth value noise that wraps every `period` cells, so textures tile.
fn noise(
    u: f32,
    v: f32,
    period: i32,
    salt: u32,
) -> f32 {
    let (x, y) = (u * period as f32, v * period as f32);
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - ix as f32, y - iy as f32);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let at = |dx: i32, dy: i32| {
        hash(
            (ix + dx).rem_euclid(period),
            (iy + dy).rem_euclid(period),
            salt,
        )
    };
    let top = at(0, 0) + (at(1, 0) - at(0, 0)) * sx;
    let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * sx;
    top + (bottom - top) * sy
}

/// Paints `f(u, v)` (each 0..1) as RGBA into a `w` by `h` image, repeating when asked.
fn paint(
    w: u32,
    h: u32,
    repeat: bool,
    f: impl Fn(f32, f32) -> [f32; 4],
) -> Image {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let c = f((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            data.extend(c.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8));
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let mut sampler = ImageSamplerDescriptor::linear();
    if repeat {
        sampler.set_address_mode(ImageAddressMode::Repeat);
    }
    image.sampler = ImageSampler::Descriptor(sampler);
    image
}

fn mix(
    a: [f32; 3],
    b: [f32; 3],
    t: f32,
) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale(
    c: [f32; 3],
    k: f32,
) -> [f32; 3] {
    c.map(|v| v * k)
}

/// Felt nap: near-white so it can tint any colour, with soft wear patches and fine grain.
pub fn felt() -> Image {
    paint(128, 128, true, |u, v| {
        let patch = noise(u, v, 4, 1) * 0.07 + noise(u, v, 16, 2) * 0.03;
        let grain = hash((u * 128.0) as i32, (v * 128.0) as i32, 3) * 0.04;
        let k = 0.9 + patch + grain - 0.05;
        [k, k, k, 1.0]
    })
}

/// Wooden floorboards, four planks per tile.
pub fn planks(base: [f32; 3]) -> Image {
    paint(128, 128, true, |u, v| {
        let row = (v * 4.0).floor() as i32;
        let across = (v * 4.0).fract();
        let seam = hash(row, 0, 5);
        let end = f32::from(u8::from((u + seam).fract() < 0.012));
        let tone = 0.75 + hash(row, (u + seam).floor() as i32, 6) * 0.5;
        let grain = noise(u * 2.0, v * 40.0 / 4.0, 8, 7) * 0.25 + noise(u, v, 32, 8) * 0.1;
        let edge = if !(0.06..=0.94).contains(&across) || end > 0.0 {
            0.45
        } else {
            1.0
        };
        let c = scale(base, (tone + grain - 0.15) * edge);
        [c[0], c[1], c[2], 1.0]
    })
}

/// Big polished tiles in a checker, with grout.
pub fn tiles(
    a: [f32; 3],
    b: [f32; 3],
) -> Image {
    paint(128, 128, true, |u, v| {
        let (cx, cy) = ((u * 2.0).floor() as i32, (v * 2.0).floor() as i32);
        let (fx, fy) = ((u * 2.0).fract(), (v * 2.0).fract());
        let grout = !(0.025..=0.975).contains(&fx) || !(0.025..=0.975).contains(&fy);
        let base = if (cx + cy) % 2 == 0 { a } else { b };
        let sheen = noise(u, v, 8, 9) * 0.12 + 0.94;
        let c = if grout {
            scale(base, 0.45)
        } else {
            scale(base, sheen)
        };
        [c[0], c[1], c[2], 1.0]
    })
}

/// Patterned carpet: fine pile noise and a diamond lattice in `accent`.
pub fn carpet(
    base: [f32; 3],
    accent: [f32; 3],
) -> Image {
    paint(128, 128, true, |u, v| {
        let d = ((u * 4.0).fract() - 0.5).abs() + ((v * 4.0).fract() - 0.5).abs();
        let line = (1.0 - (d - 0.38).abs() * 18.0).clamp(0.0, 1.0);
        let pile = 0.85 + hash((u * 128.0) as i32, (v * 128.0) as i32, 10) * 0.2;
        let c = scale(mix(base, accent, line * 0.8), pile);
        [c[0], c[1], c[2], 1.0]
    })
}

/// Chalk ring: a smudged circle of chalk dust, white on transparent.
pub fn chalk_ring() -> Image {
    paint(128, 128, false, |u, v| {
        let (x, y) = (u - 0.5, v - 0.5);
        let r = x.hypot(y) * 2.0;
        let wobble = noise(u, v, 8, 11) * 0.12;
        let ring = (1.0 - ((r - 0.62 - wobble * 0.5).abs() * 14.0)).clamp(0.0, 1.0);
        let dust = if r < 0.7 {
            (noise(u, v, 16, 12) - 0.55).max(0.0) * 0.8
        } else {
            0.0
        };
        let speck = hash((u * 128.0) as i32, (v * 128.0) as i32, 13);
        let a = (ring * (0.55 + 0.45 * speck) + dust) * (1.0 - r * 0.3);
        [1.0, 1.0, 1.0, if r < 1.0 { a * 0.8 } else { 0.0 }]
    })
}

/// A scuff: a streak of marks left by a skidding ball.
pub fn scuff() -> Image {
    paint(128, 64, false, |u, v| {
        let y = (v - 0.5).abs() * 2.0;
        let along = (u - 0.5).abs() * 2.0;
        let streak = noise(u * 2.0, v * 6.0, 6, 14);
        let a = (1.0 - y * 1.6).clamp(0.0, 1.0) * (1.0 - along * along) * (streak * 1.4 - 0.3);
        [1.0, 1.0, 1.0, a.clamp(0.0, 0.7)]
    })
}

/// A soft blob with a brighter rim, for spilled drinks.
pub fn puddle() -> Image {
    paint(128, 128, false, |u, v| {
        let (x, y) = (u - 0.5, v - 0.5);
        let angle = y.atan2(x);
        let r = x.hypot(y) * 2.0;
        let edge = 0.7 + 0.2 * (angle * 3.0).sin() * (angle * 2.0 + 1.0).cos();
        let a = ((edge - r) * 6.0).clamp(0.0, 1.0);
        let rim = (1.0 - ((edge - r).abs() * 12.0)).clamp(0.0, 1.0) * 0.4;
        let k = 0.8 + rim;
        [k, k, k, a * 0.65]
    })
}

const COLORS: [[f32; 3]; 8] = [
    [0.95, 0.80, 0.10],
    [0.10, 0.25, 0.75],
    [0.85, 0.12, 0.10],
    [0.40, 0.12, 0.55],
    [0.95, 0.45, 0.08],
    [0.08, 0.50, 0.20],
    [0.50, 0.10, 0.12],
    [0.02, 0.02, 0.03],
];

/// Three-by-five bitmaps of the digits 0 to 9, one bit per pixel, top row first.
const DIGITS: [u16; 10] = [
    0b111_101_101_101_111,
    0b010_110_010_010_111,
    0b111_001_111_100_111,
    0b111_001_111_001_111,
    0b101_101_111_001_001,
    0b111_100_111_001_111,
    0b111_100_111_101_111,
    0b111_001_001_001_001,
    0b111_101_111_101_111,
    0b111_101_111_001_111,
];

/// A pool ball's skin: number 0 is the cue ball, 1 to 8 solid, 9 to 15 striped.
pub fn ball(number: usize) -> Image {
    let color = if number == 0 {
        [0.95, 0.95, 0.92]
    } else {
        COLORS[(number - 1) % 8]
    };
    let stripe = number > 8;
    let digits: Vec<usize> = if number == 0 {
        Vec::new()
    } else if number > 9 {
        vec![number / 10, number % 10]
    } else {
        vec![number]
    };
    paint(256, 128, false, move |u, v| {
        let (px, py) = (u * 256.0, v * 128.0);
        let mut c = if stripe && !(0.27..=0.73).contains(&v) {
            [0.96, 0.96, 0.94]
        } else {
            color
        };
        // Two number discs on opposite sides
        for cx in [64.0, 192.0] {
            let (dx, dy) = (px - cx, py - 64.0);
            if number > 0 && dx * dx + dy * dy < 26.0 * 26.0 {
                c = [0.97, 0.97, 0.95];
                let size = if digits.len() > 1 { 3.0 } else { 4.0 };
                let width = digits.len() as f32 * 4.0 * size;
                let (lx, ly) = (dx + width / 2.0, dy + 2.5 * size);
                if lx >= 0.0 && ly >= 0.0 {
                    let (col, row) = ((lx / size) as usize, (ly / size) as usize);
                    let (which, col) = (col / 4, col % 4);
                    if which < digits.len() && col < 3 && row < 5 {
                        let bit = 14 - (row * 3 + col);
                        if DIGITS[digits[which]] >> bit & 1 == 1 {
                            c = [0.05, 0.05, 0.06];
                        }
                    }
                }
            } else if number == 0 && dx * dx + dy * dy < 5.0 * 5.0 {
                c = [0.2, 0.2, 0.25];
            }
        }
        [c[0], c[1], c[2], 1.0]
    })
}
