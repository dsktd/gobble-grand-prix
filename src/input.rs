//! Keyboard and gamepad mapping, for menus and for each local seat.

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::MenuInput;
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Seats>();
    app.init_resource::<MenuInput>();
    app.add_systems(PreUpdate, menu.after(bevy::input::InputSystems));
    app.add_systems(Update, drive.in_set(KartSet::Input));
}

/// The device behind a local seat. Keyboard and Gamepad seats arrive with split screen.
#[allow(unused)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Device {
    /// Keyboard plus every connected pad, for single player.
    Any,
    Keyboard,
    Gamepad(Entity),
}

/// Local seats in slot order. Split screen pushes more of them.
#[derive(Resource, Clone, Debug)]
pub struct Seats(pub Vec<Device>);
impl Default for Seats {
    fn default() -> Self {
        Self(vec![Device::Any])
    }
}

/// Menu actions pressed this frame by any device.
#[derive(Resource, Copy, Clone, Debug, Default)]
pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub back: bool,
}

fn menu(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut input: ResMut<MenuInput>,
    mut held: Local<IVec2>,
) {
    let any = |codes: &[KeyCode]| codes.iter().any(|c| keys.just_pressed(*c));
    let mut next = MenuInput {
        up: any(&[KeyCode::ArrowUp, KeyCode::KeyW]),
        down: any(&[KeyCode::ArrowDown, KeyCode::KeyS]),
        left: any(&[KeyCode::ArrowLeft, KeyCode::KeyA]),
        right: any(&[KeyCode::ArrowRight, KeyCode::KeyD]),
        confirm: any(&[KeyCode::Enter, KeyCode::Space]),
        back: any(&[KeyCode::Escape, KeyCode::Backspace]),
    };

    // Stick flicks count once until the stick returns to center
    let mut stick = IVec2::ZERO;
    for gamepad in &gamepads {
        next.up |= gamepad.just_pressed(GamepadButton::DPadUp);
        next.down |= gamepad.just_pressed(GamepadButton::DPadDown);
        next.left |= gamepad.just_pressed(GamepadButton::DPadLeft);
        next.right |= gamepad.just_pressed(GamepadButton::DPadRight);
        next.confirm |= gamepad.just_pressed(GamepadButton::South)
            || gamepad.just_pressed(GamepadButton::Start);
        next.back |= gamepad.just_pressed(GamepadButton::East);
        let s = gamepad.left_stick();
        if s.x.abs() > 0.6 {
            stick.x = s.x.signum() as i32;
        }
        if s.y.abs() > 0.6 {
            stick.y = s.y.signum() as i32;
        }
    }
    if stick != *held {
        next.left |= stick.x < 0 && held.x >= 0;
        next.right |= stick.x > 0 && held.x <= 0;
        next.up |= stick.y > 0 && held.y <= 0;
        next.down |= stick.y < 0 && held.y >= 0;
        *held = stick;
    }
    *input = next;
}

fn keyboard(keys: &ButtonInput<KeyCode>) -> KartInput {
    let axis = |neg: &[KeyCode], pos: &[KeyCode]| {
        let n = neg.iter().any(|k| keys.pressed(*k)) as i32 as f32;
        let p = pos.iter().any(|k| keys.pressed(*k)) as i32 as f32;
        p - n
    };
    KartInput {
        throttle: axis(
            &[KeyCode::ArrowDown, KeyCode::KeyS],
            &[KeyCode::ArrowUp, KeyCode::KeyW],
        ),
        steer: axis(
            &[KeyCode::ArrowRight, KeyCode::KeyD],
            &[KeyCode::ArrowLeft, KeyCode::KeyA],
        ),
        drift: keys.pressed(KeyCode::Space),
        item: keys.just_pressed(KeyCode::ShiftLeft)
            || keys.just_pressed(KeyCode::ShiftRight)
            || keys.just_pressed(KeyCode::KeyE),
    }
}

/// A = gas, X or LT = brake, RB or LB = drift, B or Y = item.
fn pad(gamepad: &Gamepad) -> KartInput {
    let stick = gamepad.left_stick().x;
    let dpad = gamepad.pressed(GamepadButton::DPadLeft) as i32 as f32
        - gamepad.pressed(GamepadButton::DPadRight) as i32 as f32;
    let steer = match stick.abs() > 0.15 {
        true => -stick,
        false => dpad,
    };
    let gas = gamepad.pressed(GamepadButton::South)
        || gamepad.get(GamepadButton::RightTrigger2).unwrap_or(0.0) > 0.3;
    let brake = gamepad.pressed(GamepadButton::West)
        || gamepad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0) > 0.3;
    KartInput {
        throttle: gas as i32 as f32 - brake as i32 as f32,
        steer,
        drift: gamepad.pressed(GamepadButton::RightTrigger)
            || gamepad.pressed(GamepadButton::LeftTrigger),
        item: gamepad.just_pressed(GamepadButton::East)
            || gamepad.just_pressed(GamepadButton::North),
    }
}

fn merge(
    a: KartInput,
    b: KartInput,
) -> KartInput {
    let pick = |x: f32, y: f32| match x.abs() >= y.abs() {
        true => x,
        false => y,
    };
    KartInput {
        throttle: pick(a.throttle, b.throttle),
        steer: pick(a.steer, b.steer),
        drift: a.drift || b.drift,
        item: a.item || b.item,
    }
}

fn drive(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    seats: Res<Seats>,
    karts: Query<(&Driver, &mut KartInput)>,
) {
    for (driver, mut input) in karts {
        let Driver::Local(slot) = driver else {
            continue;
        };
        let Some(device) = seats.0.get(*slot) else {
            continue;
        };
        *input = match device {
            Device::Keyboard => keyboard(&keys),
            Device::Gamepad(id) => match gamepads.get(*id) {
                Ok((_, gamepad)) => pad(gamepad),
                Err(_) => KartInput::default(),
            },
            Device::Any => gamepads
                .iter()
                .fold(keyboard(&keys), |acc, (_, gamepad)| merge(acc, pad(gamepad))),
        };
    }
}
