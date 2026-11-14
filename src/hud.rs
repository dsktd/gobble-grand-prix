//! Race HUD: place, lap, timer, item slot, countdown, and the results board.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Race), spawn);
    app.add_systems(
        Update,
        (stats, banner, results, burst)
            .after(KartSet::Race)
            .run_if(in_state(GameState::Race)),
    );
}

/// Which piece of a seat's HUD a text node shows.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq)]
enum Readout {
    Place,
    Lap,
    Time,
    Item,
}

/// HUD node belonging to the human in this seat.
#[derive(Component, Copy, Clone, Debug)]
struct Seat(usize);

#[derive(Component, Copy, Clone, Debug)]
struct Banner;

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
            let r = (dx * dx + dy * dy).sqrt();
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

#[derive(Component, Copy, Clone, Debug)]
struct Board;

fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn clock_text(t: f32) -> String {
    format!("{}:{:05.2}", (t / 60.0) as u32, t % 60.0)
}

fn spawn(
    mut commands: Commands,
    setup: Res<Setup>,
    mut images: ResMut<Assets<Image>>,
) {
    let scope = DespawnOnExit(GameState::Race);
    let lines = images.add(burst_image());
    for slot in 0..setup.racers.len() {
        commands.spawn((
            Burst { slot, left: 0.0 },
            ImageNode::new(lines.clone()),
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                ..default()
            },
            Visibility::Hidden,
            scope.clone(),
        ));
    }
    // One HUD per seat. With split screen these get their viewport's rect.
    for slot in 0..setup.racers.len() {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(px(Size::PAD)),
                    ..default()
                },
                scope.clone(),
            ))
            .with_children(|root| {
                root.spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|top| {
                    top.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    })
                    .with_children(|left| {
                        left.spawn((Seat(slot), Readout::Lap, label("", Size::HUD, Tone::CREAM)));
                        left.spawn((Seat(slot), Readout::Time, label("", Size::HUD, Tone::CREAM)));
                    });
                    top.spawn((
                        Seat(slot),
                        Readout::Place,
                        label("", Size::PLACE, Tone::GOLD),
                    ));
                });
                root.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: px(Size::PAD),
                    ..default()
                })
                .with_children(|bottom| {
                    bottom.spawn(prompt(Action::Drift, "Drift"));
                    bottom
                        .spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: px(6),
                            ..default()
                        })
                        .with_children(|item| {
                            item.spawn(prompt(Action::Item, ""));
                            item.spawn((
                                Seat(slot),
                                Readout::Item,
                                label("", Size::BODY, Tone::GOLD),
                            ));
                        });
                });
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
            root.spawn((Banner, label("", Size::BANNER, Tone::GOLD)));
        });

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
            scope,
        ))
        .with_children(|root| {
            root.spawn((Board, panel(), Visibility::Hidden));
        });
}

fn stats(
    clock: Res<RaceClock>,
    karts: Query<(&Kart, &Progress, &Human, Option<&Finished>)>,
    readouts: Query<(&Seat, &Readout, &mut Text, &mut TextColor)>,
) {
    for (seat, readout, mut text, mut color) in readouts {
        let Some((kart, progress, _, finished)) = karts.iter().find(|k| k.2.0 == seat.0) else {
            continue;
        };
        let shown = match readout {
            Readout::Place => ordinal(progress.place),
            Readout::Lap => match finished {
                Some(_) => "Finished!".to_string(),
                None => format!("Lap {}/{LAPS}", progress.lap.clamp(1, LAPS)),
            },
            Readout::Time => clock_text(finished.map_or(clock.time, |f| f.time)),
            Readout::Item => match kart.item {
                Some(item) => {
                    color.0 = item.color().mix(&Color::WHITE, 0.4);
                    item.name().to_string()
                }
                None => {
                    color.0 = Tone::DIM;
                    "no item".to_string()
                }
            },
        };
        if text.0 != shown {
            text.0 = shown;
        }
    }
}

fn banner(
    clock: Res<RaceClock>,
    time: Res<Time<Virtual>>,
    humans: Query<Option<&Finished>, With<Human>>,
    banners: Query<&mut Text, With<Banner>>,
) {
    let finished = humans.iter().any(|f| f.is_some());
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
            text.0 = shown.clone();
        }
    }
}

fn results(
    mut commands: Commands,
    clock: Res<RaceClock>,
    karts: Query<(&Kart, &Progress, Option<&Finished>, Has<Human>)>,
    mut board: Query<(Entity, &mut Visibility), With<Board>>,
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
    commands
        .entity(board_id)
        .despawn_children()
        .with_children(|board| {
            board.spawn(label("Results", Size::HEADING, Tone::GOLD));
            for (kart, progress, finished, human) in rows {
                let time = match finished {
                    Some(f) => clock_text(f.time),
                    None => "--:--.--".to_string(),
                };
                let color = match human {
                    true => Tone::GOLD,
                    false => Tone::CREAM,
                };
                board
                    .spawn(Node {
                        column_gap: px(Size::PAD),
                        ..default()
                    })
                    .with_children(|row| {
                        let cell = |width: f32| Node {
                            width: px(width),
                            ..default()
                        };
                        row.spawn(cell(60.0)).with_child(label(
                            ordinal(progress.place),
                            Size::BODY,
                            color,
                        ));
                        row.spawn(cell(240.0)).with_child(label(
                            ROSTER[kart.racer].name,
                            Size::BODY,
                            color,
                        ));
                        row.spawn(cell(130.0))
                            .with_child(label(time, Size::BODY, color));
                    });
            }
            board.spawn(prompt(Action::Confirm, "Back to menu"));
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
        *visibility = match burst.left > 0.0 {
            true => Visibility::Inherited,
            false => Visibility::Hidden,
        };
        image.color = Color::WHITE.with_alpha(burst.left / Burst::TIME);
    }
}
