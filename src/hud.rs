//! Race HUD per seat (lap, time, item, place, drift meter, speed-line burst), the shared
//! countdown banner, standings for an odd quadrant, and the results board.
//!
//! Layout and sizes follow the UI spec's HUD chips, scaled to each seat's viewport.

use std::fmt::Write as _;

use bevy::prelude::*;

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
struct Banner;

#[derive(Component, Copy, Clone, Debug)]
struct Standings;

#[derive(Component, Copy, Clone, Debug)]
struct Board;

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

/// A caption over a value, in a translucent chip.
fn chip(
    parent: &mut ChildSpawnerCommands,
    slot: usize,
    k: f32,
    node: Node,
    caption: &str,
    part: Part,
) {
    parent
        .spawn((node, BackgroundColor(Tone::CHIP)))
        .with_children(|chip| {
            chip.spawn((
                text(caption, Size::LABEL * k, Weight::SemiBold, Tone::CREAM_300),
                place(k, Some(16.0), Some(8.0), None, None, 150.0, 24.0),
            ));
            chip.spawn((
                Seat(slot),
                part,
                text("", Size::HUD_NUM * k, Weight::Bold, Tone::CREAM_100),
                place(k, Some(16.0), Some(32.0), None, None, 160.0, 36.0),
            ));
        });
}

fn spawn(
    mut commands: Commands,
    setup: Res<Setup>,
    options: Res<Options>,
    mut images: ResMut<Assets<Image>>,
) {
    let scope = DespawnOnExit(GameState::Race);
    let lines = images.add(burst_image());
    let seats = setup.racers.len();
    for slot in 0..seats {
        let rect = seat_rect(slot, seats, options.side_by_side);
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
                scope.clone(),
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
                        text("", Size::LABEL * k * 0.8, Weight::SemiBold, Tone::CREAM_100),
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
                        text("", Size::HUD_PLACE * k, Weight::Bold, Tone::PUMPKIN),
                        place(k, Some(20.0), Some(10.0), None, None, 60.0, 80.0),
                    ));
                    chip.spawn((
                        Seat(slot),
                        Part::Suffix,
                        text("", Size::H2 * k, Weight::Bold, Tone::PUMPKIN),
                        place(k, Some(72.0), Some(18.0), None, None, 60.0, 36.0),
                    ));
                    chip.spawn((
                        text("/6", Size::LABEL * k, Weight::SemiBold, Tone::CREAM_300),
                        place(k, Some(72.0), Some(58.0), None, None, 60.0, 24.0),
                    ));
                });

                // Drift meter, bottom left
                root.spawn((
                    Seat(slot),
                    Part::DriftMeter,
                    place(k, Some(32.0), None, None, Some(32.0), 320.0, 56.0),
                    Visibility::Hidden,
                ))
                .with_children(|meter| {
                    let mut caption = place(k, Some(0.0), Some(-6.0), None, None, 200.0, 30.0);
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
                        c.spawn(text(
                            "DRIFT",
                            Size::LABEL * k,
                            Weight::SemiBold,
                            Tone::CREAM_100,
                        ));
                    });
                    meter.spawn((
                        Seat(slot),
                        Part::DriftStage,
                        text("", Size::LABEL * k, Weight::SemiBold, Tone::CREAM_100),
                        place(k, Some(236.0), Some(0.0), None, None, 90.0, 24.0),
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
                                    BackgroundColor(Tone::BROWN_900),
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
                BackgroundColor(Tone::BROWN_900),
                scope.clone(),
            ))
            .with_children(|panel| {
                panel.spawn(text("Standings", Size::H2, Weight::Bold, Tone::PUMPKIN));
                panel.spawn((
                    Standings,
                    text("", Size::BODY, Weight::Medium, Tone::CREAM_100),
                ));
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
            scope.clone(),
        ))
        .with_children(|root| {
            root.spawn((Banner, label("", Size::BANNER, Tone::PUMPKIN)));
        });

    commands.spawn((
        Board,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::top(px(Space::XXL)),
            row_gap: px(Space::M),
            ..default()
        },
        BackgroundColor(Tone::BROWN_900.with_alpha(0.92)),
        Visibility::Hidden,
        scope,
    ));
}

fn chips(
    clock: Res<RaceClock>,
    karts: Query<(&Kart, &Progress, &Human, Option<&Finished>)>,
    parts: Query<(
        &Seat,
        &Part,
        Option<&mut Text>,
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
        if let (Some(shown), Some(mut text)) = (shown, text)
            && text.0 != shown
        {
            text.0 = shown;
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
        Option<&mut Text>,
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
            1 => Tone::PUMPKIN,
            _ => Tone::CRANBERRY,
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
                    let shown = format!("STAGE {}", tier + 1);
                    if text.0 != shown {
                        text.0 = shown;
                    }
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
    banners: Query<&mut Text, With<Banner>>,
) {
    let finished = humans.iter().all(|f| f.is_some());
    let shown = match clock.phase {
        _ if time.is_paused() => "PAUSED".to_string(),
        Phase::Countdown if clock.countdown <= 3.0 => format!("{}", clock.countdown.ceil() as u32),
        Phase::Countdown => String::new(),
        Phase::Running if clock.time < 1.2 => "GOBBLE!".to_string(),
        Phase::Running if finished => "FINISH!".to_string(),
        _ => String::new(),
    };
    for mut text in banners {
        if text.0 != shown {
            text.0.clone_from(&shown);
        }
    }
}

fn standings(
    karts: Query<(&Kart, &Progress, Option<&Human>)>,
    texts: Query<&mut Text, With<Standings>>,
) {
    let mut rows: Vec<_> = karts.iter().collect();
    rows.sort_by_key(|(_, progress, _)| progress.place);
    let mut shown = String::new();
    for (kart, progress, human) in &rows {
        let who = human.map_or(String::new(), |h| format!("  P{}", h.0 + 1));
        let _ = writeln!(
            shown,
            "{}  {}{who}",
            ordinal(progress.place),
            ROSTER[kart.racer].name
        );
    }
    for mut text in texts {
        if text.0 != shown {
            text.0.clone_from(&shown);
        }
    }
}

fn results(
    mut commands: Commands,
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
    commands
        .entity(board_id)
        .despawn_children()
        .with_children(|board| {
            board.spawn(text("Results", Size::H1, Weight::Bold, Tone::CREAM_100));
            board.spawn(text(
                track.def().name,
                Size::BODY,
                Weight::Medium,
                Tone::CREAM_300,
            ));
            for (kart, progress, finished, human) in rows {
                let time = finished.map_or_else(|| "--:--.--".to_string(), |f| clock_text(f.time));
                let mine = human.is_some();
                let (fill, ink) = if mine {
                    (Tone::PUMPKIN, Tone::BROWN_900)
                } else {
                    (Tone::BROWN_800, Tone::CREAM_100)
                };
                let place_ink = match (progress.place, mine) {
                    (1, false) => Tone::PUMPKIN,
                    _ => ink,
                };
                let name = match (human, solo) {
                    (Some(_), true) => format!("{} (you)", ROSTER[kart.racer].name),
                    (Some(h), false) => format!("{} (P{})", ROSTER[kart.racer].name, h.0 + 1),
                    (None, _) => ROSTER[kart.racer].name.to_string(),
                };
                board
                    .spawn((
                        Node {
                            width: px(800),
                            height: px(64),
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(px(Space::L)),
                            border_radius: BorderRadius::all(px(Space::RADIUS_S)),
                            ..default()
                        },
                        BackgroundColor(fill),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                width: px(112),
                                ..default()
                            },
                            children![text(
                                ordinal(progress.place),
                                Size::H2,
                                Weight::Bold,
                                place_ink
                            )],
                        ));
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                            children![text(name, Size::H2, Weight::SemiBold, ink)],
                        ));
                        row.spawn(text(time, Size::H2, Weight::SemiBold, ink));
                    });
            }
            board
                .spawn(Node {
                    column_gap: px(Space::XL),
                    margin: UiRect::top(px(Space::M)),
                    ..default()
                })
                .with_children(|hints| {
                    hints.spawn(prompt(Action::Confirm, "Race again"));
                    hints.spawn(prompt(Action::Alt, "Change track"));
                    hints.spawn(prompt(Action::Back, "Menu"));
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
