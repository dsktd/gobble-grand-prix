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
        GameState::Lobby,
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
    app.init_resource::<Ready>();
    app.add_systems(OnEnter(GameState::Lobby), |mut seats: ResMut<Seats>| {
        seats.0.clear();
    });
    app.add_systems(OnEnter(GameState::PickRacer), |mut ready: ResMut<Ready>| {
        ready.0.clear();
    });
    app.add_systems(
        Update,
        (
            lobby.run_if(in_state(GameState::Lobby)),
            pick.run_if(in_state(GameState::PickRacer)),
        )
            .before(navigate),
    );
}

/// Every screen but the race itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct InMenus;
impl ComputedStates for InMenus {
    type SourceStates = GameState;

    fn compute(state: GameState) -> Option<Self> {
        match state {
            GameState::Race => None,
            _ => Some(Self),
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

const fn header(state: GameState) -> &'static str {
    match state {
        GameState::Menu => "GOBBLE GRAND PRIX",
        GameState::Options => "Options",
        GameState::Lobby => "Join in",
        GameState::PickRacer => "Pick your racer",
        GameState::PickTrack => "Pick a track",
        GameState::Race => "",
    }
}

const fn device_name(device: Device) -> &'static str {
    match device {
        Device::Any => "Any controller",
        Device::Keyboard => "Keyboard",
        Device::Gamepad(_) => "Controller",
        Device::Phone(_) => "Phone",
        Device::Bot => "CPU (test seat)",
    }
}

fn rows(
    state: GameState,
    setup: &Setup,
    options: &Options,
    seats: &Seats,
    ready: &Ready,
) -> Vec<String> {
    match state {
        GameState::Lobby => {
            let mut rows: Vec<String> = (0..4)
                .map(|slot| {
                    seats.0.get(slot).map_or_else(
                        || format!("P{}  press A or Enter to join", slot + 1),
                        |device| format!("P{}  {}", slot + 1, device_name(*device)),
                    )
                })
                .collect();
            rows.push(if seats.0.is_empty() {
                "B / Esc: back to menu".into()
            } else {
                "Joined players press A / Enter again to start".into()
            });
            rows
        }
        GameState::PickRacer if setup.racers.len() > 1 => setup
            .racers
            .iter()
            .enumerate()
            .map(|(slot, racer)| {
                let done = if ready.0.get(slot).copied().unwrap_or(false) {
                    "  READY"
                } else {
                    ""
                };
                format!("P{}  <  {}  >{done}", slot + 1, ROSTER[*racer].name)
            })
            .collect(),
        GameState::Menu => vec!["Start Race".into(), "Options".into(), "Exit".into()],
        GameState::Options => vec![
            format!("Volume   <  {}  >", options.volume),
            format!("Music   <  {}  >", options.music),
            format!(
                "Quality  <  {}  >",
                if options.high_quality { "High" } else { "Low" }
            ),
            format!("Camera smoothing  <  {}  >", options.smoothing),
            format!(
                "Camera shake  <  {}  >",
                if options.shake { "On" } else { "Off" }
            ),
            format!(
                "2 players  <  {}  >",
                if options.side_by_side {
                    "Side by side"
                } else {
                    "Top / bottom"
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
const fn selectable(state: GameState) -> usize {
    match state {
        GameState::Menu => 3,
        GameState::Options => 7,
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
    seats: Res<Seats>,
    ready: Res<Ready>,
    mut cursor: ResMut<Cursor>,
) {
    let state = *state.get();
    cursor.0 = 0;
    let count = match state {
        // Seats can join after the screen is built
        GameState::Lobby => 5,
        GameState::PickRacer => 4.max(rows(state, &setup, &options, &seats, &ready).len()),
        _ => rows(state, &setup, &options, &seats, &ready).len(),
    };
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
                    let shown = match &join.anywhere {
                        Some(_) => format!("Any network, or same Wi-Fi: {}", join.url),
                        None => join.url.clone(),
                    };
                    text.spawn(label(shown, Size::SMALL, Tone::DIM));
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
    let step = i32::from(input.right) - i32::from(input.left);
    match state {
        GameState::Menu => {
            if input.confirm {
                match cursor.0 {
                    0 => next.set(GameState::Lobby),
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
                    options.volume = (i32::from(options.volume) + step).clamp(0, 10) as u8;
                }
                1 if step != 0 => {
                    options.music = (i32::from(options.music) + step).clamp(0, 10) as u8;
                }
                2 if step != 0 => options.high_quality = !options.high_quality,
                3 if step != 0 => {
                    options.smoothing = (i32::from(options.smoothing) + step).clamp(0, 10) as u8;
                }
                4 if step != 0 => options.shake = !options.shake,
                5 if step != 0 => options.side_by_side = !options.side_by_side,
                _ => {}
            }
            if input.back || (input.confirm && cursor.0 == 6) {
                next.set(GameState::Menu);
            }
        }
        GameState::Lobby | GameState::PickRacer => {}
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
    seats: Res<Seats>,
    ready: Res<Ready>,
    cursor: Res<Cursor>,
    texts: Query<(&Line, &mut Text, &mut TextColor)>,
) {
    let state = *state.get();
    let rows = rows(state, &setup, &options, &seats, &ready);
    let pickable = selectable(state) > 1;
    for (line, mut text, mut color) in texts {
        let row = rows.get(line.0).cloned().unwrap_or_default();
        let row = &row;
        let on = line.0 == cursor.0 && (pickable || line.0 == 0);
        let shown = match (on, pickable) {
            (true, true) => format!("> {row}"),
            _ => row.clone(),
        };
        if text.0 != shown {
            text.0 = shown;
        }
        color.0 = if on { Tone::GOLD } else { Tone::CREAM };
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

/// Which seats have locked in their racer.
#[derive(Resource, Clone, Debug, Default)]
struct Ready(Vec<bool>);

/// Pads and the keyboard take seats: A or Enter joins, then starts; B or Esc leaves.
fn lobby(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    phones: Res<Phones>,
    mut seats: ResMut<Seats>,
    mut setup: ResMut<Setup>,
    mut next: ResMut<NextState<GameState>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let mut pressed: Vec<(Device, bool, bool)> = gamepads
        .iter()
        .map(|(id, g)| {
            (
                Device::Gamepad(id),
                g.just_pressed(GamepadButton::South) || g.just_pressed(GamepadButton::Start),
                g.just_pressed(GamepadButton::East),
            )
        })
        .collect();
    for (id, pad) in &phones.pads {
        pressed.push((Device::Phone(*id), pad.menu.confirm, pad.menu.back));
    }
    pressed.push((
        Device::Keyboard,
        keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space),
        keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Backspace),
    ));

    for (device, join, leave) in pressed {
        let seated = seats.0.contains(&device);
        if join && !seated && seats.0.len() < 4 {
            seats.0.push(device);
            sfx.write(Sfx::Pickup);
        } else if join && !seated {
            if let Device::Phone(id) = device {
                phones.send(format!(r#"{{"t":"full","id":{id}}}"#));
            }
        } else if join && seated {
            // Start with everyone who joined; a lone player may use any device
            if seats.0.len() == 1 && !matches!(seats.0[0], Device::Phone(_)) {
                seats.0[0] = Device::Any;
            }
            let n = seats.0.len();
            let first = setup.racers.first().copied().unwrap_or(0);
            setup.racers = (0..n).map(|slot| (first + slot) % ROSTER.len()).collect();
            next.set(GameState::PickRacer);
            return;
        } else if leave && seated {
            seats.0.retain(|d| *d != device);
            sfx.write(Sfx::Click);
        } else if leave && seats.0.is_empty() {
            next.set(GameState::Menu);
            return;
        }
    }
}

/// Every seat picks its own racer and readies up; all ready moves on to the track.
fn pick(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    phones: Res<Phones>,
    all: Res<MenuInput>,
    seats: Res<Seats>,
    mut ready: ResMut<Ready>,
    mut setup: ResMut<Setup>,
    mut next: ResMut<NextState<GameState>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let n = setup.racers.len();
    ready.0.resize(n, false);
    for slot in 0..n {
        let device = seats.0.get(slot).copied().unwrap_or(Device::Any);
        if device == Device::Bot {
            ready.0[slot] = true;
            continue;
        }
        let input = device_menu(device, &keys, &gamepads, &phones, &all);
        let step = i32::from(input.right) - i32::from(input.left);
        if step != 0 && !ready.0[slot] {
            let count = ROSTER.len() as i32;
            setup.racers[slot] = (setup.racers[slot] as i32 + step).rem_euclid(count) as usize;
            sfx.write(Sfx::Click);
        }
        if input.confirm {
            ready.0[slot] = true;
            sfx.write(Sfx::Pickup);
        }
        if input.back {
            if ready.0[slot] {
                ready.0[slot] = false;
            } else {
                next.set(GameState::Lobby);
                return;
            }
        }
    }
    if ready.0.iter().all(|r| *r) {
        next.set(GameState::PickTrack);
    }
}
