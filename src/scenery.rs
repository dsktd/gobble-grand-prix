//! Props along each track and drifting chalk dust.

use bevy::prelude::*;
use rand::Rng;

use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn.after(spawn_track));
    app.add_systems(Update, fall.run_if(on_track));
}

#[derive(Component, Clone, Debug)]
struct Dust {
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

/// One prop, standing on the ground wherever its transform puts it.
fn put(
    commands: &mut Commands,
    track: &Track,
    kit: &Kit,
    mut transform: Transform,
) {
    let at = transform.translation;
    transform.translation.y += track.ground(Vec2::new(at.x, at.z));
    commands.spawn((
        Mesh3d(kit.mesh.clone()),
        MeshMaterial3d(kit.material.clone()),
        transform,
        DespawnOnExit(GameState::Race),
    ));
}

pub(super) fn spawn(
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

    match track.scenery {
        Scenery::Bar => {
            let post = kit(
                m,
                mt,
                Cuboid::new(0.3, 1.4, 0.3),
                Color::srgb(0.85, 0.65, 0.20),
            );
            let rail = kit(
                m,
                mt,
                Cuboid::new(0.15, 0.15, 4.0),
                Color::srgb(0.40, 0.20, 0.08),
            );
            let trunk = kit(
                m,
                mt,
                Cylinder::new(0.5, 6.0),
                Color::srgb(0.35, 0.22, 0.12),
            );
            let crowns = [
                kit(m, mt, Sphere::new(3.5), Color::srgb(0.95, 0.80, 0.10)),
                kit(m, mt, Sphere::new(3.5), Color::srgb(0.80, 0.10, 0.10)),
                kit(m, mt, Sphere::new(3.5), Color::srgb(0.15, 0.30, 0.80)),
            ];
            let stool = kit(
                m,
                mt,
                Cylinder::new(1.2, 1.8),
                Color::srgb(0.70, 0.12, 0.12),
            );
            let ball = kit(m, mt, Sphere::new(0.9), Color::srgb(0.95, 0.50, 0.08));
            let jukebox = kit(
                m,
                mt,
                Cuboid::new(16.0, 10.0, 22.0),
                Color::srgb(0.55, 0.08, 0.30),
            );
            let roof = kit(
                m,
                mt,
                Cuboid::new(12.5, 0.6, 23.0),
                Color::srgb(0.25, 0.20, 0.18),
            );

            // Brass rail posts both sides
            let mut along = 0.0;
            while along < lap {
                for side in [-1.0, 1.0] {
                    let p = track.offset(along, side * edge);
                    let (_, dir) = track.sample(along);
                    let yaw = (-dir.x).atan2(-dir.y);
                    put(
                        &mut commands,
                        &track,
                        &post,
                        Transform::from_translation(flat(p, 0.7)),
                    );
                    put(
                        &mut commands,
                        &track,
                        &rail,
                        Transform::from_translation(flat(p, 1.0))
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_translation(flat(track.offset(along + 2.0, side * edge), 1.0)),
                    );
                }
                along += 4.0;
            }
            // Cues stood on end, each with a giant ball on top
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
                    &track,
                    &trunk,
                    Transform::from_translation(flat(p, 3.0)),
                );
                put(
                    &mut commands,
                    &track,
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
                        &track,
                        &stool,
                        Transform::from_translation(flat(p, 1.2))
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    );
                } else {
                    put(
                        &mut commands,
                        &track,
                        &ball,
                        Transform::from_translation(flat(p, 0.7))
                            .with_scale(Vec3::new(1.2, 0.8, 1.2)),
                    );
                }
            }
            if let Some(at) = track.landmark {
                put(
                    &mut commands,
                    &track,
                    &jukebox,
                    Transform::from_translation(flat(at, 5.0)),
                );
                for side in [-1.0f32, 1.0] {
                    put(
                        &mut commands,
                        &track,
                        &roof,
                        Transform::from_translation(flat(at, 12.4) + Vec3::X * side * 4.3)
                            .with_rotation(Quat::from_rotation_z(side * -0.6)),
                    );
                }
            }
        }
        Scenery::Tournament => {
            let plate = kit(m, mt, Cylinder::new(9.0, 0.4), Color::srgb(0.90, 0.94, 1.0));
            let rim = kit(m, mt, Torus::new(8.0, 9.4), Color::srgb(0.85, 0.70, 0.30));
            let roast = kit(m, mt, Sphere::new(14.0), Color::srgb(0.03, 0.03, 0.04));
            let drum = kit(
                m,
                mt,
                Capsule3d::new(3.5, 10.0),
                Color::srgb(0.75, 0.50, 0.25),
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
            let pea = kit(m, mt, Sphere::new(0.8), Color::srgb(0.95, 0.85, 0.15));
            let roll = kit(m, mt, Sphere::new(1.8), Color::srgb(0.80, 0.12, 0.12));

            // The 8-ball sits in the middle of the table
            if let Some(at) = track.landmark {
                put(
                    &mut commands,
                    &track,
                    &roast,
                    Transform::from_translation(flat(at, 4.0)).with_scale(Vec3::new(1.3, 0.7, 1.0)),
                );
                for side in [-1.0f32, 1.0] {
                    put(
                        &mut commands,
                        &track,
                        &drum,
                        Transform::from_translation(
                            flat(at, 5.0) + Vec3::new(side * 16.0, 0.0, 8.0),
                        )
                        .with_rotation(
                            Quat::from_rotation_x(1.2) * Quat::from_rotation_z(side * 0.4),
                        ),
                    );
                }
            }
            // Chalk circles lie wherever the layout motes them room
            let open = |x: f32, z: f32, r: f32| {
                track.locate(Vec2::new(x, z)).lateral.abs() > track.barrier() + r
            };
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
                if !open(x, z, 9.5) {
                    continue;
                }
                put(
                    &mut commands,
                    &track,
                    &plate,
                    Transform::from_xyz(x, 0.2, z),
                );
                put(&mut commands, &track, &rim, Transform::from_xyz(x, 0.35, z));
            }
            for (x, z) in [
                (-130.0, -60.0),
                (130.0, -60.0),
                (-130.0, 80.0),
                (130.0, 80.0),
            ] {
                if !open(x, z, 1.5) {
                    continue;
                }
                put(
                    &mut commands,
                    &track,
                    &candle,
                    Transform::from_xyz(x, 6.0, z),
                );
                put(
                    &mut commands,
                    &track,
                    &flame,
                    Transform::from_xyz(x, 12.8, z).with_scale(Vec3::new(1.0, 1.8, 1.0)),
                );
            }
            // Balls spilled along the rail
            let mut along = 0.0;
            while along < lap {
                for side in [-1.0, 1.0] {
                    let p = track.offset(along, side * edge);
                    match (along as i32 / 6) % 4 {
                        0 => put(
                            &mut commands,
                            &track,
                            &roll,
                            Transform::from_translation(flat(p, 1.2))
                                .with_scale(Vec3::new(1.0, 0.7, 1.4)),
                        ),
                        _ => put(
                            &mut commands,
                            &track,
                            &pea,
                            Transform::from_translation(flat(p, 0.8)),
                        ),
                    }
                }
                along += 6.0;
            }
        }
        Scenery::Neon => {
            let glow = |m: &mut Assets<Mesh>,
                        mt: &mut Assets<StandardMaterial>,
                        mesh: Mesh,
                        color: Color| Kit {
                mesh: m.add(mesh),
                material: mt.add(StandardMaterial {
                    base_color: color,
                    emissive: color.to_linear() * 6.0,
                    ..default()
                }),
            };
            let stalk = glow(
                m,
                mt,
                Cylinder::new(0.18, 5.5).into(),
                Color::srgb(0.15, 0.95, 0.90),
            );
            let cob = kit(m, mt, Sphere::new(0.4), Color::srgb(0.95, 0.80, 0.25));
            let tassel = glow(
                m,
                mt,
                Cone::new(0.5, 1.2).into(),
                Color::srgb(0.95, 0.15, 0.70),
            );
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

            // Dense rows of glowing neon tubes hem the road in
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
                            &track,
                            &stalk,
                            Transform::from_translation(flat(p, 2.75 * h))
                                .with_scale(Vec3::new(1.0, h, 1.0)),
                        );
                        put(
                            &mut commands,
                            &track,
                            &tassel,
                            Transform::from_translation(flat(p, 5.5 * h + 0.4)),
                        );
                        if rng.random_bool(0.3) {
                            put(
                                &mut commands,
                                &track,
                                &cob,
                                Transform::from_translation(flat(p, 3.0 * h) + Vec3::X * 0.3)
                                    .with_rotation(Quat::from_rotation_z(-0.5)),
                            );
                        }
                    }
                }
                along += 1.4;
            }
            // Pool-hall regulars keep watch
            for frac in [0.1, 0.35, 0.6, 0.85] {
                let p = track.offset(lap * frac, -(hw + 3.5));
                put(
                    &mut commands,
                    &track,
                    &pole,
                    Transform::from_translation(flat(p, 3.0)),
                );
                put(
                    &mut commands,
                    &track,
                    &shirt,
                    Transform::from_translation(flat(p, 4.0)),
                );
                put(
                    &mut commands,
                    &track,
                    &head,
                    Transform::from_translation(flat(p, 5.6)),
                );
                put(
                    &mut commands,
                    &track,
                    &hat,
                    Transform::from_translation(flat(p, 6.5)),
                );
            }
        }
    }

    // Chalk dust around the camera
    let dust_mesh = m.add(Rectangle::new(0.25, 0.25));
    let colors = [
        Color::srgb(0.95, 0.96, 1.0),
        Color::srgb(0.55, 0.72, 0.98),
        Color::srgb(0.85, 0.90, 0.98),
        Color::srgb(0.35, 0.55, 0.90),
    ];
    let dust_mats: Vec<_> = colors
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
            Dust {
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
            Mesh3d(dust_mesh.clone()),
            MeshMaterial3d(dust_mats[i % dust_mats.len()].clone()),
            Transform::from_xyz(
                rng.random_range(-40.0..40.0),
                rng.random_range(0.0..25.0),
                rng.random_range(-40.0..40.0),
            ),
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
