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
        (stats, banner, results)
            .after(KartSet::Race)
            .run_if(in_state(GameState::Race)),
    );
}

const GOLD: Color = Color::srgb(1.0, 0.82, 0.35);
const CREAM: Color = Color::srgb(0.98, 0.93, 0.82);

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

fn text(
    value: &str,
    size: f32,
    color: Color,
) -> (Text, TextFont, TextColor, TextShadow) {
    (
        Text::new(value),
        TextFont::from_font_size(size),
        TextColor(color),
        TextShadow::default(),
    )
}

fn spawn(
    mut commands: Commands,
    setup: Res<Setup>,
) {
    let scope = DespawnOnExit(GameState::Race);
    // One HUD per seat. With split screen these get their viewport's rect.
    for slot in 0..setup.racers.len() {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(px(24)),
                    ..default()
                },
                scope.clone(),
            ))
            .with_children(|root| {
                root.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|left| {
                    left.spawn((Seat(slot), Readout::Lap, text("", 40.0, CREAM)));
                    left.spawn((Seat(slot), Readout::Time, text("", 32.0, CREAM)));
                    left.spawn((Seat(slot), Readout::Item, text("", 36.0, GOLD)));
                });
                root.spawn((Seat(slot), Readout::Place, text("", 96.0, GOLD)));
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
            root.spawn((Banner, text("", 140.0, GOLD)));
        });

    commands.spawn((
        Board,
        Node {
            position_type: PositionType::Absolute,
            left: percent(25),
            top: percent(15),
            width: percent(50),
            padding: UiRect::all(px(28)),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
        BackgroundColor(Color::srgba(0.25, 0.10, 0.04, 0.85)),
        Visibility::Hidden,
        scope,
    ));
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
                    format!("[ {} ]", item.name())
                }
                None => {
                    color.0 = CREAM.with_alpha(0.5);
                    "[ empty ]".to_string()
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
    humans: Query<Option<&Finished>, With<Human>>,
    banners: Query<&mut Text, With<Banner>>,
) {
    let finished = humans.iter().any(|f| f.is_some());
    let shown = match clock.phase {
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
    commands.entity(board_id).despawn_children().with_children(|board| {
        board.spawn(text("Results", 56.0, GOLD));
        for (kart, progress, finished, human) in rows {
            let time = match finished {
                Some(f) => clock_text(f.time),
                None => "--:--.--".to_string(),
            };
            let color = match human {
                true => GOLD,
                false => CREAM,
            };
            board.spawn(text(
                &format!(
                    "{:<5} {:<16} {}",
                    ordinal(progress.place),
                    ROSTER[kart.racer].name,
                    time
                ),
                32.0,
                color,
            ));
        }
        board.spawn(text("A / Enter: back to menu", 24.0, CREAM));
    });
}
