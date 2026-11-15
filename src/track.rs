//! Track layouts, the road mesh, and lookups along the centerline.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;

use crate::prelude::*;

pub(crate) mod generate;

pub(super) mod prelude {
    pub use super::LOOKS;
    pub use super::Scenery;
    pub use super::Spot;
    pub use super::Theme;
    pub use super::Track;
    pub use super::flat;
    pub use super::spawn_track;
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn_track);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Scenery {
    Farm,
    Table,
    Maze,
}

#[derive(Copy, Clone, Debug)]
pub struct Theme {
    pub sky: Color,
    pub ground: Color,
    pub road: Color,
    pub curb_a: Color,
    pub curb_b: Color,
    pub sun: Color,
    pub sun_lux: f32,
    /// Direction the sunlight travels.
    pub sun_dir: Vec3,
    pub ambient: Color,
    pub ambient_brightness: f32,
    pub fog: Color,
    pub fog_end: f32,
}

/// The looks a random track draws from.
pub struct Look {
    pub scenery: Scenery,
    pub theme: Theme,
}

pub const LOOKS: [Look; 3] = [
    Look {
        scenery: Scenery::Farm,
        theme: Theme {
            sky: Color::srgb(0.98, 0.62, 0.36),
            ground: Color::srgb(0.47, 0.42, 0.18),
            road: Color::srgb(0.55, 0.40, 0.26),
            curb_a: Color::srgb(0.93, 0.85, 0.55),
            curb_b: Color::srgb(0.75, 0.30, 0.10),
            sun: Color::srgb(1.0, 0.72, 0.45),
            sun_lux: 9000.0,
            sun_dir: Vec3::new(0.6, -0.35, 0.7),
            ambient: Color::srgb(1.0, 0.75, 0.6),
            ambient_brightness: 600.0,
            fog: Color::srgb(0.98, 0.66, 0.42),
            fog_end: 320.0,
        },
    },
    Look {
        scenery: Scenery::Table,
        theme: Theme {
            sky: Color::srgb(0.22, 0.08, 0.06),
            ground: Color::srgb(0.45, 0.26, 0.14),
            road: Color::srgb(0.90, 0.84, 0.70),
            curb_a: Color::srgb(0.70, 0.12, 0.10),
            curb_b: Color::srgb(0.95, 0.80, 0.35),
            sun: Color::srgb(1.0, 0.85, 0.6),
            sun_lux: 7000.0,
            sun_dir: Vec3::new(-0.3, -0.8, 0.4),
            ambient: Color::srgb(1.0, 0.8, 0.6),
            ambient_brightness: 700.0,
            fog: Color::srgb(0.22, 0.08, 0.06),
            fog_end: 380.0,
        },
    },
    Look {
        scenery: Scenery::Maze,
        theme: Theme {
            sky: Color::srgb(0.36, 0.30, 0.52),
            ground: Color::srgb(0.30, 0.26, 0.12),
            road: Color::srgb(0.46, 0.34, 0.22),
            curb_a: Color::srgb(0.85, 0.75, 0.35),
            curb_b: Color::srgb(0.55, 0.20, 0.40),
            sun: Color::srgb(0.95, 0.60, 0.65),
            sun_lux: 5000.0,
            sun_dir: Vec3::new(-0.7, -0.3, -0.5),
            ambient: Color::srgb(0.6, 0.6, 1.0),
            ambient_brightness: 700.0,
            fog: Color::srgb(0.36, 0.30, 0.52),
            fog_end: 220.0,
        },
    },
];

/// Where a point sits relative to the centerline.
#[derive(Copy, Clone, Debug)]
pub struct Spot {
    /// Distance along the lap from the start line.
    pub along: f32,
    /// Signed distance from the centerline, positive to the left.
    pub lateral: f32,
}

/// The track being raced, sampled evenly along its centerline.
#[derive(Resource, Clone, Debug)]
pub struct Track {
    pub name: String,
    pub points: Vec<Vec2>,
    pub dists: Vec<f32>,
    pub length: f32,
    pub half_width: f32,
    /// Road height at each centerline point.
    pub heights: Vec<f32>,
    /// Road cross-slope at each centerline point: rise per metre to the left. Corners bank
    /// down towards their inside.
    pub banks: Vec<f32>,
    pub shortcuts: Vec<(Vec2, Vec2, f32)>,
    /// Stretches of road painted as ramps, (from, to) along the lap.
    pub ramps: Vec<(f32, f32)>,
    /// Stretches with no road, only a pit, (from, to) along the lap.
    pub gaps: Vec<(f32, f32)>,
    /// How far the road's sides curl up into a half-pipe at each centerline point, 0..1.
    /// Empty when the track has none.
    pub pipes: Vec<f32>,
    /// Centerline points carried by a bridge over another leg of road. Empty when the
    /// track never crosses itself.
    pub decks: Vec<usize>,
    /// Item box rows as fractions of a lap.
    pub boxes: Vec<f32>,
    pub scenery: Scenery,
    pub theme: Theme,
    /// Open ground for the theme's centrepiece, none when the layout leaves no room.
    pub landmark: Option<Vec2>,
}
impl Track {
    pub const STEP: f32 = 1.5;
    /// Width of the slope from the road shelf down to the flat.
    pub const FALL: f32 = 30.0;
    /// Height of a half-pipe's lip above the road.
    pub const PIPE: f32 = 4.5;
    /// Height a bridge deck clears the road under it by.
    pub const CLEAR: f32 = 6.5;
    /// Depth of a gap's pit below the road.
    pub const PIT: f32 = 7.0;

    /// The track a race setup asks for, generated from its seed.
    pub fn of(setup: &Setup) -> Self {
        generate::track(setup.seed)
    }

    /// Distance from the centerline where the invisible wall stands.
    pub fn barrier(&self) -> f32 {
        self.half_width + 5.0
    }

    /// The nearest road to a track-plane point, where legs that cross count the same.
    pub fn locate(
        &self,
        p: Vec2,
    ) -> Spot {
        self.nearest(p, |_| 0.0)
    }

    /// The nearest road to a point at height `y`: where one leg crosses over another,
    /// the one at the point's level.
    pub fn find(
        &self,
        p: Vec2,
        y: f32,
    ) -> Spot {
        if self.decks.is_empty() {
            return self.locate(p);
        }
        let level = Self::CLEAR / 2.0;
        self.nearest(p, |i| {
            if (self.heights[i] - y).abs() > level {
                1e6
            } else {
                0.0
            }
        })
    }

    /// The nearest road to `p` among segments, each costing `extra` on top of its
    /// squared distance.
    fn nearest(
        &self,
        p: Vec2,
        extra: impl Fn(usize) -> f32,
    ) -> Spot {
        let n = self.points.len();
        let mut best = (f32::MAX, 0.0, 0.0);
        for i in 0..n {
            let a = self.points[i];
            let b = self.points[(i + 1) % n];
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            let proj = a + ab * t;
            let d2 = p.distance_squared(proj) + extra(i);
            if d2 < best.0 {
                let lateral = ab.normalize().perp_dot(p - proj);
                best = (d2, self.dists[i] + t * ab.length(), lateral);
            }
        }
        Spot {
            along: best.1,
            lateral: best.2,
        }
    }

    /// Centerline segment and how far along it a distance along the lap falls.
    fn segment(
        &self,
        along: f32,
    ) -> (usize, usize, f32) {
        let along = along.rem_euclid(self.length);
        let n = self.points.len();
        let i = match self.dists.binary_search_by(|d| d.total_cmp(&along)) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let seg = (self.dists.get(i + 1).copied().unwrap_or(self.length)) - self.dists[i];
        (i, (i + 1) % n, (along - self.dists[i]) / seg)
    }

    /// Centerline point and direction at a distance along the lap.
    pub fn sample(
        &self,
        along: f32,
    ) -> (Vec2, Vec2) {
        let (i, j, t) = self.segment(along);
        let (a, b) = (self.points[i], self.points[j]);
        (a.lerp(b, t), (b - a).normalize())
    }

    /// Road height at a distance along the lap.
    pub fn height(
        &self,
        along: f32,
    ) -> f32 {
        let (i, j, t) = self.segment(along);
        self.heights[i] + (self.heights[j] - self.heights[i]) * t
    }

    /// Cross-slope of the road at a distance along the lap.
    pub fn bank(
        &self,
        along: f32,
    ) -> f32 {
        let (i, j, t) = self.segment(along);
        self.banks[i] + (self.banks[j] - self.banks[i]) * t
    }

    /// Height of the road surface, banking included, out to the walls.
    pub fn surface(
        &self,
        along: f32,
        lateral: f32,
    ) -> f32 {
        let lateral = lateral.clamp(-self.barrier(), self.barrier());
        self.height(along) + lateral * self.bank(along) + self.curl(along, lateral)
    }

    /// Half-pipe rise at a spot: flat down the middle, curving up to the lip at the road's
    /// edges and level with it on the shoulders.
    fn curl(
        &self,
        along: f32,
        lateral: f32,
    ) -> f32 {
        if self.pipes.is_empty() {
            return 0.0;
        }
        let (i, j, t) = self.segment(along);
        let pipe = self.pipes[i] + (self.pipes[j] - self.pipes[i]) * t;
        if pipe == 0.0 {
            return 0.0;
        }
        let flat = self.half_width * 0.4;
        let u = ((lateral.abs() - flat) / (self.half_width - flat)).clamp(0.0, 1.0);
        pipe * u * u * Self::PIPE
    }

    /// Rise per metre to the left across the road surface at a spot.
    pub fn tilt(
        &self,
        along: f32,
        lateral: f32,
    ) -> f32 {
        self.surface(along, lateral + 0.25) - self.surface(along, lateral - 0.25)
    }

    /// Rise per metre of road, positive uphill in the racing direction.
    pub fn grade(
        &self,
        along: f32,
    ) -> f32 {
        (self.height(along + 1.0) - self.height(along - 1.0)) / 2.0
    }

    /// Whether the road ever leaves the flat.
    pub fn hilly(&self) -> bool {
        self.heights
            .iter()
            .chain(&self.banks)
            .chain(&self.pipes)
            .any(|h| *h != 0.0)
    }

    /// Ground height under a spot: the road's out to the walls, then easing down to
    /// the flat over [`Self::FALL`].
    pub fn ground_at(
        &self,
        spot: Spot,
    ) -> f32 {
        if self.gap_at(spot).is_some() {
            return self.height(spot.along) - Self::PIT;
        }
        let past = ((spot.lateral.abs() - self.barrier()) / Self::FALL).clamp(0.0, 1.0);
        let ease = past * past * (3.0 - 2.0 * past);
        self.surface(spot.along, spot.lateral) * (1.0 - ease)
    }

    /// Where the gap a spot is over ends, if it is over one.
    pub fn gap_at(
        &self,
        spot: Spot,
    ) -> Option<f32> {
        if spot.lateral.abs() > self.barrier() {
            return None;
        }
        self.gaps
            .iter()
            .find(|(from, to)| (*from..*to).contains(&spot.along))
            .map(|(_, to)| *to)
    }

    /// Whether a distance along the lap is in one of `stretches`.
    fn within(
        stretches: &[(f32, f32)],
        along: f32,
    ) -> bool {
        stretches
            .iter()
            .any(|(from, to)| (*from..*to).contains(&along))
    }

    /// Ground height under a track-plane point: the earth, never a bridge deck.
    pub fn ground(
        &self,
        p: Vec2,
    ) -> f32 {
        if self.hilly() {
            self.ground_at(self.earth(p))
        } else {
            0.0
        }
    }

    /// The nearest road on the ground to a track-plane point, ignoring bridge decks.
    pub fn earth(
        &self,
        p: Vec2,
    ) -> Spot {
        if self.decks.is_empty() {
            return self.locate(p);
        }
        self.nearest(p, |i| {
            if self.decks.binary_search(&i).is_ok() {
                1e6
            } else {
                0.0
            }
        })
    }

    /// Ground or deck height under a point at about height `y`.
    pub fn ground_near(
        &self,
        p: Vec2,
        y: f32,
    ) -> f32 {
        if self.decks.is_empty() {
            self.ground(p)
        } else {
            self.ground_at(self.find(p, y))
        }
    }

    /// A track-plane point lifted to `y` above the ground.
    pub fn lift(
        &self,
        p: Vec2,
        y: f32,
    ) -> Vec3 {
        flat(p, self.ground(p) + y)
    }

    /// Centerline point shifted sideways, positive to the left.
    pub fn offset(
        &self,
        along: f32,
        lateral: f32,
    ) -> Vec2 {
        let (point, dir) = self.sample(along);
        point + dir.perp() * lateral
    }

    pub fn in_shortcut(
        &self,
        p: Vec2,
    ) -> bool {
        self.shortcuts.iter().any(|(a, b, r)| {
            let ab = *b - *a;
            let t = ((p - *a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            p.distance(*a + ab * t) < *r
        })
    }
}

/// Even samples of a closed Catmull-Rom curve through `control`, with their distances
/// along the lap and the lap length.
fn centerline(control: &[Vec2]) -> (Vec<Vec2>, Vec<f32>, f32) {
    let n = control.len();
    let mut dense = Vec::new();
    for i in 0..n {
        let p0 = control[(i + n - 1) % n];
        let p1 = control[i];
        let p2 = control[(i + 1) % n];
        let p3 = control[(i + 2) % n];
        for s in 0..32 {
            let t = s as f32 / 32.0;
            let t2 = t * t;
            let t3 = t2 * t;
            dense.push(
                0.5 * (2.0 * p1
                    + (p2 - p0) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                    + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3),
            );
        }
    }

    // Resample at an even spacing
    let mut points = vec![dense[0]];
    let mut carry = 0.0;
    for i in 0..dense.len() {
        let a = dense[i];
        let b = dense[(i + 1) % dense.len()];
        let len = a.distance(b);
        let mut t = Track::STEP - carry;
        while t < len {
            points.push(a.lerp(b, t / len));
            t += Track::STEP;
        }
        carry = len - (t - Track::STEP);
    }
    if points.last().unwrap().distance(points[0]) < Track::STEP * 0.5 {
        points.pop();
    }

    let mut dists = Vec::with_capacity(points.len());
    let mut length = 0.0;
    for i in 0..points.len() {
        dists.push(length);
        length += points[i].distance(points[(i + 1) % points.len()]);
    }
    (points, dists, length)
}

/// Lift a track-plane point into the world.
pub const fn flat(
    p: Vec2,
    y: f32,
) -> Vec3 {
    Vec3::new(p.x, y, p.y)
}

/// Build a flat mesh from quads given as four corners each, with one color per quad.
fn quads(
    corners: &[[Vec3; 4]],
    colors: &[Color],
) -> Mesh {
    let mut positions = Vec::new();
    let mut vertex_colors = Vec::new();
    let mut indices = Vec::new();
    for (quad, color) in corners.iter().zip(colors) {
        let base = positions.len() as u32;
        for corner in quad {
            positions.push(corner.to_array());
            vertex_colors.push(color.to_linear().to_f32_array());
        }
        indices.extend([base, base + 2, base + 1, base + 1, base + 2, base + 3]);
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let uvs = vec![[0.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors)
    .with_inserted_indices(Indices::U32(indices))
}

/// A square grid of ground `size` wide following [`Track::ground`], a little below the
/// road so its flat triangles never poke through. Built once per race.
fn terrain(
    track: &Track,
    size: f32,
    cell: f32,
) -> Mesh {
    let cells = (size / cell) as usize;
    let side = cells + 1;
    let half = size / 2.0;
    let mut heights = Vec::with_capacity(side * side);
    for row in 0..side {
        for col in 0..side {
            let p = Vec2::new(col as f32 * cell - half, row as f32 * cell - half);
            heights.push(track.ground(p) - 0.25);
        }
    }
    let at = |row: usize, col: usize| heights[row.min(cells) * side + col.min(cells)];
    let mut positions = Vec::with_capacity(side * side);
    let mut normals = Vec::with_capacity(side * side);
    for row in 0..side {
        for col in 0..side {
            positions.push([
                col as f32 * cell - half,
                at(row, col),
                row as f32 * cell - half,
            ]);
            let dx = at(row, col + 1) - at(row, col.saturating_sub(1));
            let dz = at(row + 1, col) - at(row.saturating_sub(1), col);
            normals.push(Vec3::new(-dx, 2.0 * cell, -dz).normalize().to_array());
        }
    }
    let mut indices = Vec::with_capacity(cells * cells * 6);
    for row in 0..cells {
        for col in 0..cells {
            let a = (row * side + col) as u32;
            let (b, c, d) = (a + 1, a + side as u32, a + side as u32 + 1);
            indices.extend([a, c, b, b, c, d]);
        }
    }
    let uvs = vec![[0.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Quads for a band between two lateral offsets along the whole lap.
fn band(
    track: &Track,
    inner: f32,
    outer: f32,
    y: f32,
) -> Vec<[Vec3; 4]> {
    let n = track.points.len();
    (0..n)
        .filter(|i| !Track::within(&track.gaps, track.dists[*i] + Track::STEP / 2.0))
        .map(|i| {
            let a = track.dists[i];
            let b = a + Track::STEP;
            [
                flat(track.offset(a, inner), track.surface(a, inner) + y),
                flat(track.offset(a, outer), track.surface(a, outer) + y),
                flat(track.offset(b, inner), track.surface(b, inner) + y),
                flat(track.offset(b, outer), track.surface(b, outer) + y),
            ]
        })
        .collect()
}

pub fn spawn_track(
    mut commands: Commands,
    setup: Res<Setup>,
    options: Res<Options>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let track = Track::of(&setup);
    let theme = track.theme;
    let hw = track.half_width;
    let scope = DespawnOnExit(GameState::Race);

    let painted = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });

    // Ground: hills around a hilly road, a plain plane around a flat one
    let ground = if track.hilly() {
        terrain(&track, 900.0, 6.0)
    } else {
        Plane3d::default().mesh().size(900.0, 900.0).into()
    };
    commands.spawn((
        Mesh3d(meshes.add(ground)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: theme.ground,
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.02, 0.0),
        scope.clone(),
    ));

    // Road, slightly darker down the middle for a worn racing line
    // A half-pipe needs strips across the road to curve; a flat road takes one
    let strips = if track.pipes.is_empty() { 1 } else { 10 };
    let road: Vec<[Vec3; 4]> = (0..strips)
        .flat_map(|k| {
            let lat = |k: usize| -hw + 2.0 * hw * k as f32 / strips as f32;
            band(&track, lat(k), lat(k + 1), 0.0)
        })
        .collect();
    let colors: Vec<Color> = (0..road.len())
        .map(|i| {
            let wobble = ((i as f32 * 0.37).sin() * 0.03) + 1.0;
            let c = theme.road.to_srgba();
            Color::srgb(c.red * wobble, c.green * wobble, c.blue * wobble)
        })
        .collect();
    commands.spawn((
        Mesh3d(meshes.add(quads(&road, &colors))),
        MeshMaterial3d(painted.clone()),
        scope.clone(),
    ));

    // Ramps striped like curbs so they read from afar
    let mut stripes = Vec::new();
    let mut colors = Vec::new();
    for (from, to) in &track.ramps {
        let mut a = *from;
        let mut k = 0;
        while a < *to {
            let b = (a + 1.0).min(*to);
            let at = |along: f32, lat: f32| {
                flat(track.offset(along, lat), track.surface(along, lat) + 0.04)
            };
            stripes.push([at(a, -hw), at(a, hw), at(b, -hw), at(b, hw)]);
            colors.push(match k % 2 {
                0 => theme.curb_a,
                _ => theme.curb_b,
            });
            a = b;
            k += 1;
        }
    }
    if !stripes.is_empty() {
        commands.spawn((
            Mesh3d(meshes.add(quads(&stripes, &colors))),
            MeshMaterial3d(painted.clone()),
            scope.clone(),
        ));
    }

    // Bridges: the deck's sides and pillars down to the ground
    if !track.decks.is_empty() {
        let mut sides = Vec::new();
        for &i in &track.decks {
            let a = track.dists[i];
            let b = a + Track::STEP;
            for lat in [-hw - 1.2, hw + 1.2] {
                let top = |along: f32| flat(track.offset(along, lat), track.surface(along, lat));
                let low = |along: f32| top(along) - Vec3::Y * 1.2;
                sides.push([top(a), low(a), top(b), low(b)]);
            }
        }
        let colors = vec![theme.curb_b; sides.len()];
        commands.spawn((
            Mesh3d(meshes.add(quads(&sides, &colors))),
            MeshMaterial3d(painted.clone()),
            scope.clone(),
        ));
        let pillar = meshes.add(Cuboid::new(1.4, 1.0, 1.4));
        let stone = materials.add(StandardMaterial {
            base_color: theme.curb_a,
            perceptual_roughness: 0.9,
            ..default()
        });
        for &i in track.decks.iter().step_by(8) {
            let along = track.dists[i];
            for lat in [-hw, hw] {
                let p = track.offset(along, lat);
                // Not in the middle of the road below
                if track.earth(p).lateral.abs() < track.barrier() {
                    continue;
                }
                let (top, foot) = (track.surface(along, lat) - 1.2, track.ground(p) - 0.5);
                if top - foot < 1.0 {
                    continue;
                }
                commands.spawn((
                    Mesh3d(pillar.clone()),
                    MeshMaterial3d(stone.clone()),
                    Transform::from_translation(flat(p, f32::midpoint(top, foot)))
                        .with_scale(Vec3::new(1.0, top - foot, 1.0)),
                    scope.clone(),
                ));
            }
        }
    }

    // Curbs
    let mut curbs = band(&track, hw, hw + 1.2, 0.03);
    curbs.extend(band(&track, -hw - 1.2, -hw, 0.03));
    let colors: Vec<Color> = (0..curbs.len())
        .map(|i| match (i / 2) % 2 {
            0 => theme.curb_a,
            _ => theme.curb_b,
        })
        .collect();
    commands.spawn((
        Mesh3d(meshes.add(quads(&curbs, &colors))),
        MeshMaterial3d(painted.clone()),
        scope.clone(),
    ));

    // Checkered start line
    let mut checks = Vec::new();
    let mut colors = Vec::new();
    let cells = 8;
    for row in 0..2 {
        for col in 0..cells {
            let lat0 = -hw + col as f32 * 2.0 * hw / cells as f32;
            let lat1 = lat0 + 2.0 * hw / cells as f32;
            let a = row as f32 * 1.2;
            let b = a + 1.2;
            let at = |along: f32, lat: f32| {
                flat(track.offset(along, lat), track.surface(along, lat) + 0.05)
            };
            checks.push([at(a, lat0), at(a, lat1), at(b, lat0), at(b, lat1)]);
            colors.push(match (row + col) % 2 {
                0 => Color::WHITE,
                _ => Color::srgb(0.1, 0.1, 0.1),
            });
        }
    }
    commands.spawn((
        Mesh3d(meshes.add(quads(&checks, &colors))),
        MeshMaterial3d(painted),
        scope.clone(),
    ));

    // Pocket tunnels
    for (a, b, r) in &track.shortcuts {
        let ab = *b - *a;
        let mid = (*a + *b) / 2.0;
        let angle = ab.y.atan2(ab.x);
        commands.spawn((
            Mesh3d(meshes.add(Capsule2d::new(*r, ab.length()))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.03, 0.03, 0.04),
                perceptual_roughness: 0.15,
                reflectance: 0.7,
                ..default()
            })),
            Transform::from_translation(flat(mid, 0.04)).with_rotation(
                Quat::from_rotation_y(-angle)
                    * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                    * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            ),
            scope.clone(),
        ));
    }

    commands.insert_resource(ClearColor(theme.sky));
    commands.insert_resource(GlobalAmbientLight {
        color: theme.ambient,
        brightness: theme.ambient_brightness,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            color: theme.sun,
            illuminance: theme.sun_lux,
            shadow_maps_enabled: options.high_quality,
            ..default()
        },
        Transform::default().looking_to(theme.sun_dir, Vec3::Y),
        scope,
    ));
    commands.insert_resource(track);
}
