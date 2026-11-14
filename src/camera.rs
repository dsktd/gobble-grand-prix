//! One chase camera per human seat.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::ChaseCam;
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn.after(spawn_track));
    app.add_systems(
        Update,
        chase
            .after(KartSet::Physics)
            .run_if(in_state(GameState::Race)),
    );
}

/// Follows the kart of the human in this seat.
#[derive(Component, Copy, Clone, Debug)]
pub struct ChaseCam(pub usize);

fn spawn(
    mut commands: Commands,
    setup: Res<Setup>,
    track: Res<Track>,
    options: Res<Options>,
) {
    let theme = track.def().theme;
    let (start, dir) = track.sample(track.length - 20.0);
    for slot in 0..setup.racers.len() {
        let mut camera = commands.spawn((
            ChaseCam(slot),
            Camera3d::default(),
            Camera {
                order: slot as isize,
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 70f32.to_radians(),
                ..default()
            }),
            Transform::from_translation(flat(start - dir * 10.0, 4.0))
                .looking_at(flat(start, 1.0), Vec3::Y),
            DespawnOnExit(GameState::Race),
        ));
        if !options.high_quality {
            camera.insert(bevy::render::view::Msaa::Off);
        }
        if options.high_quality {
            camera.insert(DistanceFog {
                color: theme.fog,
                directional_light_color: Color::NONE,
                directional_light_exponent: 8.0,
                falloff: FogFalloff::Linear {
                    start: theme.fog_end * 0.4,
                    end: theme.fog_end,
                },
            });
        }
    }
}

fn chase(
    time: Res<Time>,
    karts: Query<(&Kart, &Human)>,
    cameras: Query<(&ChaseCam, &mut Transform, &mut Projection)>,
) {
    let dt = time.delta_secs();
    for (cam, mut transform, mut projection) in cameras {
        let Some((kart, _)) = karts.iter().find(|(_, human)| human.0 == cam.0) else {
            continue;
        };
        let fwd = flat(kart.forward(), 0.0);
        let pos = flat(kart.pos, 0.0);
        let want = pos - fwd * 8.0 + Vec3::Y * 3.6;
        let ease = 1.0 - (-6.0 * dt).exp();
        transform.translation = transform.translation.lerp(want, ease);
        let look = pos + fwd * 4.0 + Vec3::Y * 1.2;
        transform.look_at(look, Vec3::Y);

        // Widen the view while boosting
        if let Projection::Perspective(perspective) = projection.as_mut() {
            let fov = match kart.boost > 0.0 {
                true => 82f32,
                false => 70f32,
            }
            .to_radians();
            perspective.fov += (fov - perspective.fov) * ease;
        }
    }
}
