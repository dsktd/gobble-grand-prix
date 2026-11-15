//! Race HUD per seat (lap, time, item, place, drift meter, speed-line burst), the shared
//! countdown banner, standings for an odd quadrant, and the results board.
//!
//! Layout and sizes follow the UI spec's HUD chips, scaled to each seat's viewport.

use bevy::prelude::*;

use crate::minimap;
use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn.after(spawn_track));
    app.add_systems(
        Update,
        (chips, drift, banner, standings, results, burst)
            .after(KartSet::Race)
            .run_if(in_state(GameState::Race)),
    );
}

/// A HUD part belonging to the human in a seat.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq)]
enum Part {
    Lap,
    Time,
    Place,
    Suffix,
    ItemArt,
    ItemName,
    ItemChip,
    DriftMeter,
    DriftFill,
    DriftStage,
}

#[derive(Component, Copy, Clone, Debug)]
struct Seat(usize);

#[derive(Component, Copy, Clone, Debug)]
pub(crate) struct Banner;

#[derive(Component, Copy, Clone, Debug)]
struct Standings(usize);

/// Karts listed in the standings.
const STANDING_ROWS: usize = 6;

#[derive(Component, Copy, Clone, Debug)]
pub(crate) struct Board;

/// Manga speed-line burst over a seat's view, fading over [`Burst::TIME`].
#[derive(Component, Copy, Clone, Debug)]
struct Burst {
    slot: usize,
    left: f32,
}
impl Burst {
    const TIME: f32 = 0.4;
}

/// Black radial speed lines around a clear centre, made once.
fn burst_image() -> Image {
    let (w, h) = (640u32, 400u32);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 - w as f32 / 2.0) / (w as f32 / 2.0);
            let dy = (y as f32 - h as f32 / 2.0) / (h as f32 / 2.0);
            let r = dx.hypot(dy);
            let a = dy.atan2(dx);
            // Uneven spikes: several sine bands multiplied
            let spikes = ((a * 37.0).sin() * (a * 23.0 + 1.3).sin() * (a * 11.0 + 0.7).sin()).abs();
            let line = spikes > 0.35 && r > 0.45 + (1.0 - spikes) * 0.3;
            if line {
                let at = ((y * w + x) * 4) as usize;
                data[at + 3] = (((r - 0.45) * 3.0).clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

const fn ordinal_suffix(n: usize) -> &'static str {
    match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    }
}

fn ordinal(n: usize) -> String {
    format!("{n}{}", ordinal_suffix(n))
}

fn clock_text(t: f32) -> String {
    format!("{}:{:05.2}", (t / 60.0) as u32, t % 60.0)
}

/// An absolutely placed box, in spec px scaled by `k`.
fn place(
    k: f32,
    left: Option<f32>,
    top: Option<f32>,
    right: Option<f32>,
    bottom: Option<f32>,
    width: f32,
    height: f32,
) -> Node {
    let at = |v: Option<f32>| v.map_or(Val::Auto, |v| px(v * k));
    Node {
        position_type: PositionType::Absolute,
        left: at(left),
        top: at(top),
        right: at(right),
        bottom: at(bottom),
        width: px(width * k),
        height: px(height * k),
        border_radius: BorderRadius::all(px(Space::RADIUS_M * k)),
        ..default()
    }
}

/// [`place`] for a text: as tall as its line, never a box the line outgrows.
fn place_text(
    k: f32,
    left: f32,
    top: f32,
    width: f32,
) -> Node {
    Node {
        height: Val::Auto,
        ..place(k, Some(left), Some(top), None, None, width, 0.0)
    }
}

/// A node's width in spec px.
fn node_width(
    node: &Node,
    k: f32,
) -> f32 {
    match node.width {
        Val::Px(w) => w / k,
        _ => 0.0,
    }
}

/// A caption over a value, in a translucent chip.
fn chip(
    parent: &mut ChildSpawnerCommands,
    slot: usize,
    k: f32,
    node: Node,
    caption: &str,
    part: Part,
) {
    // Text boxes keep 16 px clear on both sides of the chip
    let inner = node_width(&node, k) - 32.0;
    parent
        .spawn((node, BackgroundColor(Tone::CHIP)))
        .with_children(|chip| {
            chip.spawn((
                fit_text(
                    caption,
                    Size::LABEL * k,
                    Weight::SemiBold,
                    Tone::CREAM_300,
                    inner * k,
                    1,
                ),
                place_text(k, 16.0, 8.0, inner),
            ));
            chip.spawn((
                Seat(slot),
                part,
                fit_text(
                    "",
                    Size::HUD_NUM * k,
                    Weight::Bold,
                    Tone::CREAM_100,
                    inner * k,
                    1,
                ),
                place_text(k, 16.0, 32.0, inner),
            ));
        });
}

fn spawn(
    mut commands: Commands,
    track: Res<Track>,
    setup: Res<Setup>,
    names: Res<SeatNames>,
    options: Res<Options>,
    mut images: ResMut<Assets<Image>>,
) {
    let scope = DespawnOnExit(GameState::Race);
    let lines = images.add(burst_image());
    let map = minimap::Map::new(&track);
    let art = images.add(minimap::art(&track, &map));
    commands.insert_resource(map);
    let seats = setup.racers.len();
    // One window-sized root: percentages need a sized parent to resolve against
    let hud = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            scope,
        ))
        .id();
    for slot in 0..seats {
        let rect = hud_rect(slot, seats, options.split);
        // Chips shrink with the viewport, but never below a readable size
        let k = rect.width().min(rect.height()).max(0.6);
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(rect.min.x * 100.0),
                    top: percent(rect.min.y * 100.0),
                    width: percent(rect.width() * 100.0),
                    height: percent(rect.height() * 100.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                ChildOf(hud),
            ))
            .with_children(|root| {
                root.spawn((
                    Burst { slot, left: 0.0 },
                    ImageNode::new(lines.clone()),
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    Visibility::Hidden,
                ));
                chip(
                    root,
                    slot,
                    k,
                    place(k, Some(32.0), Some(32.0), None, None, 168.0, 72.0),
                    "LAP",
                    Part::Lap,
                );
                chip(
                    root,
                    slot,
                    k,
                    place(k, Some(32.0), Some(120.0), None, None, 184.0, 72.0),
                    "TIME",
                    Part::Time,
                );
                if seats > 1 {
                    // Seat badge: number and name in the seat's colour, as on its phone
                    let mut badge = place(k, Some(32.0), Some(200.0), None, None, 184.0, 36.0);
                    badge.justify_content = JustifyContent::Center;
                    badge.align_items = AlignItems::Center;
                    badge.overflow = Overflow::clip();
                    let tag = format!("P{} {}", slot + 1, names.get(slot));
                    root.spawn((badge, BackgroundColor(seat_color(slot))))
                        .with_child(fit_text_min(
                            tag,
                            Size::LABEL * k,
                            12.0 * k,
                            Weight::Bold,
                            Tone::FELT_900,
                            176.0 * k,
                            1,
                        ));
                }

                // Item slot, centred at the top
                let mut item = place(k, None, Some(32.0), None, None, 104.0, 104.0);
                item.left = percent(50);
                item.margin.left = px(-52.0 * k);
                item.border = UiRect::all(px(Space::FOCUS * k));
                root.spawn((
                    Seat(slot),
                    Part::ItemChip,
                    item,
                    BackgroundColor(Tone::CHIP),
                    BorderColor::all(Tone::CREAM_300),
                ))
                .with_children(|chip| {
                    let mut art = place(k, Some(20.0), Some(8.0), None, None, 56.0, 56.0);
                    art.border_radius = BorderRadius::all(percent(50));
                    chip.spawn((Seat(slot), Part::ItemArt, art, BackgroundColor(Color::NONE)));
                    let mut name = place(k, Some(0.0), Some(66.0), None, None, 96.0, 24.0);
                    name.justify_content = JustifyContent::Center;
                    chip.spawn(name).with_child((
                        Seat(slot),
                        Part::ItemName,
                        fit_text_min(
                            "",
                            Size::LABEL * k,
                            Size::LABEL * k * 0.7,
                            Weight::SemiBold,
                            Tone::CREAM_100,
                            96.0 * k,
                            1,
                        ),
                    ));
                    // Which button uses it
                    let mut hint = place(
                        k,
                        None,
                        None,
                        Some(-14.0),
                        Some(-14.0),
                        Size::GLYPH,
                        Size::GLYPH,
                    );
                    hint.border_radius = BorderRadius::ZERO;
                    chip.spawn((hint, Glyph(Action::Item, Some(slot)), ImageNode::default()));
                });

                // Position, top right
                root.spawn((
                    place(k, None, Some(32.0), Some(32.0), None, 176.0, 104.0),
                    BackgroundColor(Tone::CHIP),
                ))
                .with_children(|chip| {
                    chip.spawn((
                        Seat(slot),
                        Part::Place,
                        fit_text(
                            "",
                            Size::HUD_PLACE * k,
                            Weight::Bold,
                            Tone::BRASS,
                            60.0 * k,
                            1,
                        ),
                        place_text(k, 20.0, 10.0, 60.0),
                    ));
                    chip.spawn((
                        Seat(slot),
                        Part::Suffix,
                        fit_text("", Size::H2 * k, Weight::Bold, Tone::BRASS, 60.0 * k, 1),
                        place_text(k, 72.0, 18.0, 60.0),
                    ));
                    chip.spawn((
                        fit_text(
                            "/6",
                            Size::LABEL * k,
                            Weight::SemiBold,
                            Tone::CREAM_300,
                            60.0 * k,
                            1,
                        ),
                        place_text(k, 72.0, 58.0, 60.0),
                    ));
                });

                minimap::spawn(root, slot, k, &art);

                // Drift meter, bottom left
                root.spawn((
                    Seat(slot),
                    Part::DriftMeter,
                    place(k, Some(32.0), None, None, Some(32.0), 320.0, 56.0),
                    Visibility::Hidden,
                ))
                .with_children(|meter| {
                    let mut caption = place(k, Some(0.0), Some(0.0), None, None, 200.0, 28.0);
                    caption.column_gap = px(Space::S * k);
                    caption.align_items = AlignItems::Center;
                    meter.spawn(caption).with_children(|c| {
                        c.spawn((
                            Glyph(Action::Drift, Some(slot)),
                            ImageNode::default(),
                            Node {
                                width: px(28.0 * k),
                                height: px(28.0 * k),
                                ..default()
                            },
                        ));
                        c.spawn(fit_text(
                            "DRIFT",
                            Size::LABEL * k,
                            Weight::SemiBold,
                            Tone::CREAM_100,
                            100.0 * k,
                            1,
                        ));
                    });
                    meter.spawn((
                        Seat(slot),
                        Part::DriftStage,
                        fit_text(
                            "",
                            Size::LABEL * k,
                            Weight::SemiBold,
                            Tone::CREAM_100,
                            84.0 * k,
                            1,
                        ),
                        place_text(k, 236.0, 0.0, 84.0),
                    ));
                    let mut track = place(k, Some(0.0), Some(28.0), None, None, 320.0, 24.0);
                    track.border_radius = BorderRadius::all(px(12.0 * k));
                    track.border = UiRect::all(px(2.0 * k));
                    track.overflow = Overflow::clip();
                    meter
                        .spawn((
                            track,
                            BackgroundColor(Tone::CHIP),
                            BorderColor::all(Tone::CREAM_300),
                        ))
                        .with_children(|track| {
                            track.spawn((
                                Seat(slot),
                                Part::DriftFill,
                                Node {
                                    width: percent(0),
                                    height: percent(100),
                                    border_radius: BorderRadius::all(px(12.0 * k)),
                                    ..default()
                                },
                                BackgroundColor(Tone::CREAM_100),
                            ));
                            for tick in [1.0 / 3.0, 2.0 / 3.0] {
                                track.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: percent(tick * 100.0),
                                        width: px(2.0 * k),
                                        height: percent(100),
                                        ..default()
                                    },
                                    BackgroundColor(Tone::FELT_900),
                                ));
                            }
                        });
                });
            });
    }

    // With three seats the empty quadrant lists the standings
    if seats == 3 {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    top: percent(50),
                    width: percent(50),
                    height: percent(50),
                    padding: UiRect::all(px(Space::L)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(Space::XS),
                    ..default()
                },
                BackgroundColor(Tone::FELT_900),
                ChildOf(hud),
            ))
            .with_children(|panel| {
                panel.spawn(fit_text(
                    "Standings",
                    Size::H2,
                    Weight::Bold,
                    Tone::BRASS,
                    500.0,
                    1,
                ));
                for i in 0..STANDING_ROWS {
                    panel.spawn((
                        Standings(i),
                        fit_text("", Size::LABEL, Weight::SemiBold, Tone::CREAM_100, 500.0, 1),
                    ));
                }
            });
    }

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(hud),
        ))
        .with_children(|root| {
            root.spawn((
                Banner,
                shadowed(),
                fit_text("", Size::BANNER, Weight::Bold, Tone::BRASS, 1000.0, 1),
            ));
        });

    commands.spawn((
        Board,
        BackgroundColor(Tone::FELT_900.with_alpha(0.98)),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        Visibility::Hidden,
        ChildOf(hud),
    ));
}

fn chips(
    clock: Res<RaceClock>,
    karts: Query<(&Kart, &Progress, &Human, Option<&Finished>)>,
    parts: Query<(
        &Seat,
        &Part,
        Option<&mut Fitted>,
        Option<&mut BackgroundColor>,
        Option<&mut BorderColor>,
    )>,
) {
    for (seat, part, text, background, border) in parts {
        let Some((kart, progress, _, finished)) = karts.iter().find(|k| k.2.0 == seat.0) else {
            continue;
        };
        let shown = match part {
            Part::Lap => Some(match finished {
                Some(_) => "Done".to_string(),
                None => format!("{}/{LAPS}", progress.lap.clamp(1, LAPS)),
            }),
            Part::Time => Some(clock_text(finished.map_or(clock.time, |f| f.time))),
            Part::Place => Some(progress.place.to_string()),
            Part::Suffix => Some(ordinal_suffix(progress.place).to_string()),
            Part::ItemName => Some(kart.item.map_or(String::new(), |i| i.name().to_string())),
            Part::ItemArt => {
                if let Some(mut background) = background {
                    background.0 = kart.item.map_or(Color::NONE, |i| i.color());
                }
                None
            }
            Part::ItemChip => {
                if let Some(mut border) = border {
                    *border = BorderColor::all(match kart.item {
                        Some(_) => Tone::CREAM_100,
                        None => Tone::CREAM_300.with_alpha(0.5),
                    });
                }
                None
            }
            _ => None,
        };
        if let (Some(shown), Some(mut text)) = (shown, text) {
            text.set(shown);
        }
    }
}

/// The boost meter fills while drifting, through three stages.
fn drift(
    karts: Query<(&Kart, &Human)>,
    parts: Query<(
        &Seat,
        &Part,
        &mut Visibility,
        Option<&mut Node>,
        Option<&mut BackgroundColor>,
        Option<&mut Fitted>,
        Option<&mut TextColor>,
    )>,
) {
    for (seat, part, mut visibility, node, background, text, color) in parts {
        let Some((kart, _)) = karts.iter().find(|(_, h)| h.0 == seat.0) else {
            continue;
        };
        let charge = kart.drift.map(|d| d.charge);
        let fill = charge.map_or(0.0, |c| (c / (Drift::ORANGE * 1.5)).min(1.0));
        let tier = kart.drift.map_or(0, |d| d.tier());
        let stage = match tier {
            0 => Tone::CREAM_100,
            1 => Tone::BRASS,
            _ => Tone::BALL_RED,
        };
        match part {
            Part::DriftMeter => {
                *visibility = if charge.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            Part::DriftFill => {
                if let Some(mut node) = node {
                    node.width = percent(fill * 100.0);
                }
                if let Some(mut background) = background {
                    background.0 = stage;
                }
            }
            Part::DriftStage => {
                if let Some(mut text) = text {
                    text.set(format!("STAGE {}", tier + 1));
                }
                if let Some(mut color) = color {
                    color.0 = stage;
                }
            }
            _ => {}
        }
    }
}

fn banner(
    clock: Res<RaceClock>,
    time: Res<Time<Virtual>>,
    humans: Query<Option<&Finished>, With<Human>>,
    banners: Query<&mut Fitted, With<Banner>>,
) {
    let finished = humans.iter().all(|f| f.is_some());
    let shown = match clock.phase {
        _ if time.is_paused() => "PAUSED".to_string(),
        Phase::Countdown if clock.countdown <= 3.0 => format!("{}", clock.countdown.ceil() as u32),
        Phase::Countdown => String::new(),
        Phase::Running if clock.time < 1.2 => "BREAK!".to_string(),
        Phase::Running if finished => "FINISH!".to_string(),
        _ => String::new(),
    };
    for mut text in banners {
        text.set(&shown);
    }
}

fn standings(
    names: Res<SeatNames>,
    karts: Query<(&Kart, &Progress, Option<&Human>)>,
    texts: Query<(&Standings, &mut Fitted)>,
) {
    let mut rows: Vec<_> = karts.iter().collect();
    rows.sort_by_key(|(_, progress, _)| progress.place);
    for (row, mut text) in texts {
        let shown = rows
            .get(row.0)
            .map_or(String::new(), |(kart, progress, human)| {
                let who = human.map_or(String::new(), |h| format!("  {}", names.get(h.0)));
                format!(
                    "{}  {}{who}",
                    ordinal(progress.place),
                    ROSTER[kart.racer].name
                )
            });
        text.set(shown);
    }
}

fn results(
    mut commands: Commands,
    names: Res<SeatNames>,
    clock: Res<RaceClock>,
    track: Res<Track>,
    karts: Query<(&Kart, &Progress, Option<&Finished>, Option<&Human>)>,
    mut board: Query<(Entity, &mut Visibility), With<Board>>,
    seats: Res<Seats>,
    mut shown: Local<bool>,
) {
    let Ok((board_id, mut visibility)) = board.single_mut() else {
        return;
    };
    if clock.phase != Phase::Done {
        *shown = false;
        return;
    }
    if *shown {
        return;
    }
    *shown = true;
    *visibility = Visibility::Inherited;

    let mut rows: Vec<_> = karts.iter().collect();
    rows.sort_by_key(|(_, progress, _, _)| progress.place);
    let solo = seats.0.len() <= 1;
    let rows: Vec<ResultRow> = rows
        .into_iter()
        .map(|(kart, progress, finished, human)| {
            let name = match (human, solo) {
                (Some(_), true) => format!("{} (you)", ROSTER[kart.racer].name),
                (Some(h), false) => format!("{} ({})", ROSTER[kart.racer].name, names.get(h.0)),
                (None, _) => ROSTER[kart.racer].name.to_string(),
            };
            ResultRow {
                place: progress.place,
                name,
                time: finished.map_or_else(|| "--:--.--".to_string(), |f| clock_text(f.time)),
                mine: human.is_some(),
            }
        })
        .collect();
    spawn_results(&mut commands, board_id, &track.name, &rows);
}

/// One line of the results board.
#[derive(Clone, Debug)]
pub(crate) struct ResultRow {
    pub place: usize,
    pub name: String,
    pub time: String,
    pub mine: bool,
}

/// Fill the results board: heading, track, a row per kart and the hint bar.
pub(crate) fn spawn_results(
    commands: &mut Commands,
    board_id: Entity,
    track: &str,
    rows: &[ResultRow],
) {
    commands
        .entity(board_id)
        .despawn_children()
        .insert(Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|board| {
            board
                .spawn(Node {
                    width: px(1280),
                    height: px(800),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|stage| {
                    stage.spawn((
                        boxed(48.0, 40.0, 800.0, 48.0),
                        children![fit_text(
                            "Results",
                            Size::H1,
                            Weight::Bold,
                            Tone::CREAM_100,
                            800.0,
                            1
                        )],
                    ));
                    stage.spawn((
                        boxed(48.0, 96.0, 800.0, 32.0),
                        children![fit_text(
                            track,
                            Size::BODY,
                            Weight::Medium,
                            Tone::CREAM_300,
                            800.0,
                            1
                        )],
                    ));
                    for (i, row) in rows.iter().enumerate() {
                        let (fill, ink) = if row.mine {
                            (Tone::BRASS, Tone::FELT_900)
                        } else {
                            (Tone::FELT_800, Tone::CREAM_100)
                        };
                        let place_ink = if row.place == 1 && !row.mine {
                            Tone::BRASS
                        } else {
                            ink
                        };
                        let cell = |left: f32, width: f32, end: bool| Node {
                            align_items: AlignItems::Center,
                            justify_content: if end {
                                JustifyContent::End
                            } else {
                                JustifyContent::Start
                            },
                            ..boxed(left, 0.0, width, 64.0)
                        };
                        stage
                            .spawn((
                                Node {
                                    border_radius: pill(Space::RADIUS_S),
                                    ..boxed(240.0, 160.0 + i as f32 * 80.0, 800.0, 64.0)
                                },
                                BackgroundColor(fill),
                            ))
                            .with_children(|r| {
                                r.spawn((
                                    cell(24.0, 96.0, false),
                                    children![fit_text(
                                        ordinal(row.place),
                                        Size::H2,
                                        Weight::Bold,
                                        place_ink,
                                        96.0,
                                        1
                                    )],
                                ));
                                r.spawn((
                                    cell(136.0, 440.0, false),
                                    children![fit_text(
                                        row.name.clone(),
                                        Size::H2,
                                        Weight::SemiBold,
                                        ink,
                                        440.0,
                                        1
                                    )],
                                ));
                                r.spawn((
                                    cell(596.0, 180.0, true),
                                    children![fit_text(
                                        row.time.clone(),
                                        Size::H2,
                                        Weight::SemiBold,
                                        ink,
                                        180.0,
                                        1
                                    )],
                                ));
                            });
                    }
                    hint_bar(
                        stage,
                        &[
                            (Action::Confirm, "Race again"),
                            (Action::Alt, "Change track"),
                            (Action::Back, "Menu"),
                        ],
                    );
                });
        });
}

/// Flash the speed lines when a crying racer gets a turbo or a hit.
fn burst(
    time: Res<Time>,
    karts: Query<(&Kart, &Human)>,
    bursts: Query<(&mut Burst, &mut Visibility, &mut ImageNode)>,
) {
    for (mut burst, mut visibility, mut image) in bursts {
        let Some((kart, _)) = karts.iter().find(|(_, human)| human.0 == burst.slot) else {
            continue;
        };
        if ROSTER[kart.racer].body == Body::Cat && (kart.fired > 0 || kart.struck) {
            burst.left = Burst::TIME;
        }
        burst.left = (burst.left - time.delta_secs()).max(0.0);
        *visibility = if burst.left > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        image.color = Color::WHITE.with_alpha(burst.left / Burst::TIME);
    }
}
