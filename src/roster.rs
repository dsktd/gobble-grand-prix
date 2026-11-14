//! The Thanksgiving racers and the primitive meshes they are built from.

use bevy::prelude::*;

pub(super) mod prelude {
    pub use super::Body;
    pub use super::Parts;
    pub use super::ROSTER;
    pub use super::Stats;
    pub use super::spawn_model;
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, wail);
}

/// Open and shut the cat's mouth in a long dramatic sob.
fn wail(
    time: Res<Time<Real>>,
    mouths: Query<&mut Transform, With<Mouth>>,
) {
    let t = time.elapsed_secs();
    let open = 0.35 + 0.65 * ((t * 2.2).sin() * 0.5 + 0.5).powf(0.6);
    for mut transform in mouths {
        transform.scale.y = open;
    }
}

/// Handling numbers for a racer. All are multipliers around 1.0.
#[derive(Copy, Clone, Debug)]
pub struct Stats {
    pub top_speed: f32,
    pub accel: f32,
    pub turn: f32,
    pub weight: f32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Body {
    Turkey,
    Pie,
    Can,
    Corn,
    Potato,
    Casserole,
    Cat,
}

#[derive(Copy, Clone, Debug)]
pub struct Racer {
    pub name: &'static str,
    pub blurb: &'static str,
    pub kart: Color,
    pub accent: Color,
    pub body: Body,
    pub stats: Stats,
}

pub const ROSTER: [Racer; 7] = [
    Racer {
        name: "Tom Gobbler",
        blurb: "A turkey in a gravy boat. Balanced, and loud about it.",
        kart: Color::srgb(0.93, 0.89, 0.80),
        accent: Color::srgb(0.45, 0.25, 0.12),
        body: Body::Turkey,
        stats: Stats {
            top_speed: 1.0,
            accel: 1.0,
            turn: 1.0,
            weight: 1.0,
        },
    },
    Racer {
        name: "Patty Crust",
        blurb: "A pumpkin pie on wheels. Quick off the line, light on the corners.",
        kart: Color::srgb(0.85, 0.47, 0.12),
        accent: Color::srgb(0.87, 0.70, 0.42),
        body: Body::Pie,
        stats: Stats {
            top_speed: 0.96,
            accel: 1.25,
            turn: 1.05,
            weight: 0.8,
        },
    },
    Racer {
        name: "Cranston Can",
        blurb: "Jellied cranberry, ridges and all. Heavy and fast once rolling.",
        kart: Color::srgb(0.55, 0.05, 0.12),
        accent: Color::srgb(0.75, 0.75, 0.78),
        body: Body::Can,
        stats: Stats {
            top_speed: 1.07,
            accel: 0.85,
            turn: 0.9,
            weight: 1.4,
        },
    },
    Racer {
        name: "Kernel Cob",
        blurb: "A corn cob with husk wings. Turns on a kernel.",
        kart: Color::srgb(0.96, 0.80, 0.20),
        accent: Color::srgb(0.38, 0.55, 0.20),
        body: Body::Corn,
        stats: Stats {
            top_speed: 0.98,
            accel: 1.05,
            turn: 1.25,
            weight: 0.9,
        },
    },
    Racer {
        name: "Elder Spud",
        blurb: "A potato in a pilgrim hat. Steady, sturdy, buckled up.",
        kart: Color::srgb(0.62, 0.45, 0.28),
        accent: Color::srgb(0.08, 0.08, 0.08),
        body: Body::Potato,
        stats: Stats {
            top_speed: 1.02,
            accel: 0.95,
            turn: 0.95,
            weight: 1.2,
        },
    },
    Racer {
        name: "Marsha Mallow",
        blurb: "Sweet potato casserole, toasted on top. Drifts like a dream.",
        kart: Color::srgb(0.88, 0.42, 0.18),
        accent: Color::srgb(0.98, 0.95, 0.88),
        body: Body::Casserole,
        stats: Stats {
            top_speed: 1.0,
            accel: 1.1,
            turn: 1.12,
            weight: 0.95,
        },
    },
    Racer {
        name: "Wailing Whiskers",
        blurb: "Was told there is no turkey for cats. Has not stopped crying since.",
        kart: Color::srgb(0.88, 0.88, 0.90),
        accent: Color::srgb(0.48, 0.48, 0.52),
        body: Body::Cat,
        stats: Stats {
            top_speed: 0.93,
            accel: 1.3,
            turn: 1.1,
            weight: 0.7,
        },
    },
];

/// The cat's jaw, opened and shut by [`wail`].
#[derive(Component, Copy, Clone, Debug)]
pub struct Mouth;

/// Shared asset stores for building primitive models.
pub struct Parts<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
}
impl Parts<'_> {
    pub fn mat(
        &mut self,
        color: Color,
    ) -> Handle<StandardMaterial> {
        self.materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.7,
            ..default()
        })
    }

    pub fn glow(
        &mut self,
        color: Color,
        strength: f32,
    ) -> Handle<StandardMaterial> {
        self.materials.add(StandardMaterial {
            base_color: color,
            emissive: color.to_linear() * strength,
            ..default()
        })
    }

    pub fn part(
        &mut self,
        mesh: impl Into<Mesh>,
        color: Color,
        transform: Transform,
    ) -> (Mesh3d, MeshMaterial3d<StandardMaterial>, Transform) {
        let mesh = self.meshes.add(mesh.into());
        let material = self.mat(color);
        (Mesh3d(mesh), MeshMaterial3d(material), transform)
    }
}

const fn at(
    x: f32,
    y: f32,
    z: f32,
) -> Transform {
    Transform::from_xyz(x, y, z)
}

/// Spawn a racer model facing -Z as children of `root`.
pub fn spawn_model(
    commands: &mut Commands,
    parts: &mut Parts,
    root: Entity,
    racer: &Racer,
) {
    let mut pieces = Vec::new();
    let tire = Color::srgb(0.12, 0.10, 0.09);
    let hub = Color::srgb(0.85, 0.65, 0.25);

    // Wheels, shared by every kart
    for (x, z) in [(-0.75, -0.75), (0.75, -0.75), (-0.8, 0.8), (0.8, 0.8)] {
        let wheel = Transform::from_xyz(x, 0.32, z)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        pieces.push(parts.part(Cylinder::new(0.32, 0.28), tire, wheel));
        pieces.push(parts.part(
            Cylinder::new(0.14, 0.3),
            hub,
            wheel.with_scale(Vec3::new(1.0, 1.02, 1.0)),
        ));
    }

    match racer.body {
        Body::Turkey => {
            // Gravy boat hull with a spout at the front
            pieces.push(parts.part(
                Sphere::new(1.0),
                racer.kart,
                at(0.0, 0.55, 0.0).with_scale(Vec3::new(0.85, 0.35, 1.25)),
            ));
            pieces.push(parts.part(
                Cylinder::new(0.8, 0.08),
                Color::srgb(0.45, 0.25, 0.10),
                at(0.0, 0.78, 0.0).with_scale(Vec3::new(0.95, 1.0, 1.4)),
            ));
            pieces.push(parts.part(
                Cone::new(0.25, 0.6),
                racer.kart,
                at(0.0, 0.75, -1.3).with_rotation(Quat::from_rotation_x(-1.2)),
            ));
            // Turkey
            pieces.push(parts.part(Sphere::new(0.6), racer.accent, at(0.0, 1.25, 0.15)));
            pieces.push(parts.part(
                Capsule3d::new(0.12, 0.5),
                racer.accent,
                at(0.0, 1.8, -0.25).with_rotation(Quat::from_rotation_x(-0.3)),
            ));
            pieces.push(parts.part(
                Sphere::new(0.22),
                Color::srgb(0.55, 0.35, 0.22),
                at(0.0, 2.15, -0.35),
            ));
            pieces.push(parts.part(
                Cone::new(0.07, 0.22),
                Color::srgb(0.95, 0.70, 0.10),
                at(0.0, 2.12, -0.62).with_rotation(Quat::from_rotation_x(-1.57)),
            ));
            pieces.push(parts.part(
                Sphere::new(0.09),
                Color::srgb(0.85, 0.08, 0.08),
                at(0.0, 1.98, -0.52).with_scale(Vec3::new(0.8, 1.6, 0.8)),
            ));
            for eye in [-0.1, 0.1] {
                pieces.push(parts.part(Sphere::new(0.045), Color::BLACK, at(eye, 2.22, -0.53)));
            }
            // Tail fan
            let feathers = [
                Color::srgb(0.70, 0.20, 0.08),
                Color::srgb(0.90, 0.55, 0.10),
                Color::srgb(0.55, 0.30, 0.12),
                Color::srgb(0.95, 0.75, 0.25),
            ];
            for i in 0..9 {
                let angle = -1.1 + i as f32 * 0.275;
                let feather = Transform::from_xyz(0.0, 1.4, 0.7)
                    .with_rotation(Quat::from_rotation_z(angle) * Quat::from_rotation_x(0.35))
                    * Transform::from_xyz(0.0, 0.75, 0.0).with_scale(Vec3::new(1.0, 1.0, 0.25));
                pieces.push(parts.part(Capsule3d::new(0.2, 0.9), feathers[i % 4], feather));
            }
        }
        Body::Pie => {
            pieces.push(parts.part(Cylinder::new(1.05, 0.45), racer.accent, at(0.0, 0.75, 0.0)));
            pieces.push(parts.part(Cylinder::new(0.92, 0.47), racer.kart, at(0.0, 0.77, 0.0)));
            pieces.push(parts.part(Torus::new(0.9, 1.12), racer.accent, at(0.0, 1.0, 0.0)));
            // Whipped cream swirl driver
            pieces.push(parts.part(
                Sphere::new(0.45),
                Color::srgb(0.99, 0.97, 0.92),
                at(0.0, 1.3, 0.1),
            ));
            pieces.push(parts.part(
                Cone::new(0.3, 0.6),
                Color::srgb(0.99, 0.97, 0.92),
                at(0.0, 1.85, 0.1),
            ));
            for eye in [-0.15, 0.15] {
                pieces.push(parts.part(Sphere::new(0.06), Color::BLACK, at(eye, 1.45, -0.33)));
            }
        }
        Body::Can => {
            pieces.push(parts.part(
                Cuboid::new(1.5, 0.3, 2.2),
                Color::srgb(0.3, 0.3, 0.32),
                at(0.0, 0.5, 0.0),
            ));
            let gel = Transform::from_xyz(0.0, 1.05, 0.1)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
            pieces.push(parts.part(Cylinder::new(0.6, 1.6), racer.kart, gel));
            for i in 0..4 {
                let z = -0.55 + i as f32 * 0.37;
                let ridge = Transform::from_xyz(0.0, 1.05, 0.1 + z)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
                pieces.push(parts.part(Torus::new(0.58, 0.66), racer.kart, ridge));
            }
            pieces.push(parts.part(
                Cylinder::new(0.66, 0.4),
                racer.accent,
                at(0.0, 1.05, 1.0).with_rotation(Quat::from_rotation_x(1.57)),
            ));
            for eye in [-0.2, 0.2] {
                pieces.push(parts.part(Sphere::new(0.08), Color::WHITE, at(eye, 1.2, -0.72)));
                pieces.push(parts.part(Sphere::new(0.04), Color::BLACK, at(eye, 1.2, -0.79)));
            }
        }
        Body::Corn => {
            pieces.push(parts.part(Cuboid::new(1.4, 0.3, 2.1), racer.accent, at(0.0, 0.5, 0.0)));
            pieces.push(parts.part(
                Capsule3d::new(0.42, 1.0),
                racer.kart,
                at(0.0, 1.35, 0.2).with_rotation(Quat::from_rotation_x(0.35)),
            ));
            // Kernel bumps
            for i in 0..6 {
                for j in 0..6 {
                    let a = j as f32 * std::f32::consts::TAU / 6.0 + i as f32 * 0.5;
                    let y = -0.5 + i as f32 * 0.2;
                    let kernel = Transform::from_xyz(0.0, 1.35, 0.2)
                        .with_rotation(Quat::from_rotation_x(0.35))
                        * Transform::from_xyz(a.cos() * 0.42, y, a.sin() * 0.42);
                    pieces.push(parts.part(
                        Sphere::new(0.09),
                        Color::srgb(0.99, 0.88, 0.35),
                        kernel,
                    ));
                }
            }
            for side in [-1.0f32, 1.0] {
                let husk = Transform::from_xyz(side * 0.5, 1.0, 0.6)
                    .with_rotation(Quat::from_rotation_z(side * -0.7) * Quat::from_rotation_x(0.6))
                    .with_scale(Vec3::new(1.0, 1.0, 0.2));
                pieces.push(parts.part(Capsule3d::new(0.25, 1.1), racer.accent, husk));
            }
            for eye in [-0.15, 0.15] {
                pieces.push(parts.part(Sphere::new(0.07), Color::BLACK, at(eye, 1.75, -0.2)));
            }
        }
        Body::Potato => {
            pieces.push(parts.part(
                Cuboid::new(1.5, 0.35, 2.2),
                Color::srgb(0.35, 0.22, 0.12),
                at(0.0, 0.5, 0.0),
            ));
            pieces.push(parts.part(
                Sphere::new(0.7),
                racer.kart,
                at(0.0, 1.25, 0.1).with_scale(Vec3::new(1.0, 1.15, 0.9)),
            ));
            // Pilgrim hat with buckle
            pieces.push(parts.part(Cylinder::new(0.75, 0.06), racer.accent, at(0.0, 1.98, 0.1)));
            pieces.push(parts.part(Cylinder::new(0.42, 0.65), racer.accent, at(0.0, 2.3, 0.1)));
            pieces.push(parts.part(
                Cylinder::new(0.43, 0.12),
                Color::srgb(0.95, 0.93, 0.9),
                at(0.0, 2.08, 0.1),
            ));
            pieces.push(parts.part(
                Cuboid::new(0.22, 0.16, 0.05),
                Color::srgb(0.95, 0.78, 0.2),
                at(0.0, 2.08, -0.33),
            ));
            for eye in [-0.2, 0.2] {
                pieces.push(parts.part(Sphere::new(0.08), Color::WHITE, at(eye, 1.5, -0.52)));
                pieces.push(parts.part(Sphere::new(0.04), Color::BLACK, at(eye, 1.5, -0.59)));
            }
        }
        Body::Casserole => {
            pieces.push(parts.part(
                Cuboid::new(1.7, 0.6, 2.4),
                Color::srgb(0.92, 0.92, 0.95),
                at(0.0, 0.7, 0.0),
            ));
            pieces.push(parts.part(Cuboid::new(1.5, 0.2, 2.2), racer.kart, at(0.0, 0.95, 0.0)));
            let mut seed = 7u32;
            for _ in 0..14 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                let x = (seed >> 8) as f32 / 16_777_216.0 * 1.2 - 0.6;
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                let z = (seed >> 8) as f32 / 16_777_216.0 * 1.8 - 0.9;
                pieces.push(parts.part(Cylinder::new(0.15, 0.18), racer.accent, at(x, 1.12, z)));
            }
            // A toasted marshmallow pilot
            pieces.push(parts.part(Cylinder::new(0.38, 0.7), racer.accent, at(0.0, 1.5, 0.3)));
            pieces.push(parts.part(
                Cylinder::new(0.39, 0.12),
                Color::srgb(0.75, 0.50, 0.25),
                at(0.0, 1.85, 0.3),
            ));
            for eye in [-0.12, 0.12] {
                pieces.push(parts.part(Sphere::new(0.06), Color::BLACK, at(eye, 1.6, -0.07)));
            }
        }
        Body::Cat => {
            let fur = racer.accent;
            let dark = Color::srgb(0.30, 0.30, 0.34);
            let white = Color::srgb(0.97, 0.97, 0.97);
            let pink = Color::srgb(0.95, 0.55, 0.62);
            // A licked-clean turkey platter
            pieces.push(parts.part(
                Cylinder::new(1.1, 0.18),
                racer.kart,
                at(0.0, 0.55, 0.0).with_scale(Vec3::new(1.0, 1.0, 1.3)),
            ));
            pieces.push(parts.part(
                Torus::new(1.0, 1.18),
                Color::srgb(0.80, 0.68, 0.35),
                at(0.0, 0.66, 0.0).with_scale(Vec3::new(1.0, 1.0, 1.3)),
            ));
            // Body and white chest
            pieces.push(parts.part(
                Sphere::new(0.55),
                fur,
                at(0.0, 1.1, 0.25).with_scale(Vec3::new(1.0, 1.0, 1.2)),
            ));
            pieces.push(parts.part(Sphere::new(0.42), white, at(0.0, 1.15, -0.12)));
            pieces.push(parts.part(
                Capsule3d::new(0.08, 0.8),
                fur,
                at(0.0, 1.4, 0.95).with_rotation(Quat::from_rotation_x(0.6)),
            ));
            // Head
            pieces.push(parts.part(Sphere::new(0.5), dark, at(0.0, 1.85, -0.15)));
            for side in [-1.0f32, 1.0] {
                pieces.push(parts.part(
                    Cone::new(0.2, 0.42),
                    dark,
                    at(side * 0.3, 2.3, -0.1).with_rotation(Quat::from_rotation_z(side * -0.35)),
                ));
                pieces.push(parts.part(
                    Cone::new(0.11, 0.25),
                    pink,
                    at(side * 0.29, 2.26, -0.17).with_rotation(Quat::from_rotation_z(side * -0.35)),
                ));
                // Huge glossy eyes welling up
                let eye = at(side * 0.2, 1.98, -0.55);
                pieces.push((
                    Mesh3d(parts.meshes.add(Sphere::new(0.17))),
                    MeshMaterial3d(parts.materials.add(StandardMaterial {
                        base_color: Color::srgb(0.05, 0.04, 0.06),
                        perceptual_roughness: 0.05,
                        reflectance: 1.0,
                        ..default()
                    })),
                    eye,
                ));
                let sparkle = parts.glow(Color::WHITE, 4.0);
                for (dx, dy, r) in [(0.05, 0.06, 0.05), (-0.04, -0.05, 0.025)] {
                    pieces.push((
                        Mesh3d(parts.meshes.add(Sphere::new(r))),
                        MeshMaterial3d(sparkle.clone()),
                        at(side * 0.2 + dx, 1.98 + dy, -0.7),
                    ));
                }
                // Tears pooling on the lower lids
                pieces.push(parts.part(
                    Capsule3d::new(0.05, 0.22),
                    Color::srgba(0.6, 0.85, 1.0, 0.9),
                    at(side * 0.2, 1.84, -0.66).with_rotation(Quat::from_rotation_z(1.57)),
                ));
            }
            // A tear rolling down one cheek
            pieces.push(parts.part(
                Sphere::new(0.07),
                Color::srgb(0.6, 0.85, 1.0),
                at(0.36, 1.68, -0.55).with_scale(Vec3::new(1.0, 1.5, 1.0)),
            ));
            // White muzzle
            pieces.push(parts.part(
                Sphere::new(0.26),
                white,
                at(0.0, 1.72, -0.5).with_scale(Vec3::new(1.2, 0.9, 0.8)),
            ));
            pieces.push(parts.part(Sphere::new(0.05), pink, at(0.0, 1.8, -0.72)));
        }
    }

    commands.entity(root).with_children(|parent| {
        for piece in pieces {
            parent.spawn(piece);
        }
    });

    // The wailing mouth: dark open jaw, tongue and fangs, scaled as one
    if racer.body == Body::Cat {
        let jaw = commands
            .spawn((
                Mouth,
                Transform::from_xyz(0.0, 1.6, -0.58),
                Visibility::default(),
            ))
            .id();
        let mut mouth = vec![
            parts.part(
                Sphere::new(0.17),
                Color::srgb(0.25, 0.02, 0.05),
                at(0.0, 0.0, 0.0).with_scale(Vec3::new(1.0, 1.3, 0.5)),
            ),
            parts.part(
                Sphere::new(0.1),
                Color::srgb(0.95, 0.45, 0.55),
                at(0.0, -0.1, -0.04).with_scale(Vec3::new(1.0, 0.5, 0.6)),
            ),
        ];
        for side in [-1.0, 1.0] {
            mouth.push(
                parts.part(
                    Cone::new(0.03, 0.08),
                    Color::WHITE,
                    at(side * 0.08, 0.14, -0.06)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
                ),
            );
        }
        commands.entity(jaw).with_children(|parent| {
            for piece in mouth {
                parent.spawn(piece);
            }
        });
        commands.entity(root).add_child(jaw);
    }
}
