//! Headless layout test: every screen and every HUD, at three window sizes, with the real
//! strings and with stress strings. A text node that leaves its container, or its own box, fails.

use std::sync::atomic::Ordering;

use bevy::app::ScheduleRunnerPlugin;
use bevy::audio::AudioPlugin;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;

use crate::hud::ResultRow;
use crate::prelude::*;
use crate::style::STRESS;

const SIZES: [(u32, u32); 3] = [(1280, 800), (1920, 1080), (800, 600)];

fn app(
    size: (u32, u32),
    split: Split,
) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<WinitPlugin>()
            .disable::<AudioPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            }),
    );
    app.add_plugins(ScheduleRunnerPlugin::default());
    app.add_message::<Sfx>();
    app.init_state::<GameState>();
    app.init_resource::<Setup>();
    app.insert_resource(Options { split, ..default() });
    app.init_resource::<Phones>();
    app.init_resource::<SeatNames>();
    app.insert_resource(RaceClock {
        phase: Phase::Countdown,
        countdown: 5.0,
        time: 0.0,
        after: 0.0,
    });
    app.insert_resource(Track::new(0));
    app.add_plugins((
        crate::style::plugin,
        crate::prompts::plugin,
        crate::input::plugin,
        crate::menu::plugin,
        crate::hud::plugin,
    ));
    app.insert_resource(UiScale((size.0 as f32 / 1280.0).min(size.1 as f32 / 800.0)));
    app.world_mut().spawn((
        Camera2d,
        bevy::ui::IsDefaultUiCamera,
        RenderTarget::None {
            size: UVec2::new(size.0, size.1),
        },
    ));
    app.finish();
    app.cleanup();
    // The camera needs a few frames before UI percentages resolve
    frames(&mut app, 4);
    app
}

fn frames(
    app: &mut App,
    n: usize,
) {
    for _ in 0..n {
        app.update();
    }
}

fn rect(
    node: &ComputedNode,
    at: &UiGlobalTransform,
) -> Rect {
    Rect::from_center_size(at.translation, node.size())
}

/// Every text node must sit inside every ancestor and inside the window, and inside its own box.
fn check(
    app: &mut App,
    what: &str,
    size: (u32, u32),
) -> Vec<String> {
    let world = app.world_mut();
    let window = Rect::new(0.0, 0.0, size.0 as f32, size.1 as f32);
    let tol = 1.5;
    let mut problems = Vec::new();
    let mut texts = world.query::<(Entity, &Text, &ComputedNode, &UiGlobalTransform)>();
    let found: Vec<_> = texts
        .iter(world)
        .map(|(e, t, n, g)| (e, t.0.clone(), *n, *g))
        .collect();
    let mut count = 0;
    for (entity, words, node, at) in found {
        if words.is_empty() {
            continue;
        }
        count += 1;
        let me = rect(&node, &at);
        let loose = |outer: Rect| {
            me.min.x < outer.min.x - tol
                || me.min.y < outer.min.y - tol
                || me.max.x > outer.max.x + tol
                || me.max.y > outer.max.y + tol
        };
        if loose(window) {
            problems.push(format!("{what}: {words:?} {me:?} leaves the window"));
        }
        let content = node.content_size();
        if content.x > node.size().x + tol || content.y > node.size().y + tol {
            problems.push(format!(
                "{what}: {words:?} content {content:?} bigger than its box {:?}",
                node.size()
            ));
        }
        let mut up = entity;
        while let Some(parent) = world.get::<ChildOf>(up).map(ChildOf::parent) {
            up = parent;
            let (Some(n), Some(g)) = (
                world.get::<ComputedNode>(up),
                world.get::<UiGlobalTransform>(up),
            ) else {
                continue;
            };
            if loose(rect(n, g)) {
                problems.push(format!(
                    "{what}: {words:?} {me:?} overflows its container {:?}",
                    rect(n, g)
                ));
                break;
            }
        }
    }
    assert!(count > 0, "{what}: no text was laid out");
    problems
}

/// Diagonal split: every HUD text must sit wholly on one side of the corner-to-corner line,
/// so no chip or minimap label is cut by the divider. The one race-wide banner (countdown, track
/// name) spans both views in every split and is left out.
fn inside_triangles(
    app: &mut App,
    size: (u32, u32),
) -> Vec<String> {
    let (w, h) = (size.0 as f32, size.1 as f32);
    // Signed distance to the top-left to bottom-right line, in pixels
    let side = |p: Vec2| (p.y * w - p.x * h) / w.hypot(h);
    let world = app.world_mut();
    let mut texts = world
        .query_filtered::<(&Text, &ComputedNode, &UiGlobalTransform), Without<crate::hud::Banner>>(
        );
    let mut problems = Vec::new();
    for (text, node, at) in texts.iter(world) {
        if text.0.is_empty() {
            continue;
        }
        let r = rect(node, at);
        let d = [
            r.min,
            r.max,
            Vec2::new(r.min.x, r.max.y),
            Vec2::new(r.max.x, r.min.y),
        ]
        .map(side);
        let (lo, hi) = d
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
        if lo < -1.5 && hi > 1.5 {
            problems.push(format!(
                "{}x{} diagonal: {:?} {r:?} crosses the divider",
                size.0, size.1, text.0
            ));
        }
    }
    problems
}

fn go(
    app: &mut App,
    state: GameState,
) {
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(state);
    frames(app, 6);
}

#[test]
fn no_text_overflows() {
    let mut problems = Vec::new();
    for stress in [false, true] {
        STRESS.store(stress, Ordering::Relaxed);
        for size in SIZES {
            let tag = |screen: &str| format!("{}x{} stress={stress} {screen}", size.0, size.1);
            let mut app = app(size, Split::TopBottom);
            app.insert_resource(JoinInfo {
                url: "https://192-168-100-200.example-very-long-host-name.local:7777/join".into(),
                pin: "123456".into(),
                qr: Handle::default(),
                anywhere: Some(
                    "https://example.github.io/pad/#endpoint-with-a-very-long-ticket".into(),
                ),
            });
            for (name, state) in [
                ("title", GameState::Menu),
                ("options", GameState::Options),
                ("lobby", GameState::Lobby),
                ("racers", GameState::PickRacer),
                ("tracks", GameState::PickTrack),
            ] {
                go(&mut app, state);
                problems.extend(check(&mut app, &tag(name), size));
            }
            // Two to four seats on the racer screen, some on the same racer
            for seats in 2..=4 {
                app.world_mut().resource_mut::<Setup>().racers =
                    (0..seats).map(|s| s % 3).collect();
                go(&mut app, GameState::Menu);
                go(&mut app, GameState::PickRacer);
                problems.extend(check(&mut app, &tag(&format!("racers x{seats}")), size));
            }
            app.world_mut().resource_mut::<Setup>().racers = vec![0];

            for seats in 1..=4 {
                for split in [Split::TopBottom, Split::SideBySide, Split::Diagonal] {
                    if seats != 2 && split != Split::TopBottom {
                        continue;
                    }
                    let mut app = self::app(size, split);
                    app.world_mut().resource_mut::<Setup>().racers = (0..seats).collect();
                    go(&mut app, GameState::Race);
                    problems.extend(check(
                        &mut app,
                        &tag(&format!("hud x{seats} {split:?}")),
                        size,
                    ));
                    if seats == 2 && split == Split::Diagonal {
                        problems.extend(inside_triangles(&mut app, size));
                    }
                    if seats == 4 {
                        results(&mut app);
                        frames(&mut app, 4);
                        problems.extend(check(&mut app, &tag("results"), size));
                    }
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} overflows:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

fn results(app: &mut App) {
    let board = app
        .world_mut()
        .query_filtered::<Entity, With<crate::hud::Board>>()
        .single(app.world())
        .expect("board");
    let rows: Vec<ResultRow> = (0..7)
        .map(|i| ResultRow {
            place: i + 1,
            name: "Wailing Whiskers (P4) WWWWWWWWWWWW".into(),
            time: "12:34.56".into(),
            mine: i == 2,
        })
        .collect();
    let world = app.world_mut();
    let mut commands = world.commands();
    crate::hud::spawn_results(&mut commands, board, "Harvest Lane of Wwwwwwwwwwww", &rows);
    world.flush();
    *world.get_mut::<Visibility>(board).expect("visibility") = Visibility::Inherited;
}
