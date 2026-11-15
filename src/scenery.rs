//! The room around each track: the table, its floor and ceiling, and the props of each look,
//! all placed by the generator so every random track gets them.

use std::f32::consts::FRAC_PI_2;
use std::f32::consts::PI;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use rand::Rng;
use rand::rngs::ThreadRng;

use crate::prelude::*;
use crate::textures;

mod bar;
mod props;
mod tournament;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn.after(spawn_track));
    app.add_systems(Update, fall.run_if(on_track));
}

/// Height of the ceiling over the table top.
const CEILING: f32 = 110.0;

#[derive(Component, Clone, Debug)]
struct Dust {
    vel: Vec3,
    spin: Vec3,
}

/// Shared handles so hundreds of props cost one mesh and material each.
#[derive(Clone)]
struct Kit {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

/// Where a prop goes: a spot off the road, with the track's direction there.
#[derive(Copy, Clone, Debug)]
struct Place {
    p: Vec2,
    dir: Vec2,
    /// Which side of the road: 1 left, -1 right.
    side: f32,
}

impl Place {
    /// Turns local +X along the track.
    fn yaw(self) -> Quat {
        Quat::from_rotation_y((-self.dir.y).atan2(self.dir.x))
    }

    /// Turns local +Z towards the road.
    fn facing(self) -> Quat {
        self.yaw()
            * if self.side > 0.0 {
                Quat::from_rotation_y(PI)
            } else {
                Quat::IDENTITY
            }
    }
}

/// The colours and textures that make each look what it is.
struct Style {
    /// Cushion face of the walls along the road.
    cushion: Color,
    /// Cap on top of the walls.
    cap: Color,
    /// Table frame, apron and legs.
    wood: Color,
    leather: Color,
    chalk: Color,
    /// How hard chalk marks and diamonds glow.
    glow: f32,
    /// Ceiling tint, painted unlit.
    ceiling: Color,
    floor: Image,
    floor_tile: f32,
    ceiling_image: Image,
}

fn style(scenery: Scenery) -> Style {
    match scenery {
        Scenery::Bar => Style {
            cushion: Color::srgb(0.03, 0.28, 0.12),
            cap: Color::srgb(0.30, 0.15, 0.06),
            wood: Color::srgb(0.28, 0.13, 0.05),
            leather: Color::srgb(0.30, 0.16, 0.08),
            chalk: Color::srgb(0.35, 0.60, 0.95),
            glow: 0.0,
            ceiling: Color::srgb(0.16, 0.09, 0.05),
            floor: textures::planks([0.50, 0.30, 0.14]),
            floor_tile: 26.0,
            ceiling_image: textures::planks([0.5, 0.3, 0.15]),
        },
        Scenery::Tournament => Style {
            cushion: Color::srgb(0.05, 0.17, 0.52),
            cap: Color::srgb(0.08, 0.08, 0.10),
            wood: Color::srgb(0.06, 0.06, 0.08),
            leather: Color::srgb(0.05, 0.05, 0.06),
            chalk: Color::srgb(0.9, 0.95, 1.0),
            glow: 0.0,
            ceiling: Color::srgb(0.80, 0.82, 0.88),
            floor: textures::tiles([0.80, 0.84, 0.92], [0.22, 0.27, 0.40]),
            floor_tile: 44.0,
            ceiling_image: textures::tiles([0.9, 0.92, 0.96], [0.78, 0.8, 0.86]),
        },
        Scenery::Neon => Style {
            cushion: Color::srgb(0.16, 0.05, 0.30),
            cap: Color::srgb(0.05, 0.02, 0.10),
            wood: Color::srgb(0.08, 0.04, 0.12),
            leather: Color::srgb(0.10, 0.04, 0.12),
            chalk: Color::srgb(0.15, 0.95, 0.90),
            glow: 5.0,
            ceiling: Color::srgb(0.05, 0.02, 0.10),
            floor: textures::carpet([0.06, 0.03, 0.12], [0.75, 0.10, 0.65]),
            floor_tile: 22.0,
            ceiling_image: textures::carpet([0.05, 0.03, 0.1], [0.1, 0.5, 0.6]),
        },
    }
}

/// Everything one race's dressing needs: a spawner, shared assets and the dice.
struct Dress<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    track: &'a Track,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    images: &'a mut Assets<Image>,
    rng: ThreadRng,
    high: bool,
    /// Ground already claimed by props: centre and radius.
    taken: Vec<(Vec2, f32)>,
    balls: Vec<Handle<StandardMaterial>>,
    ball_mesh: Handle<Mesh>,
}

impl Dress<'_, '_, '_> {
    fn kit(
        &mut self,
        mesh: impl Into<Mesh>,
        color: Color,
    ) -> Kit {
        self.shiny(mesh, color, 0.8)
    }

    fn shiny(
        &mut self,
        mesh: impl Into<Mesh>,
        color: Color,
        roughness: f32,
    ) -> Kit {
        Kit {
            mesh: self.meshes.add(mesh.into()),
            material: self.materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: roughness,
                ..default()
            }),
        }
    }

    /// A lit-from-within prop; `power` multiplies the colour into the emissive channel.
    fn glow(
        &mut self,
        mesh: impl Into<Mesh>,
        color: Color,
        power: f32,
    ) -> Kit {
        Kit {
            mesh: self.meshes.add(mesh.into()),
            material: self.materials.add(StandardMaterial {
                base_color: color,
                emissive: color.to_linear() * power,
                ..default()
            }),
        }
    }

    /// A see-through prop such as a glass.
    fn glass(
        &mut self,
        mesh: impl Into<Mesh>,
        color: Color,
    ) -> Kit {
        Kit {
            mesh: self.meshes.add(mesh.into()),
            material: self.materials.add(StandardMaterial {
                base_color: color,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.1,
                reflectance: 0.8,
                ..default()
            }),
        }
    }

    /// Whether a spot is clear of the road, shortcuts and other props by `apart`.
    fn free(
        &self,
        p: Vec2,
        apart: f32,
    ) -> bool {
        self.track.locate(p).lateral.abs() > self.track.barrier() + 1.0
            && !self.track.in_shortcut(p)
            && self.taken.iter().all(|(q, r)| q.distance(p) > apart + r)
    }

    /// Keeps props off a stretch of ground.
    fn claim(
        &mut self,
        p: Vec2,
        radius: f32,
    ) {
        self.taken.push((p, radius));
    }

    /// Up to `n` random spots between `near` and `far` metres outside the walls, each with
    /// `apart` metres of its own room.
    fn spots(
        &mut self,
        n: usize,
        near: f32,
        far: f32,
        apart: f32,
    ) -> Vec<Place> {
        let edge = self.track.barrier() + 1.5;
        let mut out = Vec::new();
        for _ in 0..n * 5 {
            if out.len() >= n {
                break;
            }
            let along = self.rng.random_range(0.0..self.track.length);
            let side = if self.rng.random_bool(0.5) { 1.0 } else { -1.0 };
            let p = self
                .track
                .offset(along, side * (edge + self.rng.random_range(near..far)));
            if !self.free(p, apart) {
                continue;
            }
            self.claim(p, apart);
            out.push(Place {
                p,
                dir: self.track.sample(along).1,
                side,
            });
        }
        out
    }

    /// Spawns a prop standing on the ground under its transform's x and z.
    fn put(
        &mut self,
        kit: &Kit,
        mut transform: Transform,
    ) {
        let at = transform.translation;
        transform.translation.y += self.track.ground(Vec2::new(at.x, at.z));
        self.put_abs(kit, transform);
    }

    /// Spawns a prop exactly where its transform says.
    fn put_abs(
        &mut self,
        kit: &Kit,
        transform: Transform,
    ) {
        self.commands.spawn((
            Mesh3d(kit.mesh.clone()),
            MeshMaterial3d(kit.material.clone()),
            transform,
            DespawnOnExit(GameState::Race),
        ));
    }

    fn light(
        &mut self,
        color: Color,
        lumens: f32,
        range: f32,
        at: Vec3,
    ) {
        self.commands.spawn((
            PointLight {
                color,
                intensity: lumens,
                range,
                ..default()
            },
            Transform::from_translation(at),
            DespawnOnExit(GameState::Race),
        ));
    }

    /// A pool ball of `radius`, number 0 being the cue ball, resting on the ground.
    fn ball(
        &mut self,
        number: usize,
        at: Vec2,
        radius: f32,
        spin: Quat,
    ) {
        self.ball_on(number, at, radius, 0.0, spin);
    }

    /// A pool ball resting on something `base` metres off the ground.
    fn ball_on(
        &mut self,
        number: usize,
        at: Vec2,
        radius: f32,
        base: f32,
        spin: Quat,
    ) {
        if self.balls.is_empty() {
            let skins: Vec<_> = (0..16)
                .map(|n| {
                    self.materials.add(StandardMaterial {
                        base_color_texture: Some(self.images.add(textures::ball(n))),
                        perceptual_roughness: 0.18,
                        reflectance: 0.6,
                        ..default()
                    })
                })
                .collect();
            self.balls = skins;
            self.ball_mesh = self.meshes.add(Sphere::new(1.0).mesh().uv(24, 12));
        }
        let kit = Kit {
            mesh: self.ball_mesh.clone(),
            material: self.balls[number % 16].clone(),
        };
        self.put(
            &kit,
            Transform::from_translation(flat(at, radius + base))
                .with_rotation(spin)
                .with_scale(Vec3::splat(radius)),
        );
    }

    /// A random spin, so a field of balls does not all show the same face.
    fn tumble(&mut self) -> Quat {
        Quat::from_euler(
            EulerRot::XYZ,
            self.rng.random_range(0.0..PI * 2.0),
            self.rng.random_range(0.0..PI * 2.0),
            self.rng.random_range(0.0..PI * 2.0),
        )
    }

    /// A decal lying on the road at a spot along it, tilted with the road.
    fn decal(
        &mut self,
        kit: &Kit,
        along: f32,
        lateral: f32,
        yaw: f32,
    ) {
        let t = self.track;
        let at = |a: f32, l: f32| flat(t.offset(a, l), t.surface(a, l));
        let (p, forward, left) = (
            at(along, lateral),
            at(along + 1.0, lateral),
            at(along, lateral + 1.0),
        );
        let up = (left - p).cross(forward - p);
        let up = if up.y < 0.0 { -up } else { up }.normalize_or(Vec3::Y);
        let (_, dir) = t.sample(along);
        self.put_abs(
            kit,
            Transform::from_translation(p + up * 0.07).with_rotation(
                Quat::from_rotation_arc(Vec3::Y, up)
                    * Quat::from_rotation_y((-dir.y).atan2(dir.x) + yaw)
                    * Quat::from_rotation_x(-FRAC_PI_2),
            ),
        );
    }

    /// A decal on the felt off the road.
    fn stain(
        &mut self,
        kit: &Kit,
        at: Vec2,
        yaw: f32,
    ) {
        self.put(
            kit,
            Transform::from_translation(flat(at, 0.07))
                .with_rotation(Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-FRAC_PI_2)),
        );
    }

    /// A flat blended picture: chalk marks, scuffs, spills.
    fn mark(
        &mut self,
        image: Image,
        size: Vec2,
        color: Color,
        glow: f32,
    ) -> Kit {
        Kit {
            mesh: self.meshes.add(Rectangle::new(size.x, size.y)),
            material: self.materials.add(StandardMaterial {
                base_color: color,
                base_color_texture: Some(self.images.add(image)),
                emissive: color.to_linear() * glow,
                alpha_mode: AlphaMode::Blend,
                double_sided: true,
                cull_mode: None,
                perceptual_roughness: 0.5,
                ..default()
            }),
        }
    }

    /// A cue stick: butt, shaft, ferrule and chalked tip, 62 long, standing on `base` and
    /// pointing along `rot`'s +Y.
    fn cue(
        &mut self,
        kits: &[Kit; 4],
        base: Vec3,
        rot: Quat,
    ) {
        for (kit, at) in kits.iter().zip([15.0, 45.0, 60.8, 61.9]) {
            self.put_abs(
                kit,
                Transform::from_translation(base + rot * Vec3::Y * at).with_rotation(rot),
            );
        }
    }

    fn cue_kits(&mut self) -> [Kit; 4] {
        [
            self.shiny(
                ConicalFrustum {
                    radius_top: 0.6,
                    radius_bottom: 0.95,
                    height: 30.0,
                },
                Color::srgb(0.10, 0.06, 0.04),
                0.35,
            ),
            self.shiny(
                ConicalFrustum {
                    radius_top: 0.35,
                    radius_bottom: 0.6,
                    height: 30.0,
                },
                Color::srgb(0.85, 0.68, 0.40),
                0.35,
            ),
            self.kit(Cylinder::new(0.37, 1.6), Color::srgb(0.95, 0.95, 0.9)),
            self.kit(Cylinder::new(0.37, 0.6), Color::srgb(0.25, 0.5, 0.9)),
        ]
    }

    /// A cue lying on the ground with its middle at `at`, butt behind it along `dir`.
    fn lay_cue(
        &mut self,
        kits: &[Kit; 4],
        at: Vec2,
        dir: Vec2,
    ) {
        let rot = Quat::from_rotation_arc(Vec3::Y, Vec3::new(dir.x, 0.0, dir.y).normalize());
        let base = at - dir.normalize() * 31.0;
        let ground = self.track.ground(at);
        self.cue(kits, flat(base, ground + 0.9), rot);
    }
}

/// A mesh of flat-shaded quads, each given as four corners (two along, two across) with the
/// direction its front face should look.
fn solid(parts: &[([Vec3; 4], Vec3, Color)]) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for (quad, hint, color) in parts {
        let base = positions.len() as u32;
        let n = (quad[1] - quad[0]).cross(quad[2] - quad[0]);
        let n = if n.dot(*hint) < 0.0 { -n } else { n }.normalize_or(Vec3::Y);
        for corner in quad {
            positions.push(corner.to_array());
            normals.push(n.to_array());
            colors.push(color.to_linear().to_f32_array());
        }
        indices.extend([base, base + 2, base + 1, base + 1, base + 2, base + 3]);
    }
    let uvs = vec![[0.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

pub(super) fn spawn(
    mut commands: Commands,
    track: Res<Track>,
    options: Res<Options>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut dress = Dress {
        commands: &mut commands,
        track: &track,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        rng: rand::rng(),
        high: options.high_quality,
        taken: Vec::new(),
        balls: Vec::new(),
        ball_mesh: Handle::default(),
    };
    let look = style(track.scenery);
    if let Some(at) = track.landmark {
        dress.claim(at, 26.0);
    }
    room(&mut dress, &look);
    table(&mut dress, &look);
    walls(&mut dress, &look);
    hoops(&mut dress, &look);
    marks(&mut dress, &look);
    match track.scenery {
        Scenery::Bar => bar::dress(&mut dress),
        Scenery::Tournament => tournament::dress(&mut dress),
        Scenery::Neon => {}
    }
    dust(&mut dress);
}

/// The floor far below the table and the ceiling far above it.
fn room(
    d: &mut Dress,
    look: &Style,
) {
    let theme = d.track.theme;
    let size = 3000.0;
    for (image, y, color, flip, unlit, tile) in [
        (
            &look.floor,
            -Track::DROP,
            theme.floor,
            false,
            false,
            look.floor_tile,
        ),
        (&look.ceiling_image, CEILING, look.ceiling, true, true, 60.0),
    ] {
        let handle = d.images.add(image.clone());
        let mut mesh = Plane3d::default().mesh().size(size, size).build();
        let repeat = size / tile;
        if let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
        {
            for uv in uvs {
                *uv = [uv[0] * repeat, uv[1] * repeat];
            }
        }
        let material = d.materials.add(StandardMaterial {
            base_color: color,
            base_color_texture: Some(handle),
            perceptual_roughness: 0.55,
            unlit,
            ..default()
        });
        let mesh = d.meshes.add(mesh);
        d.commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(0.0, y, 0.0).with_rotation(if flip {
                Quat::from_rotation_x(PI)
            } else {
                Quat::IDENTITY
            }),
            DespawnOnExit(GameState::Race),
        ));
    }
}

/// The table's rails and cushions, diamond sights, pockets, apron and legs.
fn table(
    d: &mut Dress,
    look: &Style,
) {
    let (lo, hi) = d.track.table();
    let (mid, half) = ((lo + hi) / 2.0, (hi - lo) / 2.0);
    let (cushion, rail) = (5.0, 14.0);
    let outer = cushion + rail;
    let cushion_kit = d.kit(Cuboid::new(1.0, 1.0, 1.0), look.cushion);
    let rail_kit = d.shiny(Cuboid::new(1.0, 1.0, 1.0), look.wood, 0.4);
    let apron = d.kit(Cuboid::new(1.0, 1.0, 1.0), look.wood);
    let diamond = d.glow(
        Cuboid::new(1.8, 0.25, 1.8),
        Color::srgb(0.95, 0.93, 0.85),
        look.glow * 0.3,
    );
    let pocket = d.kit(Cylinder::new(15.0, 0.1), Color::srgb(0.01, 0.01, 0.01));
    let jaw = d.shiny(Torus::new(13.0, 16.5), look.leather, 0.6);
    // Long sides run along x, short sides along z
    for side in [-1.0f32, 1.0] {
        for (len, centre, along_x) in [
            (
                half.x * 2.0,
                Vec2::new(mid.x, mid.y + side * (half.y + cushion / 2.0)),
                true,
            ),
            (
                half.y * 2.0,
                Vec2::new(mid.x + side * (half.x + cushion / 2.0), mid.y),
                false,
            ),
        ] {
            let turn = if along_x {
                Quat::IDENTITY
            } else {
                Quat::from_rotation_y(FRAC_PI_2)
            };
            let off = if along_x {
                Vec2::new(0.0, side)
            } else {
                Vec2::new(side, 0.0)
            };
            d.put_abs(
                &cushion_kit,
                Transform::from_translation(flat(centre, 2.0))
                    .with_rotation(turn)
                    .with_scale(Vec3::new(len + cushion * 2.0, 4.0, cushion)),
            );
            let rail_at = centre + off * (cushion / 2.0 + rail / 2.0);
            d.put_abs(
                &rail_kit,
                Transform::from_translation(flat(rail_at, 2.5))
                    .with_rotation(turn)
                    .with_scale(Vec3::new(len + outer * 2.0, 5.0, rail)),
            );
            d.put_abs(
                &apron,
                Transform::from_translation(flat(rail_at, -9.0))
                    .with_rotation(turn)
                    .with_scale(Vec3::new(len + outer * 2.0, 24.0, rail - 3.0)),
            );
            // Diamond sights on the rail, one every 40 m
            let fit = (len / 40.0) as i32;
            for k in 0..=fit {
                let t = (k as f32 - fit as f32 / 2.0) * 40.0;
                let at = rail_at
                    + if along_x {
                        Vec2::new(t, 0.0)
                    } else {
                        Vec2::new(0.0, t)
                    };
                d.put_abs(
                    &diamond,
                    Transform::from_translation(flat(at, 5.05))
                        .with_rotation(Quat::from_rotation_y(PI / 4.0)),
                );
            }
        }
    }
    // Legs under the corners
    let leg_h = Track::DROP - 10.0;
    for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let at = mid + Vec2::new(sx * (half.x + outer - 14.0), sz * (half.y + outer - 14.0));
        d.put_abs(
            &apron,
            Transform::from_translation(flat(at, -Track::DROP + leg_h / 2.0 - 0.1))
                .with_scale(Vec3::new(28.0, leg_h, 28.0)),
        );
    }
    // Six pockets: four corners and the middle of both long sides
    let long_x = half.x >= half.y;
    let mut spots = vec![
        mid + Vec2::new(-half.x, -half.y),
        mid + Vec2::new(half.x, -half.y),
        mid + Vec2::new(-half.x, half.y),
        mid + Vec2::new(half.x, half.y),
    ];
    spots.extend(if long_x {
        [mid + Vec2::new(0.0, -half.y), mid + Vec2::new(0.0, half.y)]
    } else {
        [mid + Vec2::new(-half.x, 0.0), mid + Vec2::new(half.x, 0.0)]
    });
    for at in spots {
        d.put_abs(&pocket, Transform::from_translation(flat(at, 0.12)));
        d.put_abs(&jaw, Transform::from_translation(flat(at, 0.3)));
    }
}

/// Cushioned walls with a capped rail along both sides of the road, open at gaps and shortcuts.
fn walls(
    d: &mut Dress,
    look: &Style,
) {
    let t = d.track;
    let (inner, cap_w, height) = (t.barrier() - 0.4, 2.4, 1.9);
    let mut parts = Vec::new();
    let mut studs = Vec::new();
    for i in 0..t.points.len() {
        let (a, b) = (t.dists[i], t.dists[i] + Track::STEP);
        let mid = a + Track::STEP / 2.0;
        if t.gaps.iter().any(|(from, to)| (*from..*to).contains(&mid)) {
            continue;
        }
        for side in [-1.0f32, 1.0] {
            if t.in_shortcut(t.offset(mid, side * inner)) {
                continue;
            }
            let pt = |along: f32, lat: f32, up: f32| {
                flat(t.offset(along, side * lat), t.surface(along, lat) + up)
            };
            let face = [
                pt(a, inner, -0.3),
                pt(a, inner, height),
                pt(b, inner, -0.3),
                pt(b, inner, height),
            ];
            let hint = {
                let (_, dir) = t.sample(mid);
                let toward = -dir.perp() * side;
                Vec3::new(toward.x, 0.0, toward.y)
            };
            parts.push((face, hint, look.cushion));
            let top = [
                pt(a, inner, height),
                pt(a, inner + cap_w, height),
                pt(b, inner, height),
                pt(b, inner + cap_w, height),
            ];
            parts.push((top, Vec3::Y, look.cap));
            let out = [
                pt(a, inner + cap_w, height),
                pt(a, inner + cap_w, -0.3),
                pt(b, inner + cap_w, height),
                pt(b, inner + cap_w, -0.3),
            ];
            parts.push((out, -hint, look.cap));
            if i % 17 == 8 {
                studs.push((pt(mid, inner + cap_w / 2.0, height + 0.05), t.sample(mid).1));
            }
        }
    }
    let mesh = d.meshes.add(solid(&parts));
    let material = d.materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.7,
        cull_mode: None,
        double_sided: true,
        ..default()
    });
    d.commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        DespawnOnExit(GameState::Race),
    ));
    // Diamond sights set into the cap
    let diamond = d.glow(
        Cuboid::new(0.9, 0.12, 0.9),
        Color::srgb(0.95, 0.93, 0.85),
        look.glow * 0.5,
    );
    for (at, dir) in studs {
        d.put_abs(
            &diamond,
            Transform::from_translation(at)
                .with_rotation(Quat::from_rotation_y((-dir.y).atan2(dir.x) + PI / 4.0)),
        );
    }
}

/// Leather pocket jaws arching over the road here and there, like tunnels through a pocket.
fn hoops(
    d: &mut Dress,
    look: &Style,
) {
    let t = d.track;
    let radius = t.half_width + 3.0;
    let jaw = d.shiny(Torus::new(radius - 1.1, radius + 1.1), look.leather, 0.55);
    let mut done: Vec<f32> = Vec::new();
    for _ in 0..40 {
        if done.len() >= 4 {
            break;
        }
        let along = d.rng.random_range(0.0..t.length);
        let flatish = [-8.0, 0.0, 8.0]
            .iter()
            .all(|k| t.grade(along + k).abs() < 0.02 && t.bank(along + k).abs() < 0.02);
        let clear = [-12.0f32, 0.0, 12.0].iter().all(|k| {
            !t.ramps
                .iter()
                .chain(&t.gaps)
                .any(|(from, to)| (*from - 6.0..*to + 6.0).contains(&(along + k)))
        });
        if !flatish || !clear || done.iter().any(|a| (a - along).abs() < t.length / 8.0) {
            continue;
        }
        done.push(along);
        let (p, dir) = t.sample(along);
        let axis = Quat::from_rotation_y((-dir.y).atan2(dir.x)) * Quat::from_rotation_z(FRAC_PI_2);
        let centre = flat(p, t.height(along));
        d.put_abs(
            &jaw,
            Transform::from_translation(centre).with_rotation(axis),
        );
    }
}

/// Chalk marks and scuffs on the road, plus whatever else the look leaves on the cloth.
fn marks(
    d: &mut Dress,
    look: &Style,
) {
    let hw = d.track.half_width;
    let lap = d.track.length;
    let ring = d.mark(
        textures::chalk_ring(),
        Vec2::splat(5.5),
        look.chalk,
        look.glow * 0.3,
    );
    let scuff = d.mark(
        textures::scuff(),
        Vec2::new(9.0, 3.0),
        Color::srgb(0.9, 0.9, 0.95),
        look.glow * 0.1,
    );
    for i in 0..14 {
        let along = d.rng.random_range(0.0..lap);
        if d.track
            .gaps
            .iter()
            .any(|(from, to)| (*from - 4.0..*to + 4.0).contains(&along))
        {
            continue;
        }
        let lateral = d.rng.random_range(-hw * 0.8..hw * 0.8);
        let yaw = d.rng.random_range(0.0..PI);
        if i % 2 == 0 {
            d.decal(&ring, along, lateral, yaw);
        } else {
            let yaw = d.rng.random_range(-0.4..0.4);
            d.decal(&scuff, along, lateral, yaw);
        }
    }
}

/// Chalk dust around the camera.
fn dust(d: &mut Dress) {
    let dust_mesh = d.meshes.add(Rectangle::new(0.25, 0.25));
    let colors = [
        Color::srgb(0.95, 0.96, 1.0),
        Color::srgb(0.55, 0.72, 0.98),
        Color::srgb(0.85, 0.90, 0.98),
        Color::srgb(0.35, 0.55, 0.90),
    ];
    let dust_mats: Vec<_> = colors
        .iter()
        .map(|c| {
            d.materials.add(StandardMaterial {
                base_color: *c,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    let count = if d.high { 260 } else { 90 };
    for i in 0..count {
        let rng = &mut d.rng;
        let mote = Dust {
            vel: Vec3::new(
                rng.random_range(-1.0..1.0),
                -rng.random_range(1.0..2.5),
                rng.random_range(-1.0..1.0),
            ),
            spin: Vec3::new(
                rng.random_range(-3.0..3.0),
                rng.random_range(-3.0..3.0),
                0.0,
            ),
        };
        let at = Vec3::new(
            rng.random_range(-40.0..40.0),
            rng.random_range(0.0..25.0),
            rng.random_range(-40.0..40.0),
        );
        d.commands.spawn((
            mote,
            Mesh3d(dust_mesh.clone()),
            MeshMaterial3d(dust_mats[i % dust_mats.len()].clone()),
            Transform::from_translation(at),
            DespawnOnExit(GameState::Race),
        ));
    }
}

/// Chalk dust drifts down and wraps around the first chase camera.
fn fall(
    time: Res<Time>,
    cameras: Query<(&ChaseCam, &Transform), Without<Dust>>,
    motes: Query<(&Dust, &mut Transform)>,
) {
    let Some((_, camera)) = cameras.iter().find(|(c, _)| c.0 == 0) else {
        return;
    };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let center = camera.translation;
    for (mote, mut transform) in motes {
        let sway = Vec3::new(
            (t * 1.3 + transform.translation.y).sin(),
            0.0,
            (t * 0.9).cos(),
        ) * 0.8;
        transform.translation += (mote.vel + sway) * dt;
        transform.rotate_local_x(mote.spin.x * dt);
        transform.rotate_local_y(mote.spin.y * dt);
        let mut p = transform.translation - center;
        p.x = (p.x + 40.0).rem_euclid(80.0) - 40.0;
        p.z = (p.z + 40.0).rem_euclid(80.0) - 40.0;
        if transform.translation.y < 0.0 {
            p.y = 22.0 - center.y + rng_ish(t, p.x);
        }
        transform.translation = center + p;
    }
}

fn rng_ish(
    t: f32,
    x: f32,
) -> f32 {
    ((t * 7.0 + x * 13.0).sin() * 3.0).abs()
}
