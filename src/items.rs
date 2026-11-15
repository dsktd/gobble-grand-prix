//! Harvest crates and what comes out of them.

use bevy::prelude::*;
use rand::Rng;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Item;
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn.after(spawn_track));
    app.add_systems(
        Update,
        (crates, pickup, fire, roll, slicks)
            .chain()
            .after(KartSet::Physics)
            .before(KartSet::Race)
            .run_if(on_track),
    );
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Item {
    /// A rolling cranberry that homes in on the kart ahead.
    Cranberry,
    /// A puddle of gravy dropped behind, spins out whoever drives through.
    Gravy,
    /// A burst of stuffing-powered speed.
    Stuffing,
}
impl Item {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Cranberry => "Cranberry",
            Self::Gravy => "Gravy Slick",
            Self::Stuffing => "Stuffing Boost",
        }
    }

    pub const fn color(&self) -> Color {
        match self {
            Self::Cranberry => Color::srgb(0.75, 0.05, 0.15),
            Self::Gravy => Color::srgb(0.50, 0.30, 0.12),
            Self::Stuffing => Color::srgb(0.85, 0.65, 0.30),
        }
    }

    /// Leaders get defence, the back of the pack gets speed.
    fn roll(place: usize) -> Self {
        let mut rng = rand::rng();
        let back = (place as f32 - 1.0) / 5.0;
        let r: f32 = rng.random();
        if r < 0.15 + back * 0.45 {
            Self::Stuffing
        } else if r < 0.6 + back * 0.2 {
            Self::Cranberry
        } else {
            Self::Gravy
        }
    }
}

/// A crate on the road. Respawns a few seconds after being taken.
#[derive(Component, Copy, Clone, Debug)]
pub struct Crate {
    pub pos: Vec2,
    pub hidden: f32,
}

#[derive(Component, Copy, Clone, Debug)]
pub struct Cranberry {
    pub vel: Vec2,
    pub owner: Entity,
    pub life: f32,
}

#[derive(Component, Copy, Clone, Debug)]
pub struct Slick {
    pub life: f32,
}

#[derive(Resource, Clone, Debug)]
struct Looks {
    berry_mesh: Handle<Mesh>,
    berry: Handle<StandardMaterial>,
    slick_mesh: Handle<Mesh>,
    slick: Handle<StandardMaterial>,
}

pub(super) fn spawn(
    mut commands: Commands,
    track: Res<Track>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let crate_mesh = meshes.add(Cuboid::new(1.6, 1.6, 1.6));
    let crate_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.60, 0.15, 0.8),
        emissive: LinearRgba::rgb(0.6, 0.25, 0.0),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let band_mesh = meshes.add(Cuboid::new(1.7, 0.25, 1.7));
    let band_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.25, 0.10),
        ..default()
    });
    for frac in &track.boxes {
        let along = track.length * frac;
        for lane in [-0.6, -0.2, 0.2, 0.6] {
            let pos = track.offset(along, lane * track.half_width);
            commands
                .spawn((
                    Crate { pos, hidden: 0.0 },
                    Mesh3d(crate_mesh.clone()),
                    MeshMaterial3d(crate_mat.clone()),
                    Transform::from_translation(flat(
                        pos,
                        track.surface(along, lane * track.half_width) + 1.3,
                    )),
                    DespawnOnExit(GameState::Race),
                ))
                .with_child((
                    Mesh3d(band_mesh.clone()),
                    MeshMaterial3d(band_mat.clone()),
                    Transform::default(),
                ));
        }
    }

    commands.insert_resource(Looks {
        berry_mesh: meshes.add(Sphere::new(0.6)),
        berry: materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.02, 0.12),
            perceptual_roughness: 0.25,
            ..default()
        }),
        slick_mesh: meshes.add(Cylinder::new(1.8, 0.06)),
        slick: materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.25, 0.08),
            perceptual_roughness: 0.1,
            reflectance: 0.8,
            ..default()
        }),
    });
}

fn crates(
    time: Res<Time>,
    crates: Query<(&mut Crate, &mut Transform, &mut Visibility)>,
) {
    let t = time.elapsed_secs();
    for (mut item_crate, mut transform, mut visibility) in crates {
        item_crate.hidden = (item_crate.hidden - time.delta_secs()).max(0.0);
        *visibility = if item_crate.hidden > 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        transform.translation.y = 1.3 + (t * 2.0 + item_crate.pos.x).sin() * 0.2;
        transform.rotation = Quat::from_rotation_y(t * 1.5) * Quat::from_rotation_x(0.4);
    }
}

fn pickup(
    crates: Query<&mut Crate>,
    karts: Query<(&mut Kart, &Progress, Has<Human>)>,
    mut sfx: MessageWriter<Sfx>,
) {
    let mut crates = crates;
    for (mut kart, progress, human) in karts {
        for mut item_crate in &mut crates {
            if item_crate.hidden > 0.0 || item_crate.pos.distance(kart.pos) > 2.2 {
                continue;
            }
            item_crate.hidden = 3.0;
            if kart.item.is_none() {
                kart.item = Some(Item::roll(progress.place));
                if human {
                    sfx.write(Sfx::Pickup);
                }
            }
        }
    }
}

fn fire(
    mut commands: Commands,
    looks: Res<Looks>,
    karts: Query<(Entity, &mut Kart, &KartInput, Has<Human>)>,
    mut sfx: MessageWriter<Sfx>,
) {
    for (id, mut kart, input, human) in karts {
        if !input.item || kart.spin > 0.0 || kart.frozen {
            continue;
        }
        let Some(item) = kart.item.take() else {
            continue;
        };
        let fwd = kart.forward();
        match item {
            Item::Cranberry => {
                let pos = kart.pos + fwd * 2.5;
                commands.spawn((
                    Cranberry {
                        vel: fwd * (kart.speed.max(0.0) + 30.0),
                        owner: id,
                        life: 5.0,
                    },
                    Mesh3d(looks.berry_mesh.clone()),
                    MeshMaterial3d(looks.berry.clone()),
                    Transform::from_translation(flat(pos, kart.y + 0.6)),
                    DespawnOnExit(GameState::Race),
                ));
                if human {
                    sfx.write(Sfx::Throw);
                }
            }
            Item::Gravy => {
                let pos = kart.pos - fwd * 3.0;
                commands.spawn((
                    Slick { life: 30.0 },
                    Mesh3d(looks.slick_mesh.clone()),
                    MeshMaterial3d(looks.slick.clone()),
                    Transform::from_translation(flat(pos, kart.ground + 0.06)),
                    DespawnOnExit(GameState::Race),
                ));
                if human {
                    sfx.write(Sfx::Splat);
                }
            }
            Item::Stuffing => {
                kart.boost = kart.boost.max(1.6);
                if human {
                    sfx.write(Sfx::Boost);
                }
            }
        }
    }
}

fn roll(
    mut commands: Commands,
    time: Res<Time>,
    track: Res<Track>,
    berries: Query<(Entity, &mut Cranberry, &mut Transform)>,
    mut karts: Query<(Entity, &mut Kart, Has<Human>)>,
    mut sfx: MessageWriter<Sfx>,
) {
    let dt = time.delta_secs();
    for (berry_id, mut berry, mut transform) in berries {
        berry.life -= dt;
        let pos = Vec2::new(transform.translation.x, transform.translation.z);
        let y = transform.translation.y - 0.6;

        // Home in on the nearest kart in front
        let dir = berry.vel.normalize_or_zero();
        let target = karts
            .iter()
            .filter(|(id, kart, _)| {
                *id != berry.owner
                    && kart.pos.distance(pos) < 35.0
                    && (kart.pos - pos).normalize_or_zero().dot(dir) > 0.5
            })
            .min_by(|a, b| a.1.pos.distance(pos).total_cmp(&b.1.pos.distance(pos)))
            .map(|(_, kart, _)| kart.pos);
        let speed = berry.vel.length();
        if let Some(target) = target {
            let want = (target - pos).normalize_or_zero() * speed;
            berry.vel = berry.vel.lerp(want, (dt * 3.0).min(1.0));
        } else {
            // Otherwise follow the road
            let spot = track.find(pos, y);
            let (_, road) = track.sample(spot.along + 4.0);
            let want = road * speed - road.perp() * spot.lateral * 0.5;
            berry.vel = berry.vel.lerp(want, (dt * 1.5).min(1.0));
        }
        let pos = pos + berry.vel * dt;
        transform.translation = flat(pos, track.ground_near(pos, y) + 0.6);
        transform.rotate_local_x(-speed * dt / 0.6);

        let mut hit = false;
        for (id, mut kart, human) in &mut karts {
            if id == berry.owner && berry.life > 4.6 {
                continue;
            }
            // Same level only: not through a bridge
            if kart.pos.distance(pos) < 1.8 && (kart.y - y).abs() < 2.5 {
                kart.hit(1.1);
                hit = true;
                if human {
                    sfx.write(Sfx::Hit);
                }
                break;
            }
        }
        if hit || berry.life <= 0.0 {
            commands.entity(berry_id).despawn();
        }
    }
}

fn slicks(
    mut commands: Commands,
    time: Res<Time>,
    slicks: Query<(Entity, &mut Slick, &Transform)>,
    mut karts: Query<(&mut Kart, Has<Human>)>,
    mut sfx: MessageWriter<Sfx>,
) {
    for (id, mut slick, transform) in slicks {
        slick.life -= time.delta_secs();
        let pos = Vec2::new(transform.translation.x, transform.translation.z);
        let mut hit = false;
        for (mut kart, human) in &mut karts {
            if kart.pos.distance(pos) < 2.4
                && (kart.y - transform.translation.y).abs() < 2.5
                && kart.spin <= 0.0
            {
                kart.hit(0.9);
                hit = true;
                if human {
                    sfx.write(Sfx::Hit);
                }
                break;
            }
        }
        if hit || slick.life <= 0.0 {
            commands.entity(id).despawn();
        }
    }
}
