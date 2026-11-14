//! The Thanksgiving racers and the primitive meshes they are built from.

use bevy::prelude::*;

pub(super) mod prelude {
    pub use super::Parts;
    pub use super::ROSTER;
    pub use super::Stats;
    pub use super::spawn_model;
}

pub(super) fn plugin(_app: &mut App) {
    //
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

pub const ROSTER: [Racer; 6] = [
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
];

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

fn at(
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
            pieces.push(parts.part(
                Sphere::new(0.6),
                racer.accent,
                at(0.0, 1.25, 0.15),
            ));
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
            pieces.push(parts.part(
                Cylinder::new(1.05, 0.45),
                racer.accent,
                at(0.0, 0.75, 0.0),
            ));
            pieces.push(parts.part(
                Cylinder::new(0.92, 0.47),
                racer.kart,
                at(0.0, 0.77, 0.0),
            ));
            pieces.push(parts.part(
                Torus::new(0.9, 1.12),
                racer.accent,
                at(0.0, 1.0, 0.0),
            ));
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
            pieces.push(parts.part(
                Cuboid::new(1.4, 0.3, 2.1),
                racer.accent,
                at(0.0, 0.5, 0.0),
            ));
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
            pieces.push(parts.part(
                Cylinder::new(0.75, 0.06),
                racer.accent,
                at(0.0, 1.98, 0.1),
            ));
            pieces.push(parts.part(
                Cylinder::new(0.42, 0.65),
                racer.accent,
                at(0.0, 2.3, 0.1),
            ));
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
            pieces.push(parts.part(
                Cuboid::new(1.5, 0.2, 2.2),
                racer.kart,
                at(0.0, 0.95, 0.0),
            ));
            let mut seed = 7u32;
            for _ in 0..14 {
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                let x = (seed >> 8) as f32 / 16_777_216.0 * 1.2 - 0.6;
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                let z = (seed >> 8) as f32 / 16_777_216.0 * 1.8 - 0.9;
                pieces.push(parts.part(
                    Cylinder::new(0.15, 0.18),
                    racer.accent,
                    at(x, 1.12, z),
                ));
            }
            // A toasted marshmallow pilot
            pieces.push(parts.part(
                Cylinder::new(0.38, 0.7),
                racer.accent,
                at(0.0, 1.5, 0.3),
            ));
            pieces.push(parts.part(
                Cylinder::new(0.39, 0.12),
                Color::srgb(0.75, 0.50, 0.25),
                at(0.0, 1.85, 0.3),
            ));
            for eye in [-0.12, 0.12] {
                pieces.push(parts.part(Sphere::new(0.06), Color::BLACK, at(eye, 1.6, -0.07)));
            }
        }
    }

    commands.entity(root).with_children(|parent| {
        for piece in pieces {
            parent.spawn(piece);
        }
    });
}
