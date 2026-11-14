//! Title menu, options, and racer and track pickers. Pad and keyboard alike.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::InMenus;
    pub use super::Whats;
}

/// The build's version line, from build.rs.
pub const VERSION: &str = env!("GOBBLE_VERSION");

/// CHANGELOG.md, embedded.
const CHANGELOG: &str = include_str!("../CHANGELOG.md");

/// Lines of "What's new" shown at most.
const NEWS_LINES: usize = 5;

/// The newest changes: the first few bullet lines of the changelog.
fn news() -> Vec<&'static str> {
    CHANGELOG
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
        .take(NEWS_LINES)
        .collect()
}

/// Whether the "What's new" box is open on the title screen.
#[derive(Resource, Copy, Clone, Debug, Default)]
pub struct Whats {
    pub open: bool,
}

#[derive(Component, Copy, Clone, Debug)]
struct NewsBox;

/// A new build opens the box on its first launch; the build line is stored to tell.
#[cfg(not(test))]
fn first_launch(mut whats: ResMut<Whats>) {
    let Some(path) = dirs::data_dir().map(|d| d.join("gobble-gp").join("last-seen")) else {
        return;
    };
    // The build time changes on every compile; the tag, count and sha say what it is
    let build = VERSION.rsplit_once(" \u{b7} ").map_or(VERSION, |(b, _)| b);
    if std::fs::read_to_string(&path).is_ok_and(|seen| seen.trim() == build) {
        return;
    }
    whats.open = true;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, build);
}

/// Y or T opens the box on the title screen; any of A, B, Y closes it.
fn whats(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut input: ResMut<MenuInput>,
    mut whats: ResMut<Whats>,
    mut boxes: Query<&mut Visibility, With<NewsBox>>,
) {
    let y = keys.just_pressed(KeyCode::KeyT)
        || gamepads
            .iter()
            .any(|g| g.just_pressed(GamepadButton::North));
    if whats.open {
        if y || input.confirm || input.back {
            whats.open = false;
        }
        // The box swallows the press
        *input = MenuInput::default();
    } else if y {
        whats.open = true;
    }
    for mut visibility in &mut boxes {
        *visibility = if whats.open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Whats>();
    #[cfg(not(test))]
    app.add_systems(Startup, first_launch);
    app.add_systems(
        PreUpdate,
        whats
            .after(bevy::input::InputSystems)
            .after(crate::input::menu)
            .run_if(in_state(GameState::Menu)),
    );
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
        (navigate, focus, bind, frame, preview, spin)
            .chain()
            .run_if(in_state(InMenus)),
    );
    app.add_systems(
        Update,
        late_invite.run_if(|s: Res<State<GameState>>| {
            matches!(s.get(), GameState::Menu | GameState::Lobby)
        }),
    );
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
    app.add_systems(
        Update,
        tell_picks
            .after(pick)
            .run_if(in_state(GameState::PickRacer)),
    );
    app.add_systems(OnExit(GameState::PickRacer), |phones: Res<Phones>| {
        phones.send(r#"{"t":"racers","list":null}"#.to_string());
    });
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

/// The racer model on the menu platter.
#[derive(Component, Copy, Clone, Debug)]
struct Preview(usize);

#[derive(Component, Copy, Clone, Debug)]
struct Platter;

/// Films the platter over the attract race, on racer select only.
#[derive(Component, Copy, Clone, Debug)]
struct PlatterCam;

/// The platter stands far off the track so the race camera never sees it.
const PODIUM: Vec3 = Vec3::new(5000.0, 0.0, 0.0);

/// Something on a menu screen that shows live data.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq)]
enum Bind {
    /// Value text of an option row.
    Value(Opt),
    /// Pip `n` of a slider row.
    Pip(Opt, u8),
    /// A lobby seat row's text and its colour.
    Seat(usize),
    LobbyHelp,
    RacerName,
    RacerBlurb,
    RacerStat(usize),
    RacerRow(usize),
    /// Seat `1`'s tag beside racer row `0`, shown while that seat's cursor is on it.
    RacerTag(usize, usize),
    SeatPlayer,
    SeatStrip,
}

/// The settings on the options screen, in row order.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Opt {
    Volume,
    Music,
    Quality,
    Smoothing,
    Shake,
    Sides,
}
impl Opt {
    const ALL: [Self; 6] = [
        Self::Volume,
        Self::Music,
        Self::Quality,
        Self::Smoothing,
        Self::Shake,
        Self::Sides,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Volume => "Volume",
            Self::Music => "Music",
            Self::Quality => "Quality",
            Self::Smoothing => "Camera smoothing",
            Self::Shake => "Camera shake",
            Self::Sides => "2 players",
        }
    }

    /// The 0..=10 level of a slider row.
    const fn level(
        self,
        options: &Options,
    ) -> Option<u8> {
        match self {
            Self::Volume => Some(options.volume),
            Self::Music => Some(options.music),
            Self::Smoothing => Some(options.smoothing),
            _ => None,
        }
    }

    fn value(
        self,
        options: &Options,
    ) -> String {
        match self {
            Self::Quality => if options.high_quality { "High" } else { "Low" }.into(),
            Self::Shake => if options.shake { "On" } else { "Off" }.into(),
            Self::Sides => match options.split {
                Split::TopBottom => "Top / bottom",
                Split::SideBySide => "Side by side",
                Split::Diagonal => "Diagonal",
            }
            .into(),
            _ => self.level(options).unwrap_or(0).to_string(),
        }
    }
}

/// The controls panel on the options screen: what each glyph does.
const CONTROLS: [(Action, &str); 8] = [
    (Action::Gas, "Gas"),
    (Action::GasButton, "Gas (button)"),
    (Action::Brake, "Brake"),
    (Action::BrakeButton, "Brake (button)"),
    (Action::Steer, "Steer"),
    (Action::Drift, "Drift: hold, release for a turbo"),
    (Action::Item, "Use item"),
    (Action::Quit, "Quit race"),
];

const fn device_name(device: Device) -> &'static str {
    match device {
        Device::Any => "Any controller",
        Device::Keyboard => "Keyboard",
        Device::Gamepad(_) => "Controller",
        Device::Phone(_) => "Phone",
        Device::Bot => "CPU (test seat)",
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

/// The 1280x800 design, centred in the window; everything inside is placed in spec px.
fn stage(
    commands: &mut Commands,
    state: GameState,
) -> Entity {
    // A vignette over the race behind; busier screens dim it more
    let (inner, outer) = match state {
        GameState::Menu | GameState::PickRacer => (0.15, 0.7),
        _ => (0.5, 0.8),
    };
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::FarthestCorner,
                vec![
                    ColorStop::percent(Tone::BROWN_900.with_alpha(inner), 30.0),
                    ColorStop::percent(Tone::BROWN_900.with_alpha(outer), 100.0),
                ],
            )),
            DespawnOnExit(state),
        ))
        .id()
}

fn page(
    commands: &mut Commands,
    state: GameState,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    let root = stage(commands, state);
    commands.entity(root).with_children(|root| {
        root.spawn(Node {
            width: px(1280),
            height: px(800),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(build);
    });
}

fn heading(
    stage: &mut ChildSpawnerCommands,
    value: &str,
    width: f32,
) {
    stage.spawn((
        boxed(48.0, 40.0, width, 48.0),
        children![fit_text(
            value,
            Size::H1,
            Weight::Bold,
            Tone::CREAM_100,
            width,
            1
        )],
    ));
}

/// A line of text in a fixed box, vertically centred, `inset` from the left (or centred).
fn caption(
    stage: &mut ChildSpawnerCommands,
    at: (f32, f32),
    size: (f32, f32),
    text: impl Bundle,
    centred: bool,
) -> Entity {
    stage
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: if centred {
                    JustifyContent::Center
                } else {
                    JustifyContent::Start
                },
                ..boxed(at.0, at.1, size.0, size.1)
            },
            children![text],
        ))
        .id()
}

fn button(
    stage: &mut ChildSpawnerCommands,
    slot: usize,
    at: (f32, f32),
    width: f32,
    value: &str,
) {
    stage
        .spawn((
            Slot(slot),
            Ring,
            Fill {
                idle: Tone::BROWN_700,
                focus: Tone::PUMPKIN,
            },
            BackgroundColor(Tone::BROWN_700),
            BorderColor::all(Color::NONE),
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(Space::FOCUS)),
                border_radius: pill(Space::RADIUS_M),
                ..boxed(at.0, at.1, width, 72.0)
            },
        ))
        .with_children(|b| {
            b.spawn((
                Slot(slot),
                Ink {
                    idle: Tone::CREAM_100,
                    focus: Tone::BROWN_900,
                },
                fit_text(
                    value,
                    Size::BODY,
                    Weight::Medium,
                    Tone::CREAM_100,
                    width - 48.0,
                    1,
                ),
            ));
        });
}

/// A list row (spec 360x64 style) with its text inset 24.
fn list_row(
    stage: &mut ChildSpawnerCommands,
    slot: usize,
    at: (f32, f32),
    size: (f32, f32),
    value: &str,
    extra: impl Bundle,
) {
    stage
        .spawn((
            Slot(slot),
            Fill {
                idle: Tone::BROWN_800,
                focus: Tone::PUMPKIN,
            },
            BackgroundColor(Tone::BROWN_800),
            Node {
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(Space::L)),
                border_radius: pill(Space::RADIUS_S),
                ..boxed(at.0, at.1, size.0, size.1)
            },
            extra,
        ))
        .with_children(|row| {
            row.spawn((
                Slot(slot),
                Ink {
                    idle: Tone::CREAM_100,
                    focus: Tone::BROWN_900,
                },
                fit_text(
                    value,
                    Size::BODY,
                    Weight::Medium,
                    Tone::CREAM_100,
                    size.0 - 48.0,
                    1,
                ),
            ));
            // The focus marker on the left edge
            row.spawn((
                Slot(slot),
                Fill {
                    idle: Color::NONE,
                    focus: Tone::CREAM_100,
                },
                BackgroundColor(Color::NONE),
                Node {
                    left: px(0),
                    ..boxed(0.0, 0.0, 8.0, size.1)
                },
            ));
        });
}

/// One option or slider row (spec 600x72).
fn option_row(
    stage: &mut ChildSpawnerCommands,
    slot: usize,
    at: (f32, f32),
    opt: Opt,
) {
    let ink = Ink {
        idle: Tone::CREAM_100,
        focus: Tone::BROWN_900,
    };
    stage
        .spawn((
            Slot(slot),
            Fill {
                idle: Tone::BROWN_800,
                focus: Tone::PUMPKIN,
            },
            BackgroundColor(Tone::BROWN_800),
            Node {
                border_radius: pill(Space::RADIUS_S),
                ..boxed(at.0, at.1, 600.0, 72.0)
            },
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    align_items: AlignItems::Center,
                    ..boxed(24.0, 0.0, 316.0, 72.0)
                },
                children![(
                    Slot(slot),
                    ink,
                    fit_text(
                        opt.name(),
                        Size::BODY,
                        Weight::Medium,
                        Tone::CREAM_100,
                        316.0,
                        1
                    )
                )],
            ));
            if opt.level(&Options::default()).is_some() {
                // Ten pips, then the number
                for i in 0..10u8 {
                    row.spawn((
                        Bind::Pip(opt, i),
                        Slot(slot),
                        Fill {
                            idle: Tone::BROWN_700,
                            focus: Tone::CREAM_100,
                        },
                        BackgroundColor(Tone::BROWN_700),
                        Node {
                            border_radius: pill(4.0),
                            ..boxed(356.0 + f32::from(i) * 18.0, 24.0, 14.0, 24.0)
                        },
                    ));
                }
                row.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::End,
                        ..boxed(546.0, 0.0, 30.0, 72.0)
                    },
                    children![(
                        Bind::Value(opt),
                        Slot(slot),
                        ink,
                        fit_text("0", Size::BODY, Weight::Medium, Tone::CREAM_100, 30.0, 1)
                    )],
                ));
            } else {
                for (x, mark) in [(356.0, "<"), (562.0, ">")] {
                    row.spawn((
                        Node {
                            align_items: AlignItems::Center,
                            ..boxed(x, 0.0, 14.0, 72.0)
                        },
                        children![(
                            Slot(slot),
                            ink,
                            fit_text(mark, Size::BODY, Weight::Medium, Tone::CREAM_100, 14.0, 1)
                        )],
                    ));
                }
                row.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..boxed(376.0, 0.0, 180.0, 72.0)
                    },
                    children![(
                        Bind::Value(opt),
                        Slot(slot),
                        ink,
                        fit_text("", Size::BODY, Weight::Medium, Tone::CREAM_100, 180.0, 1)
                    )],
                ));
            }
        });
}

fn screen(
    mut commands: Commands,
    state: Res<State<GameState>>,
    setup: Res<Setup>,
    mut cursor: ResMut<Cursor>,
) {
    let state = *state.get();
    cursor.0 = 0;
    match state {
        GameState::Menu => title(&mut commands),
        GameState::Options => options(&mut commands),
        GameState::Lobby => lobby_screen(&mut commands),
        GameState::PickRacer => racers(&mut commands, setup.racers.len()),
        GameState::PickTrack => tracks(&mut commands),
        GameState::Race => {}
    }
}

fn title(commands: &mut Commands) {
    page(commands, GameState::Menu, |stage| {
        stage.spawn((
            Node {
                border_radius: pill(Space::RADIUS_S),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..boxed(720.0, 96.0, 512.0, 44.0)
            },
            BackgroundColor(Tone::CRANBERRY),
            children![fit_text(
                "THANKSGIVING KART RACING",
                Size::LABEL,
                Weight::SemiBold,
                Tone::CREAM_100,
                480.0,
                1
            )],
        ));
        caption(
            stage,
            (720.0, 154.0),
            (512.0, 108.0),
            fit_text(
                "GOBBLE",
                Size::DISPLAY,
                Weight::Bold,
                Tone::PUMPKIN,
                512.0,
                1,
            ),
            false,
        );
        caption(
            stage,
            (720.0, 262.0),
            (512.0, 78.0),
            fit_text(
                "GRAND PRIX",
                Size::DISPLAY_SUB,
                Weight::Bold,
                Tone::CREAM_100,
                512.0,
                1,
            ),
            false,
        );
        for (i, name) in ["Start Race", "Options", "Exit"].into_iter().enumerate() {
            button(stage, i, (720.0, 416.0 + i as f32 * 88.0), 512.0, name);
        }
        caption(
            stage,
            (48.0, 760.0),
            (560.0, 28.0),
            fit_text(
                VERSION,
                Size::LABEL,
                Weight::SemiBold,
                Tone::CREAM_300,
                560.0,
                1,
            ),
            false,
        );
        hint_bar(
            stage,
            &[
                (Action::Alt, "What's new"),
                (Action::Navigate, "Choose"),
                (Action::Confirm, "OK"),
                (Action::Back, "Back"),
            ],
        );
        news_box(stage);
    });
}

/// The "What's new" box: the latest changelog lines over the title screen.
fn news_box(stage: &mut ChildSpawnerCommands) {
    stage
        .spawn((
            NewsBox,
            Visibility::Hidden,
            GlobalZIndex(10),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(Space::S),
                padding: UiRect::all(px(Space::L)),
                border: UiRect::all(px(Space::FOCUS)),
                border_radius: pill(Space::RADIUS_M),
                ..boxed(320.0, 160.0, 640.0, 440.0)
            },
            BackgroundColor(Tone::BROWN_800),
            BorderColor::all(Tone::CREAM_100),
        ))
        .with_children(|b| {
            b.spawn(fit_text(
                "What's new",
                Size::H2,
                Weight::Bold,
                Tone::PUMPKIN,
                584.0,
                1,
            ));
            b.spawn(fit_text(
                VERSION,
                Size::LABEL,
                Weight::SemiBold,
                Tone::CREAM_300,
                584.0,
                1,
            ));
            for line in news() {
                b.spawn(fit_text(
                    line,
                    Size::BODY,
                    Weight::Medium,
                    Tone::CREAM_100,
                    584.0,
                    2,
                ));
            }
        });
}

/// Marks the phone invite so it is spawned once per visit to the title menu or the lobby.
#[derive(Component, Copy, Clone, Debug)]
struct Invite;

/// The phone controller QR over the title's turntable area, once the server is up.
fn late_invite(
    mut commands: Commands,
    state: Res<State<GameState>>,
    join: Option<Res<JoinInfo>>,
    shown: Query<(), With<Invite>>,
) {
    let state = *state.get();
    let Some(join) = join else {
        return;
    };
    if !shown.is_empty() {
        return;
    }
    // Title: over the turntable. Lobby: under the help panel
    let at = if state == GameState::Lobby {
        (672.0, 340.0, 560.0)
    } else {
        (64.0, 560.0, 600.0)
    };
    let root = stage(&mut commands, state);
    // An overlay: never the solid backdrop, which would hide the screen under it
    commands
        .entity(root)
        .insert((Invite, BackgroundColor(Color::NONE)))
        .with_children(|root| {
            root.spawn(Node {
                width: px(1280),
                height: px(800),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|stage| invite(stage, &join, at));
        });
}

fn invite(
    stage: &mut ChildSpawnerCommands,
    join: &JoinInfo,
    (left, top, width): (f32, f32, f32),
) {
    stage
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(Space::M),
                padding: UiRect::all(px(Space::S)),
                border_radius: pill(Space::RADIUS_M),
                ..boxed(left, top, width, 168.0)
            },
            BackgroundColor(Tone::CHIP),
        ))
        .with_children(|invite| {
            invite.spawn((
                ImageNode::new(join.qr.clone()),
                Node {
                    width: px(150),
                    height: px(150),
                    flex_shrink: 0.0,
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
                    // width - 2 * 8 padding - 150 QR - 16 gap
                    let w = width - 182.0;
                    text.spawn(fit_text(
                        "Phone controller",
                        Size::BODY,
                        Weight::Medium,
                        Tone::PUMPKIN,
                        w,
                        1,
                    ));
                    text.spawn(fit_text(
                        format!("PIN {}", join.pin),
                        Size::BODY,
                        Weight::Medium,
                        Tone::CREAM_100,
                        w,
                        1,
                    ));
                    let shown = match &join.anywhere {
                        Some(_) => format!("Any network, or same Wi-Fi: {}", join.url),
                        None => join.url.clone(),
                    };
                    text.spawn(fit_text(
                        shown,
                        Size::LABEL,
                        Weight::SemiBold,
                        Tone::CREAM_300,
                        w,
                        2,
                    ));
                    text.spawn(fit_text(
                        "Accept the certificate warning once",
                        Size::LABEL,
                        Weight::SemiBold,
                        Tone::CREAM_300,
                        w,
                        2,
                    ));
                });
        });
}

fn options(commands: &mut Commands) {
    page(commands, GameState::Options, |stage| {
        heading(stage, "Options", 560.0);
        for (i, opt) in Opt::ALL.into_iter().enumerate() {
            option_row(stage, i, (48.0, 120.0 + i as f32 * 80.0), opt);
        }
        button(stage, 6, (48.0, 600.0), 600.0, "Back");
        stage
            .spawn((
                Node {
                    border_radius: pill(Space::RADIUS_M),
                    ..boxed(672.0, 120.0, 560.0, 488.0)
                },
                BackgroundColor(Tone::BROWN_800),
            ))
            .with_children(|panel| {
                caption(
                    panel,
                    (24.0, 20.0),
                    (512.0, 40.0),
                    fit_text(
                        "Controls",
                        Size::H2,
                        Weight::Bold,
                        Tone::CREAM_100,
                        512.0,
                        1,
                    ),
                    false,
                );
                for (i, (action, name)) in CONTROLS.into_iter().enumerate() {
                    let y = 72.0 + i as f32 * 50.0;
                    if i > 0 {
                        panel.spawn((boxed(24.0, y, 512.0, 2.0), BackgroundColor(Tone::BROWN_700)));
                    }
                    panel.spawn((
                        Glyph(action, None),
                        ImageNode::default(),
                        boxed(24.0, y + 5.0, 40.0, 40.0),
                    ));
                    caption(
                        panel,
                        (84.0, y),
                        (452.0, 50.0),
                        fit_text(name, Size::BODY, Weight::Medium, Tone::CREAM_100, 452.0, 1),
                        false,
                    );
                }
            });
        hint_bar(
            stage,
            &[
                (Action::Navigate, "Choose / adjust"),
                (Action::Confirm, "OK"),
                (Action::Back, "Back"),
            ],
        );
    });
}

fn lobby_screen(commands: &mut Commands) {
    page(commands, GameState::Lobby, |stage| {
        heading(stage, "Join in", 800.0);
        for slot in 0..4 {
            stage
                .spawn((
                    Bind::Seat(slot),
                    Fill {
                        idle: Tone::BROWN_800,
                        focus: Tone::BROWN_800,
                    },
                    BackgroundColor(Tone::BROWN_800),
                    Node {
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(px(Space::L)),
                        border_radius: pill(Space::RADIUS_S),
                        ..boxed(48.0, 120.0 + slot as f32 * 88.0, 600.0, 72.0)
                    },
                ))
                .with_child((
                    Bind::Seat(slot),
                    fit_text("", Size::BODY, Weight::Medium, Tone::CREAM_300, 552.0, 1),
                ));
        }
        stage
            .spawn((
                Node {
                    padding: UiRect::all(px(Space::L)),
                    border_radius: pill(Space::RADIUS_M),
                    ..boxed(672.0, 120.0, 560.0, 200.0)
                },
                BackgroundColor(Tone::BROWN_800),
            ))
            .with_child((
                Bind::LobbyHelp,
                fit_text("", Size::BODY, Weight::Medium, Tone::CREAM_100, 512.0, 5),
            ));
        hint_bar(
            stage,
            &[
                (Action::Confirm, "Join / start"),
                (Action::Back, "Leave / back"),
            ],
        );
    });
}

/// The racer panel: name, blurb and stat bars of one seat's pick. Built per seat so split
/// screen can hang one per player; `left` is its x on the stage.
fn racer_panel(
    stage: &mut ChildSpawnerCommands,
    left: f32,
) {
    caption(
        stage,
        (left, 120.0),
        (336.0, 48.0),
        (
            Bind::RacerName,
            fit_text("", Size::H1, Weight::Bold, Tone::PUMPKIN, 336.0, 1),
        ),
        false,
    );
    stage.spawn((
        boxed(left, 176.0, 336.0, 150.0),
        children![(
            Bind::RacerBlurb,
            fit_text("", Size::BODY, Weight::Medium, Tone::CREAM_100, 336.0, 5)
        )],
    ));
    for (i, name) in ["Speed", "Accel", "Turn"].into_iter().enumerate() {
        bar(
            stage,
            left,
            336.0 + i as f32 * 48.0,
            name,
            (
                Bind::RacerStat(i),
                Node {
                    border_radius: pill(8.0),
                    ..boxed(112.0, 8.0, 20.0, 16.0)
                },
                BackgroundColor(Tone::PUMPKIN),
            ),
        );
    }
}

fn racers(
    commands: &mut Commands,
    seats: usize,
) {
    page(commands, GameState::PickRacer, |stage| {
        heading(stage, "Pick your racer", 640.0);
        caption(
            stage,
            (912.0, 52.0),
            (320.0, 28.0),
            (
                Bind::SeatPlayer,
                fit_text(
                    "Player 1",
                    Size::LABEL,
                    Weight::SemiBold,
                    Tone::CREAM_300,
                    320.0,
                    1,
                ),
            ),
            false,
        );
        // With several seats the rows make room for a tag per seat on the right
        let width = if seats > 1 { 240.0 } else { 360.0 };
        for (i, racer) in ROSTER.iter().enumerate() {
            let top = 120.0 + i as f32 * 72.0;
            list_row(
                stage,
                i,
                (48.0, top),
                (width, 64.0),
                racer.name,
                Bind::RacerRow(i),
            );
            for seat in 0..seats.min(4) {
                if seats < 2 {
                    break;
                }
                caption(
                    stage,
                    (296.0 + seat as f32 * 28.0, top),
                    (28.0, 64.0),
                    (
                        Bind::RacerTag(i, seat),
                        fit_text("", Size::LABEL, Weight::Bold, seat_color(seat), 28.0, 1),
                    ),
                    true,
                );
            }
        }
        // The 3D turntable shows through here
        stage.spawn(boxed(432.0, 120.0, 440.0, 568.0));
        racer_panel(stage, 896.0);
        let strip = if seats > 1 { 4 } else { 1 };
        stage.spawn((
            boxed(896.0, 536.0, 336.0, 152.0),
            children![(
                Bind::SeatStrip,
                fit_text(
                    format!("{} racers", ROSTER.len()),
                    Size::LABEL,
                    Weight::SemiBold,
                    Tone::CREAM_300,
                    336.0,
                    strip + 1
                )
            )],
        ));
        hint_bar(
            stage,
            &[
                (Action::Navigate, "Choose"),
                (Action::Confirm, "Lock in"),
                (Action::Back, "Unlock or back"),
            ],
        );
    });
}

fn tracks(commands: &mut Commands) {
    page(commands, GameState::PickTrack, |stage| {
        heading(stage, "Pick a track", 800.0);
        for (i, track) in TRACKS.iter().enumerate() {
            stage
                .spawn((
                    Slot(i),
                    Ring,
                    Fill {
                        idle: Tone::BROWN_800,
                        focus: Tone::PUMPKIN,
                    },
                    BackgroundColor(Tone::BROWN_800),
                    BorderColor::all(Color::NONE),
                    Node {
                        border: UiRect::all(px(Space::FOCUS)),
                        border_radius: pill(Space::RADIUS_M),
                        ..boxed(48.0 + i as f32 * 400.0, 120.0, 384.0, 520.0)
                    },
                ))
                .with_children(|card| {
                    // Sky over ground stands in for the flyover
                    card.spawn((
                        Node {
                            overflow: Overflow::clip(),
                            flex_direction: FlexDirection::Column,
                            border_radius: pill(Space::RADIUS_M),
                            ..boxed(12.0, 12.0, 352.0, 264.0)
                        },
                        BackgroundColor(track.theme.sky),
                        children![(
                            Node {
                                width: percent(100),
                                height: percent(40),
                                margin: UiRect::top(percent(60)),
                                ..default()
                            },
                            BackgroundColor(track.theme.ground)
                        )],
                    ));
                    let ink = Ink {
                        idle: Tone::CREAM_100,
                        focus: Tone::BROWN_900,
                    };
                    caption(
                        card,
                        (20.0, 296.0),
                        (336.0, 44.0),
                        (
                            Slot(i),
                            ink,
                            fit_text(
                                track.name,
                                Size::H2,
                                Weight::Bold,
                                Tone::CREAM_100,
                                336.0,
                                1,
                            ),
                        ),
                        false,
                    );
                    card.spawn((
                        boxed(20.0, 348.0, 336.0, 156.0),
                        children![(
                            Slot(i),
                            Ink {
                                idle: Tone::CREAM_300,
                                focus: Tone::BROWN_900,
                            },
                            fit_text(
                                track.blurb,
                                Size::BODY,
                                Weight::Medium,
                                Tone::CREAM_300,
                                336.0,
                                5
                            )
                        )],
                    ));
                });
        }
        caption(
            stage,
            (48.0, 672.0),
            (400.0, 28.0),
            fit_text(
                format!("{LAPS} laps"),
                Size::LABEL,
                Weight::SemiBold,
                Tone::CREAM_300,
                400.0,
                1,
            ),
            false,
        );
        hint_bar(
            stage,
            &[
                (Action::Navigate, "Choose"),
                (Action::Confirm, "Race"),
                (Action::Back, "Back"),
            ],
        );
    });
}

/// Which row the screen shows as focused.
fn focus(
    state: Res<State<GameState>>,
    cursor: Res<Cursor>,
    setup: Res<Setup>,
    mut focus: ResMut<Focus>,
) {
    let at = match state.get() {
        GameState::Menu | GameState::Options => Some(cursor.0),
        // Several seats show their cursors as tags and fills instead
        GameState::PickRacer if setup.racers.len() == 1 => setup.racers.first().copied(),
        GameState::PickTrack => Some(setup.track),
        _ => None,
    };
    if focus.0 != at {
        focus.0 = at;
    }
}

/// Fill the live parts of the current screen.
fn bind(
    setup: Res<Setup>,
    options: Res<Options>,
    seats: Res<Seats>,
    names: Res<SeatNames>,
    ready: Res<Ready>,
    mut texts: Query<(&Bind, &mut Fitted)>,
    mut nodes: Query<(&Bind, &mut Node, Option<&mut Fill>), Without<Fitted>>,
) {
    let racer = &ROSTER[setup
        .racers
        .first()
        .copied()
        .unwrap_or(0)
        .min(ROSTER.len() - 1)];
    let locked = |slot: usize| ready.0.get(slot).copied().unwrap_or(false);
    let stat = |i: usize| {
        let v = match i {
            0 => racer.stats.top_speed,
            1 => racer.stats.accel,
            _ => racer.stats.turn,
        };
        ((v - 0.7) / 0.6 * 200.0).round().clamp(20.0, 200.0)
    };
    for (bind, mut fitted) in &mut texts {
        match *bind {
            Bind::Value(opt) => fitted.set(opt.value(&options)),
            Bind::Seat(slot) => fitted.set(seats.0.get(slot).map_or_else(
                || format!("P{}  press A or Enter to join", slot + 1),
                |device| format!("P{}  {}  ({})", slot + 1, names.get(slot), device_name(*device)),
            )),
            Bind::LobbyHelp => fitted.set(if seats.0.is_empty() {
                "Press A or Enter on a controller, the keyboard or a phone to take a seat. B or Esc goes back to the menu."
            } else {
                "Joined players press A or Enter again to start. B or Esc leaves your seat."
            }),
            Bind::RacerName => fitted.set(racer.name),
            Bind::RacerBlurb => fitted.set(racer.blurb),
            Bind::SeatPlayer => fitted.set(if setup.racers.len() > 1 {
                format!("Players: {}", setup.racers.len())
            } else {
                "Player 1".to_string()
            }),
            Bind::SeatStrip => {
                if setup.racers.len() > 1 {
                    let mut lines = Vec::new();
                    for (slot, pick) in setup.racers.iter().enumerate() {
                        let done = if locked(slot) { "  LOCKED" } else { "" };
                        lines.push(format!(
                            "P{} {}: {}{done}",
                            slot + 1,
                            names.get(slot),
                            ROSTER[*pick].name
                        ));
                    }
                    fitted.set(lines.join("\n"));
                } else {
                    fitted.set("");
                }
            }
            Bind::RacerTag(row, seat) => fitted.set(
                if setup.racers.get(seat) == Some(&row) {
                    format!("P{}", seat + 1)
                } else {
                    String::new()
                },
            ),
            _ => {}
        }
    }
    for (bind, mut node, fill) in &mut nodes {
        let Some(mut fill) = fill else {
            if let Bind::RacerStat(i) = *bind {
                node.width = px(stat(i));
            }
            continue;
        };
        match *bind {
            Bind::Pip(opt, i) => {
                let on = opt.level(&options).is_some_and(|level| i < level);
                *fill = if on {
                    Fill {
                        idle: Tone::PUMPKIN,
                        focus: Tone::BROWN_900,
                    }
                } else {
                    Fill {
                        idle: Tone::BROWN_700,
                        focus: Tone::CREAM_100,
                    }
                };
            }
            Bind::Seat(slot) => {
                let joined = seats.0.get(slot).is_some();
                let c = if joined {
                    Tone::CRANBERRY
                } else {
                    Tone::BROWN_800
                };
                *fill = Fill { idle: c, focus: c };
            }
            Bind::RacerRow(i) => {
                // Several seats: the row takes the colour of the first seat on it, darker
                // until that seat locks in
                let on = setup.racers.iter().position(|r| *r == i);
                fill.idle = match on {
                    Some(seat) if setup.racers.len() > 1 => {
                        let c = seat_color(seat);
                        // Kept dark enough for the cream name on top
                        c.mix(&Tone::BROWN_800, if locked(seat) { 0.35 } else { 0.7 })
                    }
                    _ => Tone::BROWN_800,
                };
            }
            _ => {}
        }
    }
}

/// The platter only shows on racer select; the attract race fills every other screen.
fn frame(
    state: Res<State<GameState>>,
    mut cameras: Query<&mut Camera, With<PlatterCam>>,
) {
    for mut camera in &mut cameras {
        camera.is_active = *state.get() == GameState::PickRacer;
    }
}

fn backdrop(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scope = DespawnOnExit(InMenus);
    commands.spawn((
        PlatterCam,
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            is_active: false,
            ..default()
        },
        Transform::from_translation(PODIUM + Vec3::new(0.0, 3.0, -7.5))
            .looking_at(PODIUM + Vec3::Y, Vec3::Y),
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
        Transform::from_translation(PODIUM),
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
        Transform::from_translation(PODIUM + Vec3::Y * 0.08),
        scope,
    ));
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
                5 if step != 0 => options.split = options.split.cycle(step),
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
            Transform::from_translation(PODIUM + Vec3::Y * 0.1),
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
        let step = i32::from(input.down) - i32::from(input.up);
        if step != 0 && !ready.0[slot] {
            let count = ROSTER.len() as i32;
            setup.racers[slot] = (setup.racers[slot] as i32 + step).rem_euclid(count) as usize;
            sfx.write(Sfx::Click);
        }
        // A phone taps a racer on its own list
        if let Device::Phone(id) = device
            && let Some(racer) = phones.pick(id).filter(|r| *r < ROSTER.len())
            && !ready.0[slot]
        {
            setup.racers[slot] = racer;
            sfx.write(Sfx::Click);
        }
        if input.confirm && !ready.0[slot] {
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

/// Each seated phone gets the racer list with its own pick, so it can choose by tapping.
fn tell_picks(
    phones: Res<Phones>,
    seats: Res<Seats>,
    setup: Res<Setup>,
    ready: Res<Ready>,
    state: Res<State<GameState>>,
    mut told: Local<Vec<(u32, usize, bool)>>,
) {
    // Phones hide the list when the screen is left, so tell them again on the way back
    if state.is_changed() {
        told.clear();
    }
    let list = serde_json::to_string(&ROSTER.iter().map(|r| r.name).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut now = Vec::new();
    for (slot, device) in seats.0.iter().enumerate() {
        let (Device::Phone(id), Some(pick)) = (device, setup.racers.get(slot)) else {
            continue;
        };
        let locked = ready.0.get(slot).copied().unwrap_or(false);
        let state = (*id, *pick, locked);
        // Only on change; a phone that reconnects gets the next change
        if !told.contains(&state) {
            phones.send(format!(
                r#"{{"t":"racers","id":{id},"pick":{pick},"locked":{locked},"list":{list}}}"#
            ));
        }
        now.push(state);
    }
    *told = now;
}
