//! The pool-hall racers and the primitive meshes they are built from.

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
    EightBall,
    Striped,
    Chalk,
    Rack,
    Hustler,
    Table,
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
        name: "Eight Ball Eddie",
        blurb: "An 8-ball in a cue-ball kart. Balanced, and sure he is lucky.",
        kart: Color::srgb(0.95, 0.95, 0.92),
        accent: Color::srgb(0.06, 0.06, 0.07),
        body: Body::EightBall,
        stats: Stats {
            top_speed: 1.0,
            accel: 1.0,
            turn: 1.0,
            weight: 1.0,
        },
    },
    Racer {
        name: "Stripe Sally",
        blurb: "A striped ball with somewhere to be. Quick off the break, light on the corners.",
        kart: Color::srgb(0.92, 0.72, 0.10),
        accent: Color::srgb(0.70, 0.12, 0.12),
        body: Body::Striped,
        stats: Stats {
            top_speed: 0.96,
            accel: 1.25,
            turn: 1.05,
            weight: 0.8,
        },
    },
    Racer {
        name: "Chalky Blue",
        blurb: "A cube of tournament chalk. Heavy, dusty and fast once rolling.",
        kart: Color::srgb(0.20, 0.42, 0.85),
        accent: Color::srgb(0.12, 0.26, 0.60),
        body: Body::Chalk,
        stats: Stats {
            top_speed: 1.07,
            accel: 0.85,
            turn: 0.9,
            weight: 1.4,
        },
    },
    Racer {
        name: "Rack Rita",
        blurb: "Fifteen balls and a triangle, all moving as one. Turns on a dime.",
        kart: Color::srgb(0.75, 0.55, 0.30),
        accent: Color::srgb(0.10, 0.45, 0.22),
        body: Body::Rack,
        stats: Stats {
            top_speed: 0.98,
            accel: 1.05,
            turn: 1.25,
            weight: 0.9,
        },
    },
    Racer {
        name: "Hustler Hank",
        blurb: "Lost the first game on purpose. Steady, sturdy, wearing sunglasses indoors.",
        kart: Color::srgb(0.85, 0.65, 0.50),
        accent: Color::srgb(0.10, 0.10, 0.12),
        body: Body::Hustler,
        stats: Stats {
            top_speed: 1.02,
            accel: 0.95,
            turn: 0.95,
            weight: 1.2,
        },
    },
    Racer {
        name: "Felt Felicia",
        blurb: "A whole pool table on wheels. Drifts like a dream.",
        kart: Color::srgb(0.10, 0.52, 0.28),
        accent: Color::srgb(0.98, 0.95, 0.88),
        body: Body::Table,
        stats: Stats {
            top_speed: 1.0,
            accel: 1.1,
            turn: 1.12,
            weight: 0.95,
        },
    },
    Racer {
        name: "Wailing Whiskers",
        blurb: "Bet it all on the 8-ball and lost. Has not stopped crying since.",
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
        Body::EightBall => {
            // Cue-ball hull with a rail around it
            pieces.push(parts.part(
                Sphere::new(1.0),
                racer.kart,
                at(0.0, 0.55, 0.0).with_scale(Vec3::new(0.85, 0.35, 1.25)),
            ));
            pieces.push(parts.part(
                Cylinder::new(0.8, 0.08),
                Color::srgb(0.35, 0.18, 0.08),
                at(0.0, 0.78, 0.0).with_scale(Vec3::new(0.95, 1.0, 1.4)),
            ));
            pieces.push(parts.part(
                Cone::new(0.25, 0.6),
                racer.kart,
                at(0.0, 0.75, -1.3).with_rotation(Quat::from_rotation_x(-1.2)),
            ));
            // The 8-ball driver: black ball, white spot, a pair of eyes
            pieces.push(parts.part(Sphere::new(0.75), racer.accent, at(0.0, 1.45, 0.1)));
            pieces.push(parts.part(
                Cylinder::new(0.3, 0.06),
                Color::WHITE,
                at(0.0, 1.55, -0.62).with_rotation(Quat::from_rotation_x(1.2)),
            ));
            for eye in [-0.2, 0.2] {
                pieces.push(parts.part(Sphere::new(0.08), Color::WHITE, at(eye, 1.2, -0.67)));
                pieces.push(parts.part(Sphere::new(0.04), Color::BLACK, at(eye, 1.2, -0.74)));
            }
            // A fan of cue sticks out the back
            let woods = [
                Color::srgb(0.85, 0.65, 0.35),
                Color::srgb(0.45, 0.25, 0.12),
                Color::srgb(0.75, 0.50, 0.25),
                Color::srgb(0.30, 0.16, 0.08),
            ];
            for i in 0..7 {
                let angle = -0.9 + i as f32 * 0.3;
                let cue = Transform::from_xyz(0.0, 1.0, 0.8)
                    .with_rotation(Quat::from_rotation_z(angle) * Quat::from_rotation_x(0.35))
                    * Transform::from_xyz(0.0, 0.8, 0.0);
                pieces.push(parts.part(Capsule3d::new(0.06, 1.5), woods[i % 4], cue));
            }
        }
        Body::Striped => {
            // A striped ball riding a round pocket liner
            pieces.push(parts.part(Cylinder::new(1.05, 0.45), racer.accent, at(0.0, 0.75, 0.0)));
            pieces.push(parts.part(Cylinder::new(0.92, 0.47), Color::srgb(0.08, 0.08, 0.08), at(0.0, 0.77, 0.0)));
            pieces.push(parts.part(Torus::new(0.9, 1.12), racer.accent, at(0.0, 1.0, 0.0)));
            pieces.push(parts.part(Sphere::new(0.8), racer.kart, at(0.0, 1.55, 0.05)));
            pieces.push(parts.part(
                Cylinder::new(0.81, 0.4),
                Color::WHITE,
                at(0.0, 1.55, 0.05).with_rotation(Quat::from_rotation_x(1.57)),
            ));
            for eye in [-0.2, 0.2] {
                pieces.push(parts.part(Sphere::new(0.07), Color::BLACK, at(eye, 1.6, -0.78)));
            }
        }
        Body::Chalk => {
            pieces.push(parts.part(
                Cuboid::new(1.5, 0.3, 2.2),
                Color::srgb(0.3, 0.3, 0.32),
                at(0.0, 0.5, 0.0),
            ));
            // A big cube of blue chalk with the worn dimple on top
            pieces.push(parts.part(Cuboid::new(1.3, 1.2, 1.3), racer.kart, at(0.0, 1.2, 0.1)));
            pieces.push(parts.part(
                Cylinder::new(0.45, 0.08),
                racer.accent,
                at(0.0, 1.8, 0.1),
            ));
            for eye in [-0.25, 0.25] {
                pieces.push(parts.part(Sphere::new(0.1), Color::WHITE, at(eye, 1.4, -0.58)));
                pieces.push(parts.part(Sphere::new(0.05), Color::BLACK, at(eye, 1.4, -0.67)));
            }
        }
        Body::Rack => {
            pieces.push(parts.part(Cuboid::new(1.4, 0.3, 2.1), racer.accent, at(0.0, 0.5, 0.0)));
            // Fifteen balls, minus a few: a rack triangle on wheels
            let colors = [
                Color::srgb(0.95, 0.80, 0.15),
                Color::srgb(0.15, 0.30, 0.80),
                Color::srgb(0.80, 0.15, 0.15),
                Color::srgb(0.45, 0.15, 0.55),
                Color::srgb(0.90, 0.45, 0.10),
                Color::srgb(0.10, 0.50, 0.25),
            ];
            let mut n = 0;
            for row in 0..4 {
                for col in 0..=row {
                    let x = (col as f32 - row as f32 / 2.0) * 0.52;
                    let z = -0.7 + row as f32 * 0.5;
                    pieces.push(parts.part(Sphere::new(0.25), colors[n % 6], at(x, 0.9, z)));
                    n += 1;
                }
            }
            // Wooden triangle frame, one bar per side of the rack
            pieces.push(parts.part(Cuboid::new(2.2, 0.18, 0.12), racer.kart, at(0.0, 0.85, 1.05)));
            for side in [-1.0f32, 1.0] {
                pieces.push(parts.part(
                    Cuboid::new(0.12, 0.18, 2.2),
                    racer.kart,
                    at(side * 0.95, 0.85, 0.15).with_rotation(Quat::from_rotation_y(side * 0.45)),
                ));
            }
            // The driver peeks over the top row
            pieces.push(parts.part(Sphere::new(0.4), Color::WHITE, at(0.0, 1.45, -0.1)));
            for eye in [-0.15, 0.15] {
                pieces.push(parts.part(Sphere::new(0.07), Color::BLACK, at(eye, 1.55, -0.45)));
            }
        }
        Body::Hustler => {
            pieces.push(parts.part(
                Cuboid::new(1.5, 0.35, 2.2),
                Color::srgb(0.12, 0.10, 0.10),
                at(0.0, 0.5, 0.0),
            ));
            pieces.push(parts.part(
                Sphere::new(0.7),
                racer.kart,
                at(0.0, 1.25, 0.1).with_scale(Vec3::new(1.0, 1.15, 0.9)),
            ));
            // Pool-hall fedora with a red band
            pieces.push(parts.part(Cylinder::new(0.85, 0.06), racer.accent, at(0.0, 1.9, 0.1)));
            pieces.push(parts.part(Cylinder::new(0.45, 0.5), racer.accent, at(0.0, 2.15, 0.1)));
            pieces.push(parts.part(
                Cylinder::new(0.46, 0.12),
                Color::srgb(0.70, 0.10, 0.12),
                at(0.0, 2.0, 0.1),
            ));
            // Shades
            pieces.push(parts.part(Cuboid::new(0.8, 0.14, 0.06), Color::BLACK, at(0.0, 1.5, -0.6)));
            // A cue over the shoulder
            pieces.push(parts.part(
                Capsule3d::new(0.05, 2.4),
                Color::srgb(0.75, 0.50, 0.25),
                at(0.7, 1.6, 0.4).with_rotation(Quat::from_rotation_z(-0.3)),
            ));
        }
        Body::Table => {
            // A little pool table: wooden rails, a felt top, diamond sights and a ball pilot
            pieces.push(parts.part(
                Cuboid::new(1.7, 0.6, 2.4),
                Color::srgb(0.35, 0.18, 0.08),
                at(0.0, 0.7, 0.0),
            ));
            pieces.push(parts.part(Cuboid::new(1.5, 0.2, 2.2), racer.kart, at(0.0, 0.95, 0.0)));
            for i in 0..5 {
                let z = -0.9 + i as f32 * 0.45;
                for x in [-0.8, 0.8] {
                    pieces.push(parts.part(
                        Cuboid::new(0.1, 0.06, 0.1),
                        racer.accent,
                        at(x, 1.07, z).with_rotation(Quat::from_rotation_y(0.78)),
                    ));
                }
            }
            for (x, z) in [(-0.75, -1.05), (0.75, -1.05), (-0.75, 1.05), (0.75, 1.05)] {
                pieces.push(parts.part(Cylinder::new(0.14, 0.05), Color::BLACK, at(x, 1.07, z)));
            }
            pieces.push(parts.part(Sphere::new(0.4), racer.accent, at(0.0, 1.5, 0.3)));
            for eye in [-0.12, 0.12] {
                pieces.push(parts.part(Sphere::new(0.06), Color::BLACK, at(eye, 1.58, -0.04)));
            }
        }
        Body::Cat => {
            let fur = racer.accent;
            let dark = Color::srgb(0.30, 0.30, 0.34);
            let white = Color::srgb(0.97, 0.97, 0.97);
            let pink = Color::srgb(0.95, 0.55, 0.62);
            // A ball-return tray
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
