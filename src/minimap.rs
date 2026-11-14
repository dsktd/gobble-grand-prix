//! Minimap per seat: the track drawn once into a texture, north up and never rotating,
//! with only the dots moved each frame (through `UiTransform`, so nothing is laid out again).
//!
//! Everything comes from [`Track`], so procedural tracks get a minimap too. Off-road is
//! shaded with a fixed hatch: the places not to go. No labels, nothing flashes.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::UiTransform;
use bevy::ui::Val2;

use crate::items::Cranberry;
use crate::items::Crate;
use crate::items::Slick;
use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        dots.after(KartSet::Race).run_if(in_state(GameState::Race)),
    );
}

/// The map area inside the 200x200 chip, in spec px.
const AREA: Vec2 = Vec2::new(140.0, 150.0);
/// Pixels per spec px in the texture.
const DETAIL: f32 = 2.0;
/// Empty border around the track inside the area, spec px.
const MARGIN: f32 = 8.0;
/// Pooled dots per seat.
const CRATES: usize = 48;
const SLICKS: usize = 12;
const BERRIES: usize = 12;

/// How world positions land on the map, in spec px from the area's top left.
#[derive(Resource, Copy, Clone, Debug)]
pub struct Map {
    min: Vec2,
    top: f32,
    scale: f32,
    offset: Vec2,
}
impl Map {
    /// Fit the track's bounds into the area, keeping its shape.
    pub fn new(track: &Track) -> Self {
        let mut lo = Vec2::splat(f32::MAX);
        let mut hi = Vec2::splat(f32::MIN);
        for p in &track.points {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let pad = Vec2::splat(track.half_width);
        lo -= pad;
        hi += pad;
        let size = (hi - lo).max(Vec2::splat(1.0));
        let room = AREA - Vec2::splat(MARGIN * 2.0);
        let scale = (room.x / size.x).min(room.y / size.y);
        Self {
            min: lo,
            top: hi.y,
            scale,
            offset: (AREA - size * scale) / 2.0,
        }
    }

    /// A world position (x, z) as spec px in the area, north (-z) up.
    pub fn at(
        &self,
        world: Vec2,
    ) -> Vec2 {
        Vec2::new(
            (world.x - self.min.x) * self.scale + self.offset.x,
            (self.top - world.y) * self.scale + self.offset.y,
        )
    }
}

/// Draw the track once: road, shortcuts, hatched off-road and the start line.
pub fn art(
    track: &Track,
    map: &Map,
) -> Image {
    let (w, h) = ((AREA.x * DETAIL) as usize, (AREA.y * DETAIL) as usize);
    // 0 off-road, 1 road, 2 shortcut
    let mut mask = vec![0u8; w * h];
    let radius = track.half_width * map.scale * DETAIL;
    let mut stamp = |at: Vec2, value: u8| {
        let c = map.at(at) * DETAIL;
        let (x0, x1) = (
            (c.x - radius).floor().max(0.0) as usize,
            (c.x + radius).ceil() as usize,
        );
        let (y0, y1) = (
            (c.y - radius).floor().max(0.0) as usize,
            (c.y + radius).ceil() as usize,
        );
        for y in y0..=y1.min(h - 1) {
            for x in x0..=x1.min(w - 1) {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - c;
                if d.length_squared() <= radius * radius {
                    let cell = &mut mask[y * w + x];
                    *cell = (*cell).max(value);
                }
            }
        }
    };
    for (a, b, r) in &track.shortcuts {
        let steps = ((*b - *a).length() / (r.max(0.5))).ceil().max(1.0) as usize;
        for i in 0..=steps {
            stamp(a.lerp(*b, i as f32 / steps as f32), 2);
        }
    }
    for p in &track.points {
        stamp(*p, 1);
    }

    let shade = |c: Color, a: f32| {
        let c = c.to_srgba();
        [
            (c.red * 255.0) as u8,
            (c.green * 255.0) as u8,
            (c.blue * 255.0) as u8,
            (a * 255.0) as u8,
        ]
    };
    let void_a = shade(Tone::BROWN_900, 0.75);
    let void_b = shade(Tone::BROWN_900, 0.45);
    let road = shade(Tone::CREAM_300, 0.95);
    let cut = shade(Tone::CREAM_300, 0.45);
    let mut data = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let px = match mask[y * w + x] {
                1 => road,
                2 => cut,
                // Fixed diagonal hatch over the voids
                _ if ((x + y) / 6) % 2 == 0 => void_a,
                _ => void_b,
            };
            data[(y * w + x) * 4..][..4].copy_from_slice(&px);
        }
    }

    // Start and finish line across the road at the first point
    if let [a, b, ..] = track.points[..] {
        let dir = (b - a).normalize_or_zero();
        let side = dir.perp();
        let line = shade(Tone::CRANBERRY, 1.0);
        for i in -10..=10 {
            for j in -1..=1 {
                let world =
                    a + side * (track.half_width * i as f32 / 10.0) + dir * (j as f32 * 0.6);
                let c = (map.at(world) * DETAIL).round();
                let (x, y) = (c.x as usize, c.y as usize);
                if x < w && y < h {
                    data[(y * w + x) * 4..][..4].copy_from_slice(&line);
                }
            }
        }
    }

    Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// What a pooled dot stands for.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq)]
enum Kind {
    Crate(usize),
    Slick(usize),
    Berry(usize),
    /// The kart of this racer.
    Kart(usize),
}

#[derive(Component, Copy, Clone, Debug)]
struct Dot {
    seat: usize,
    kind: Kind,
}

/// The dot box: fixed size, moved and scaled with `UiTransform` only.
fn dot(
    k: f32,
    size: f32,
    color: Color,
    ring: f32,
) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: px(size * k),
            height: px(size * k),
            border: UiRect::all(px(ring * k)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(color),
        BorderColor::all(Color::NONE),
        UiTransform::default(),
        ZIndex(0),
        Visibility::Hidden,
    )
}

/// The chip for one seat, at spec (1048, 568) from the viewport's bottom right corner.
pub fn spawn(
    parent: &mut ChildSpawnerCommands,
    seat: usize,
    k: f32,
    art: &Handle<Image>,
) {
    let mut chip = Node {
        position_type: PositionType::Absolute,
        right: px(32.0 * k),
        bottom: px(32.0 * k),
        width: px(200.0 * k),
        height: px(200.0 * k),
        border_radius: BorderRadius::all(px(Space::RADIUS_M * k)),
        ..default()
    };
    chip.overflow = Overflow::clip();
    parent
        .spawn((chip, BackgroundColor(Tone::BROWN_800.with_alpha(0.7))))
        .with_children(|chip| {
            chip.spawn((
                ImageNode::new(art.clone()),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(30.0 * k),
                    top: px(25.0 * k),
                    width: px(AREA.x * k),
                    height: px(AREA.y * k),
                    ..default()
                },
            ))
            .with_children(|area| {
                for i in 0..CRATES {
                    area.spawn((
                        Dot {
                            seat,
                            kind: Kind::Crate(i),
                        },
                        dot(k, 6.0, Tone::PUMPKIN, 0.0),
                    ));
                }
                for i in 0..SLICKS {
                    area.spawn((
                        Dot {
                            seat,
                            kind: Kind::Slick(i),
                        },
                        dot(k, 10.0, Tone::BROWN_700, 0.0),
                    ));
                }
                for i in 0..BERRIES {
                    area.spawn((
                        Dot {
                            seat,
                            kind: Kind::Berry(i),
                        },
                        dot(k, 6.0, Tone::CRANBERRY, 0.0),
                    ));
                }
                for (racer, look) in ROSTER.iter().enumerate() {
                    area.spawn((
                        Dot {
                            seat,
                            kind: Kind::Kart(racer),
                        },
                        dot(k, 16.0, look.kart, 2.0),
                    ));
                }
            });
        });
}

fn dots(
    map: Option<Res<Map>>,
    karts: Query<(&Kart, Option<&Human>)>,
    crates: Query<&Crate>,
    slicks: Query<&Transform, With<Slick>>,
    berries: Query<&Transform, With<Cranberry>>,
    mut dots: Query<(
        &Dot,
        &mut UiTransform,
        &mut Visibility,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut ZIndex,
        &Node,
    )>,
) {
    let Some(map) = map else {
        return;
    };
    let crate_at: Vec<Option<Vec2>> = crates
        .iter()
        .map(|c| (c.hidden <= 0.0).then_some(c.pos))
        .collect();
    let slick_at: Vec<Vec2> = slicks.iter().map(|t| t.translation.xz()).collect();
    let berry_at: Vec<Vec2> = berries.iter().map(|t| t.translation.xz()).collect();
    for (dot, mut at, mut visibility, mut fill, mut ring, mut z, node) in &mut dots {
        let size = match node.width {
            Val::Px(w) => w,
            _ => 0.0,
        };
        // The pool size per seat is spec px, scaled by k: recover k from the box
        let (world, scale, show) = match dot.kind {
            Kind::Crate(i) => match crate_at.get(i) {
                Some(Some(p)) => (*p, 1.0, true),
                _ => (Vec2::ZERO, 1.0, false),
            },
            Kind::Slick(i) => slick_at
                .get(i)
                .map_or((Vec2::ZERO, 1.0, false), |p| (*p, 1.0, true)),
            Kind::Berry(i) => berry_at
                .get(i)
                .map_or((Vec2::ZERO, 1.0, false), |p| (*p, 1.0, true)),
            Kind::Kart(racer) => {
                let Some((kart, human)) = karts.iter().find(|(kart, _)| kart.racer == racer) else {
                    *visibility = Visibility::Hidden;
                    continue;
                };
                let pos = kart.pos;
                let (scale, color, edge, layer) = match human {
                    Some(h) if h.0 == dot.seat => (1.0, Tone::PUMPKIN, Tone::BROWN_900, 3),
                    Some(h) => (0.9, seat_color(h.0), Tone::CREAM_100, 2),
                    None => (0.75, ROSTER[racer].kart, Tone::BROWN_900, 1),
                };
                if fill.0 != color {
                    fill.0 = color;
                }
                *ring = BorderColor::all(edge);
                if z.0 != layer {
                    z.0 = layer;
                }
                (pos, scale, true)
            }
        };
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        if show {
            let p = map.at(world);
            // Centre the dot on the point; the box is `size` px (already scaled by k)
            let k = size / base(dot.kind);
            at.translation = Val2::px(p.x * k - size / 2.0, p.y * k - size / 2.0);
            at.scale = Vec2::splat(scale);
        }
    }
}

/// Spec px size of each kind of dot.
const fn base(kind: Kind) -> f32 {
    match kind {
        Kind::Crate(_) | Kind::Berry(_) => 6.0,
        Kind::Slick(_) => 10.0,
        Kind::Kart(_) => 16.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every point of every track, edges of the road included, lands inside the map area.
    #[test]
    fn tracks_fit_the_map() {
        for index in 0..TRACKS.len() {
            let track = Track::new(index);
            let map = Map::new(&track);
            for p in &track.points {
                for edge in [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y] {
                    let at = map.at(*p + edge * track.half_width);
                    assert!(
                        at.x >= 0.0 && at.y >= 0.0 && at.x <= AREA.x && at.y <= AREA.y,
                        "track {index}: {at:?} leaves the map area"
                    );
                }
            }
        }
    }

    /// The texture is made once and holds road pixels.
    #[test]
    fn art_has_road() {
        let track = Track::new(0);
        let image = art(&track, &Map::new(&track));
        let road = image
            .data
            .as_ref()
            .expect("data")
            .chunks(4)
            .filter(|p| p[3] > 230)
            .count();
        assert!(road > 100);
    }
}
