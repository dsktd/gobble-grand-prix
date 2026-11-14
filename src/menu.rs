//! Title menu, options, and racer and track pickers. Pad and keyboard alike.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_computed_state::<InMenus>();
    app.init_resource::<Cursor>();
    app.add_systems(OnEnter(InMenus), backdrop);
    for state in [
        GameState::Menu,
        GameState::Options,
        GameState::PickRacer,
        GameState::PickTrack,
    ] {
        app.add_systems(OnEnter(state), screen);
    }
    app.add_systems(
        Update,
        (navigate, lines, preview, spin)
            .chain()
            .run_if(in_state(InMenus)),
    );
    app.add_systems(Update, invite.run_if(in_state(GameState::Menu)));
}

/// Every screen but the race itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct InMenus;
impl ComputedStates for InMenus {
    type SourceStates = GameState;

    fn compute(state: GameState) -> Option<Self> {
        match state {
            GameState::Race => None,
            _ => Some(InMenus),
        }
    }
}

/// Highlighted row on the current screen.
#[derive(Resource, Copy, Clone, Debug, Default)]
struct Cursor(usize);

#[derive(Component, Copy, Clone, Debug)]
struct Line(usize);

/// The racer model on the menu platter.
#[derive(Component, Copy, Clone, Debug)]
struct Preview(usize);

#[derive(Component, Copy, Clone, Debug)]
struct Platter;

/// Controls listed on the options screen.
const CONTROLS: [(Action, Option<Action>, &str); 6] = [
    (Action::Gas, Some(Action::GasButton), "Gas"),
    (Action::Brake, Some(Action::BrakeButton), "Brake"),
    (Action::Steer, None, "Steer"),
    (
        Action::Drift,
        None,
        "Drift (hold, then release for a turbo)",
    ),
    (Action::Item, None, "Use item"),
    (Action::Quit, None, "Quit race"),
];

fn header(state: GameState) -> &'static str {
    match state {
        GameState::Menu => "GOBBLE GRAND PRIX",
        GameState::Options => "Options",
        GameState::PickRacer => "Pick your racer",
        GameState::PickTrack => "Pick a track",
        GameState::Race => "",
    }
}

fn rows(
    state: GameState,
    setup: &Setup,
    options: &Options,
) -> Vec<String> {
    match state {
        GameState::Menu => vec!["Start Race".into(), "Options".into(), "Exit".into()],
        GameState::Options => vec![
            format!("Volume   <  {}  >", options.volume),
            format!(
                "Quality  <  {}  >",
                match options.high_quality {
                    true => "High",
                    false => "Low",
                }
            ),
            format!("Camera smoothing  <  {}  >", options.smoothing),
            format!(
                "Camera shake  <  {}  >",
                match options.shake {
                    true => "On",
                    false => "Off",
                }
            ),
            "Back".into(),
        ],
        GameState::PickRacer => {
            let racer = &ROSTER[setup.racers[0]];
            let bar = |v: f32| "#".repeat(((v - 0.7) * 20.0).round().max(1.0) as usize);
            vec![
                format!("<  {}  >", racer.name),
                racer.blurb.into(),
                format!(
                    "Speed {}   Accel {}   Turn {}",
                    bar(racer.stats.top_speed),
                    bar(racer.stats.accel),
                    bar(racer.stats.turn),
                ),
            ]
        }
        GameState::PickTrack => {
            let track = &TRACKS[setup.track];
            vec![format!("<  {}  >", track.name), track.blurb.into()]
        }
        GameState::Race => vec![],
    }
}

/// Rows the cursor can stop on.
fn selectable(state: GameState) -> usize {
    match state {
        GameState::Menu => 3,
        GameState::Options => 5,
        _ => 1,
    }
}

fn backdrop(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scope = DespawnOnExit(InMenus);
    commands.insert_resource(ClearColor(Tone::MENU_BG));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.8, 0.6),
        brightness: 500.0,
        ..default()
    });
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-3.2, 3.0, -6.5).looking_at(Vec3::new(-1.6, 1.0, 0.0), Vec3::Y),
        scope.clone(),
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.8, 0.6),
            illuminance: 8000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(0.5, -0.7, 0.6), Vec3::Y),
        scope.clone(),
    ));
    // A dinner plate to stand on
    commands.spawn((
        Platter,
        Mesh3d(meshes.add(Cylinder::new(2.6, 0.15))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Tone::CREAM,
            perceptual_roughness: 0.3,
            ..default()
        })),
        Transform::default(),
        scope.clone(),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(2.4, 2.75))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Tone::GOLD,
            metallic: 0.6,
            perceptual_roughness: 0.3,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.08, 0.0),
        scope,
    ));
}

fn screen(
    mut commands: Commands,
    state: Res<State<GameState>>,
    setup: Res<Setup>,
    options: Res<Options>,
    mut cursor: ResMut<Cursor>,
) {
    let state = *state.get();
    cursor.0 = 0;
    let count = rows(state, &setup, &options).len();
    let big = matches!(state, GameState::Menu | GameState::Options);
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::End,
                padding: UiRect::all(px(Size::PAD * 1.5)),
                ..default()
            },
            DespawnOnExit(state),
        ))
        .with_children(|root| {
            let size = match state {
                GameState::Menu => Size::TITLE,
                _ => Size::HEADING,
            };
            root.spawn(label(header(state), size, Tone::GOLD));
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                row_gap: px(Size::GAP * 1.5),
                max_width: percent(50),
                ..default()
            })
            .with_children(|list| {
                for i in 0..count {
                    let size = match (i, big) {
                        (0, _) | (_, true) => Size::HEADING,
                        _ => Size::BODY,
                    };
                    list.spawn((
                        Line(i),
                        label("", size, Tone::CREAM),
                        TextLayout::justify(Justify::Right),
                    ));
                }
                if state == GameState::Options {
                    list.spawn(panel()).with_children(|controls| {
                        for (action, alternative, text) in CONTROLS {
                            match alternative {
                                Some(alternative) => {
                                    controls.spawn(prompt_either(action, alternative, text));
                                }
                                None => {
                                    controls.spawn(prompt(action, text));
                                }
                            }
                        }
                    });
                }
            });
            root.spawn(Node {
                column_gap: px(Size::PAD),
                ..default()
            })
            .with_children(|hints| {
                hints.spawn(prompt(Action::Navigate, "Choose"));
                hints.spawn(prompt(Action::Confirm, "OK"));
                hints.spawn(prompt(Action::Back, "Back"));
            });
        });
}

/// Marks the phone invite so it is spawned once per visit to the title menu.
#[derive(Component, Copy, Clone, Debug)]
struct Invite;

/// The phone controller QR, bottom left of the title menu, once the server is up.
fn invite(
    mut commands: Commands,
    join: Option<Res<JoinInfo>>,
    shown: Query<(), With<Invite>>,
) {
    let Some(join) = join else {
        return;
    };
    if !shown.is_empty() {
        return;
    }
    let state = GameState::Menu;
    // Phone controller invite, bottom left
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(Size::PAD * 1.5),
                bottom: px(Size::PAD * 1.5),
                column_gap: px(Size::GAP * 1.5),
                align_items: AlignItems::Center,
                ..default()
            },
            Invite,
            DespawnOnExit(state),
        ))
        .with_children(|invite| {
            invite.spawn((
                ImageNode::new(join.qr.clone()),
                Node {
                    width: px(150),
                    height: px(150),
                    ..default()
                },
            ));
            invite
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|text| {
                    text.spawn(label("Phone controller", Size::BODY, Tone::GOLD));
                    text.spawn(label(format!("PIN {}", join.pin), Size::BODY, Tone::CREAM));
                    text.spawn(label(join.url.clone(), Size::SMALL, Tone::DIM));
                    text.spawn(label(
                        "Accept the certificate warning once",
                        Size::SMALL,
                        Tone::DIM,
                    ));
                });
        });
}

fn navigate(
    input: Res<MenuInput>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut cursor: ResMut<Cursor>,
    mut setup: ResMut<Setup>,
    mut options: ResMut<Options>,
    mut exit: MessageWriter<AppExit>,
    mut sfx: MessageWriter<Sfx>,
) {
    let state = *state.get();
    let n = selectable(state);
    let input = *input;
    if input.up || input.down || input.left || input.right || input.confirm || input.back {
        sfx.write(Sfx::Click);
    }
    if input.up {
        cursor.0 = (cursor.0 + n - 1) % n;
    }
    if input.down {
        cursor.0 = (cursor.0 + 1) % n;
    }
    let step = input.right as i32 - input.left as i32;
    match state {
        GameState::Menu => {
            if input.confirm {
                match cursor.0 {
                    0 => next.set(GameState::PickRacer),
                    1 => next.set(GameState::Options),
                    _ => {
                        exit.write(AppExit::Success);
                    }
                }
            }
            if input.back {
                exit.write(AppExit::Success);
            }
        }
        GameState::Options => {
            match cursor.0 {
                0 if step != 0 => {
                    options.volume = (options.volume as i32 + step).clamp(0, 10) as u8;
                }
                1 if step != 0 => options.high_quality = !options.high_quality,
                2 if step != 0 => {
                    options.smoothing = (options.smoothing as i32 + step).clamp(0, 10) as u8;
                }
                3 if step != 0 => options.shake = !options.shake,
                _ => {}
            }
            if input.back || (input.confirm && cursor.0 == 4) {
                next.set(GameState::Menu);
            }
        }
        GameState::PickRacer => {
            if step != 0 {
                let n = ROSTER.len() as i32;
                setup.racers[0] = (setup.racers[0] as i32 + step).rem_euclid(n) as usize;
            }
            if input.confirm {
                next.set(GameState::PickTrack);
            }
            if input.back {
                next.set(GameState::Menu);
            }
        }
        GameState::PickTrack => {
            if step != 0 {
                let n = TRACKS.len() as i32;
                setup.track = (setup.track as i32 + step).rem_euclid(n) as usize;
            }
            if input.confirm {
                next.set(GameState::Race);
            }
            if input.back {
                next.set(GameState::PickRacer);
            }
        }
        GameState::Race => {}
    }
}

fn lines(
    state: Res<State<GameState>>,
    setup: Res<Setup>,
    options: Res<Options>,
    cursor: Res<Cursor>,
    texts: Query<(&Line, &mut Text, &mut TextColor)>,
) {
    let state = *state.get();
    let rows = rows(state, &setup, &options);
    let pickable = selectable(state) > 1;
    for (line, mut text, mut color) in texts {
        let Some(row) = rows.get(line.0) else {
            continue;
        };
        let on = line.0 == cursor.0 && (pickable || line.0 == 0);
        let shown = match (on, pickable) {
            (true, true) => format!("> {row}"),
            _ => row.clone(),
        };
        if text.0 != shown {
            text.0 = shown;
        }
        color.0 = match on {
            true => Tone::GOLD,
            false => Tone::CREAM,
        };
    }
}

fn preview(
    mut commands: Commands,
    setup: Res<Setup>,
    current: Query<(Entity, &Preview)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let racer = setup.racers[0];
    if current.iter().any(|(_, p)| p.0 == racer) {
        return;
    }
    for (id, _) in &current {
        commands.entity(id).despawn();
    }
    let root = commands
        .spawn((
            Preview(racer),
            Transform::from_xyz(0.0, 0.1, 0.0),
            Visibility::default(),
            DespawnOnExit(InMenus),
        ))
        .id();
    let mut parts = Parts {
        meshes: &mut meshes,
        materials: &mut materials,
    };
    spawn_model(&mut commands, &mut parts, root, &ROSTER[racer]);
}

fn spin(
    time: Res<Time>,
    previews: Query<&mut Transform, Or<(With<Preview>, With<Platter>)>>,
) {
    for mut transform in previews {
        // Swing to and fro so the racer's face stays in view
        transform.rotation = Quat::from_rotation_y((time.elapsed_secs() * 0.7).sin() * 1.0 - 0.3);
    }
}
