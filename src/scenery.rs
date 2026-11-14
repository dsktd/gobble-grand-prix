//! Props along each track and falling autumn leaves.

use bevy::prelude::*;
use rand::Rng;

use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn.after(spawn_track));
    app.add_systems(Update, fall.run_if(in_state(GameState::Race)));
}

#[derive(Component, Clone, Debug)]
struct Leaf {
    vel: Vec3,
    spin: Vec3,
}

/// Shared handles so hundreds of props cost one mesh and material each.
struct Kit {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

fn kit(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    mesh: impl Into<Mesh>,
    color: Color,
) -> Kit {
    Kit {
        mesh: meshes.add(mesh.into()),
        material: materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.8,
            ..default()
        }),
    }
}

fn put(
    commands: &mut Commands,
    kit: &Kit,
    transform: Transform,
) {
    commands.spawn((
        Mesh3d(kit.mesh.clone()),
        MeshMaterial3d(kit.material.clone()),
        transform,
        DespawnOnExit(GameState::Race),
    ));
}

fn spawn(
    mut commands: Commands,
    track: Res<Track>,
    options: Res<Options>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::rng();
    let hw = track.half_width;
    let edge = track.barrier() + 1.5;
    let lap = track.length;
    let m = &mut *meshes;
    let mt = &mut *materials;

    // Keep props off the road: only accept spots well outside it
    let clear = |p: Vec2, track: &Track| {
        track.locate(p).lateral.abs() > track.barrier() + 1.0 && !track.in_shortcut(p)
    };

    match track.def().scenery {
        Scenery::Farm => {
            let post = kit(
                m,
                mt,
                Cuboid::new(0.3, 1.4, 0.3),
                Color::srgb(0.45, 0.30, 0.18),
            );
            let rail = kit(
                m,
                mt,
                Cuboid::new(0.15, 0.15, 4.0),
                Color::srgb(0.55, 0.38, 0.22),
            );
            let trunk = kit(
                m,
                mt,
                Cylinder::new(0.5, 6.0),
                Color::srgb(0.35, 0.22, 0.12),
            );
            let crowns = [
                kit(m, mt, Sphere::new(3.5), Color::srgb(0.90, 0.45, 0.10)),
                kit(m, mt, Sphere::new(3.5), Color::srgb(0.80, 0.20, 0.08)),
                kit(m, mt, Sphere::new(3.5), Color::srgb(0.95, 0.70, 0.15)),
            ];
            let hay = kit(
                m,
                mt,
                Cylinder::new(1.2, 1.8),
                Color::srgb(0.88, 0.75, 0.40),
            );
            let pumpkin = kit(m, mt, Sphere::new(0.9), Color::srgb(0.95, 0.50, 0.08));
            let barn = kit(
                m,
                mt,
                Cuboid::new(16.0, 10.0, 22.0),
                Color::srgb(0.62, 0.12, 0.08),
            );
            let roof = kit(
                m,
                mt,
                Cuboid::new(12.5, 0.6, 23.0),
                Color::srgb(0.25, 0.20, 0.18),
            );

            // Fences both sides
            let mut along = 0.0;
            while along < lap {
                for side in [-1.0, 1.0] {
                    let p = track.offset(along, side * edge);
                    let (_, dir) = track.sample(along);
                    let yaw = (-dir.x).atan2(-dir.y);
                    put(
                        &mut commands,
                        &post,
                        Transform::from_translation(flat(p, 0.7)),
                    );
                    put(
                        &mut commands,
                        &rail,
                        Transform::from_translation(flat(p, 1.0))
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_translation(flat(track.offset(along + 2.0, side * edge), 1.0)),
                    );
                }
                along += 4.0;
            }
            // Orchard
            let trees = if options.high_quality { 140 } else { 60 };
            for i in 0..trees {
                let along = rng.random_range(0.0..lap);
                let side = match i % 2 {
                    0 => 1.0,
                    _ => -1.0,
                };
                let p = track.offset(along, side * (edge + rng.random_range(6.0..50.0)));
                if !clear(p, &track) {
                    continue;
                }
                put(
                    &mut commands,
                    &trunk,
                    Transform::from_translation(flat(p, 3.0)),
                );
                put(
                    &mut commands,
                    &crowns[i % 3],
                    Transform::from_translation(flat(p, 7.5)).with_scale(Vec3::new(
                        1.0,
                        rng.random_range(0.8..1.2),
                        1.0,
                    )),
                );
            }
            for _ in 0..40 {
                let along = rng.random_range(0.0..lap);
                let p = track.offset(
                    along,
                    rng.random_range(edge + 1.5..edge + 6.0) * [1.0, -1.0][rng.random_range(0..2)],
                );
                if !clear(p, &track) {
                    continue;
                }
                if rng.random_bool(0.5) {
                    put(
                        &mut commands,
                        &hay,
                        Transform::from_translation(flat(p, 1.2))
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    );
                } else {
                    put(
                        &mut commands,
                        &pumpkin,
                        Transform::from_translation(flat(p, 0.7))
                            .with_scale(Vec3::new(1.2, 0.8, 1.2)),
                    );
                }
            }
            put(&mut commands, &barn, Transform::from_xyz(20.0, 5.0, 60.0));
            for side in [-1.0f32, 1.0] {
                put(
                    &mut commands,
                    &roof,
                    Transform::from_xyz(20.0 + side * 4.3, 12.4, 60.0)
                        .with_rotation(Quat::from_rotation_z(side * -0.6)),
                );
            }
        }
        Scenery::Table => {
            let plate = kit(
                m,
                mt,
                Cylinder::new(9.0, 0.4),
                Color::srgb(0.97, 0.96, 0.92),
            );
            let rim = kit(m, mt, Torus::new(8.0, 9.4), Color::srgb(0.85, 0.70, 0.30));
            let roast = kit(m, mt, Sphere::new(14.0), Color::srgb(0.60, 0.33, 0.12));
            let drum = kit(
                m,
                mt,
                Capsule3d::new(3.5, 10.0),
                Color::srgb(0.65, 0.36, 0.14),
            );
            let candle = kit(
                m,
                mt,
                Cylinder::new(1.0, 12.0),
                Color::srgb(0.95, 0.92, 0.85),
            );
            let flame = Kit {
                mesh: m.add(Sphere::new(0.7)),
                material: mt.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.7, 0.2),
                    emissive: LinearRgba::rgb(20.0, 9.0, 2.0),
                    ..default()
                }),
            };
            let pea = kit(m, mt, Sphere::new(0.8), Color::srgb(0.35, 0.65, 0.20));
            let roll = kit(m, mt, Sphere::new(1.8), Color::srgb(0.85, 0.60, 0.30));

            // The roast sits in the middle of the table
            put(
                &mut commands,
                &roast,
                Transform::from_xyz(0.0, 4.0, -18.0).with_scale(Vec3::new(1.3, 0.7, 1.0)),
            );
            for side in [-1.0f32, 1.0] {
                put(
                    &mut commands,
                    &drum,
                    Transform::from_xyz(side * 16.0, 5.0, -10.0).with_rotation(
                        Quat::from_rotation_x(1.2) * Quat::from_rotation_z(side * 0.4),
                    ),
                );
            }
            for (x, z) in [
                (-60.0, -15.0),
                (60.0, -15.0),
                (-140.0, 20.0),
                (140.0, 20.0),
                (0.0, -90.0),
                (0.0, 100.0),
                (-100.0, -80.0),
                (100.0, -80.0),
            ] {
                put(&mut commands, &plate, Transform::from_xyz(x, 0.2, z));
                put(&mut commands, &rim, Transform::from_xyz(x, 0.35, z));
            }
            for (x, z) in [
                (-130.0, -60.0),
                (130.0, -60.0),
                (-130.0, 80.0),
                (130.0, 80.0),
            ] {
                put(&mut commands, &candle, Transform::from_xyz(x, 6.0, z));
                put(
                    &mut commands,
                    &flame,
                    Transform::from_xyz(x, 12.8, z).with_scale(Vec3::new(1.0, 1.8, 1.0)),
                );
            }
            // Peas and rolls spilled along the runner
            let mut along = 0.0;
            while along < lap {
                for side in [-1.0, 1.0] {
                    let p = track.offset(along, side * edge);
                    match (along as i32 / 6) % 4 {
                        0 => put(
                            &mut commands,
                            &roll,
                            Transform::from_translation(flat(p, 1.2))
                                .with_scale(Vec3::new(1.0, 0.7, 1.4)),
                        ),
                        _ => put(
                            &mut commands,
                            &pea,
                            Transform::from_translation(flat(p, 0.8)),
                        ),
                    }
                }
                along += 6.0;
            }
        }
        Scenery::Maze => {
            let stalk = kit(
                m,
                mt,
                Cylinder::new(0.18, 5.5),
                Color::srgb(0.55, 0.60, 0.20),
            );
            let cob = kit(
                m,
                mt,
                Capsule3d::new(0.25, 0.7),
                Color::srgb(0.95, 0.80, 0.25),
            );
            let tassel = kit(m, mt, Cone::new(0.5, 1.2), Color::srgb(0.85, 0.70, 0.35));
            let pole = kit(
                m,
                mt,
                Cylinder::new(0.2, 6.0),
                Color::srgb(0.40, 0.28, 0.15),
            );
            let shirt = kit(
                m,
                mt,
                Cuboid::new(2.8, 1.8, 0.6),
                Color::srgb(0.65, 0.25, 0.20),
            );
            let head = kit(m, mt, Sphere::new(0.7), Color::srgb(0.90, 0.75, 0.45));
            let hat = kit(m, mt, Cone::new(1.1, 1.0), Color::srgb(0.55, 0.40, 0.20));

            // Dense rows of corn hem the road in
            let rows = if options.high_quality { 4 } else { 2 };
            let mut along = 0.0;
            while along < lap {
                for side in [-1.0, 1.0] {
                    for row in 0..rows {
                        let jitter = rng.random_range(-0.6..0.6);
                        let p = track.offset(along + jitter, side * (edge + row as f32 * 1.6));
                        if !clear(p, &track) {
                            continue;
                        }
                        let h = rng.random_range(0.85..1.2);
                        put(
                            &mut commands,
                            &stalk,
                            Transform::from_translation(flat(p, 2.75 * h))
                                .with_scale(Vec3::new(1.0, h, 1.0)),
                        );
                        put(
                            &mut commands,
                            &tassel,
                            Transform::from_translation(flat(p, 5.5 * h + 0.4)),
                        );
                        if rng.random_bool(0.3) {
                            put(
                                &mut commands,
                                &cob,
                                Transform::from_translation(flat(p, 3.0 * h) + Vec3::X * 0.3)
                                    .with_rotation(Quat::from_rotation_z(-0.5)),
                            );
                        }
                    }
                }
                along += 1.4;
            }
            // Scarecrows keep watch
            for frac in [0.1, 0.35, 0.6, 0.85] {
                let p = track.offset(lap * frac, -(hw + 3.5));
                put(
                    &mut commands,
                    &pole,
                    Transform::from_translation(flat(p, 3.0)),
                );
                put(
                    &mut commands,
                    &shirt,
                    Transform::from_translation(flat(p, 4.0)),
                );
                put(
                    &mut commands,
                    &head,
                    Transform::from_translation(flat(p, 5.6)),
                );
                put(
                    &mut commands,
                    &hat,
                    Transform::from_translation(flat(p, 6.5)),
                );
            }
        }
    }

    // Falling leaves around the camera
    let leaf_mesh = m.add(Rectangle::new(0.5, 0.35));
    let colors = [
        Color::srgb(0.92, 0.45, 0.08),
        Color::srgb(0.80, 0.18, 0.08),
        Color::srgb(0.95, 0.72, 0.15),
        Color::srgb(0.60, 0.30, 0.10),
    ];
    let leaf_mats: Vec<_> = colors
        .iter()
        .map(|c| {
            mt.add(StandardMaterial {
                base_color: *c,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    let count = if options.high_quality { 260 } else { 90 };
    for i in 0..count {
        commands.spawn((
            Leaf {
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
            },
            Mesh3d(leaf_mesh.clone()),
            MeshMaterial3d(leaf_mats[i % leaf_mats.len()].clone()),
            Transform::from_xyz(
                rng.random_range(-40.0..40.0),
                rng.random_range(0.0..25.0),
                rng.random_range(-40.0..40.0),
            ),
            DespawnOnExit(GameState::Race),
        ));
    }
}

/// Leaves drift down and wrap around the first chase camera.
fn fall(
    time: Res<Time>,
    cameras: Query<(&ChaseCam, &Transform), Without<Leaf>>,
    leaves: Query<(&Leaf, &mut Transform)>,
) {
    let Some((_, camera)) = cameras.iter().find(|(c, _)| c.0 == 0) else {
        return;
    };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let center = camera.translation;
    for (leaf, mut transform) in leaves {
        let sway = Vec3::new(
            (t * 1.3 + transform.translation.y).sin(),
            0.0,
            (t * 0.9).cos(),
        ) * 0.8;
        transform.translation += (leaf.vel + sway) * dt;
        transform.rotate_local_x(leaf.spin.x * dt);
        transform.rotate_local_y(leaf.spin.y * dt);
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
