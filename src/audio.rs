//! Synthesised sound effects and a kart engine hum. No audio files.

use bevy::audio::AudioSinkPlayback;
use bevy::audio::Volume;
use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::Sfx;
}

const RATE: u32 = 22050;

pub(super) fn plugin(app: &mut App) {
    app.add_message::<Sfx>();
    app.add_systems(Startup, setup);
    app.add_systems(Update, (play, volume));
    app.add_systems(OnEnter(GameState::Race), engine_start.after(spawn_track));
    app.add_systems(Update, engine.run_if(in_state(GameState::Race)));
}

#[derive(Message, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Sfx {
    Beep,
    Go,
    Pickup,
    Throw,
    Splat,
    Hit,
    Boost,
    Turbo,
    Fanfare,
    Click,
    /// The cat's MRAOW.
    Wail,
}

#[derive(Resource, Clone, Debug)]
struct Sounds {
    clips: Vec<(Sfx, Handle<AudioSource>)>,
    engine: Handle<AudioSource>,
}

#[derive(Component, Copy, Clone, Debug)]
struct Engine;

/// Encode mono samples in -1..1 as a 16 bit WAV file.
fn wav(samples: &[f32]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend(b"RIFF");
    out.extend((36 + data).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(RATE.to_le_bytes());
    out.extend((RATE * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(data.to_le_bytes());
    for s in samples {
        out.extend(((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
    }
    out
}

/// Render `seconds` of audio from a function of time and progress (0..1).
fn render(
    seconds: f32,
    voice: impl Fn(f32, f32) -> f32,
) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let p = i as f32 / n as f32;
            // Short fade in and out to avoid clicks
            let edge = (p * 60.0).min(1.0) * ((1.0 - p) * 30.0).min(1.0);
            voice(t, p) * edge
        })
        .collect()
}

fn sine(phase: f32) -> f32 {
    (phase * std::f32::consts::TAU).sin()
}

fn square(phase: f32) -> f32 {
    if phase.fract() < 0.5 { 0.6 } else { -0.6 }
}

/// Cheap deterministic noise.
fn noise(t: f32) -> f32 {
    let x = (t * 12_345.679).sin() * 43_758.547;
    x.fract() * 2.0 - 1.0
}

fn notes(
    freqs: &[f32],
    each: f32,
) -> Vec<f32> {
    freqs
        .iter()
        .flat_map(|f| {
            render(each, |t, p| {
                (square(t * f) * 0.4 + sine(t * f * 2.0) * 0.3) * (1.0 - p * 0.7)
            })
        })
        .collect()
}

fn clip(sfx: Sfx) -> Vec<f32> {
    match sfx {
        Sfx::Beep => render(0.2, |t, p| sine(t * 440.0) * 0.6 * (1.0 - p)),
        Sfx::Go => {
            // A rising air-horn warble: warbling pitch with stutter
            render(0.7, |t, p| {
                let f = 330.0 + 140.0 * p + 60.0 * sine(t * 22.0);
                let stutter = 0.6 + 0.4 * sine(t * 11.0).abs();
                (square(t * f) * 0.5 + sine(t * f * 0.5) * 0.3) * stutter * (1.0 - p * 0.6)
            })
        }
        Sfx::Pickup => notes(&[660.0, 880.0, 1100.0], 0.07),
        Sfx::Throw => render(0.25, |t, p| {
            noise(t) * 0.35 * (1.0 - p) + sine(t * (600.0 - 400.0 * p)) * 0.2
        }),
        Sfx::Splat => render(0.3, |t, p| {
            noise(t * (1.0 - p * 0.8)) * 0.5 * (1.0 - p).powi(2)
        }),
        Sfx::Hit => render(0.45, |t, p| {
            square(t * (420.0 - 330.0 * p)) * 0.5 * (1.0 - p)
        }),
        Sfx::Boost => render(0.6, |t, p| {
            noise(t) * 0.25 * (1.0 - p) + square(t * (180.0 + 500.0 * p)) * 0.25
        }),
        Sfx::Turbo => render(0.35, |t, p| {
            square(t * (300.0 + 600.0 * p)) * 0.3 * (1.0 - p)
        }),
        Sfx::Fanfare => notes(&[523.0, 659.0, 784.0, 1047.0, 784.0, 1047.0], 0.13),
        Sfx::Click => render(0.05, |t, _| sine(t * 1200.0) * 0.4),
        Sfx::Wail => {
            // MRAOW: pitch swoops up then sags, a nasal buzz with sob vibrato
            render(0.9, |t, p| {
                let f = 420.0 + 520.0 * (p * std::f32::consts::PI).sin().powf(0.7) - 160.0 * p
                    + 25.0 * sine(t * 7.0);
                let body = (t * f).fract() * 2.0 - 1.0;
                let formant = sine(t * f * 2.0) * 0.5 + sine(t * f * 3.0) * 0.25;
                let open = (p * 6.0).min(1.0) * (1.0 - p).powf(0.8);
                (body * 0.25 + formant * 0.35) * open
            })
        }
    }
}

fn setup(
    mut commands: Commands,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let all = [
        Sfx::Beep,
        Sfx::Go,
        Sfx::Pickup,
        Sfx::Throw,
        Sfx::Splat,
        Sfx::Hit,
        Sfx::Boost,
        Sfx::Turbo,
        Sfx::Fanfare,
        Sfx::Click,
        Sfx::Wail,
    ];
    let clips = all
        .iter()
        .map(|sfx| {
            let bytes = wav(&clip(*sfx));
            (
                *sfx,
                sources.add(AudioSource {
                    bytes: bytes.into(),
                }),
            )
        })
        .collect();
    // 0.5 s of a low buzz with whole cycles so the loop is seamless
    let engine = render(0.5, |t, _| {
        square(t * 70.0) * 0.25 + sine(t * 140.0) * 0.2 + sine(t * 210.0) * 0.08
    });
    let engine = sources.add(AudioSource {
        bytes: wav(&engine).into(),
    });
    commands.insert_resource(Sounds { clips, engine });
}

fn play(
    mut commands: Commands,
    sounds: Res<Sounds>,
    setup: Res<Setup>,
    state: Res<State<GameState>>,
    attract: Option<Res<Attract>>,
    mut messages: MessageReader<Sfx>,
) {
    // A crying cat in P1's seat wails at the start, when hit, and when using items
    let cat = *state.get() == GameState::Race && ROSTER[setup.racers[0]].body == Body::Cat;
    let mut wail = false;
    for sfx in messages.read() {
        wail |= cat
            && matches!(
                sfx,
                Sfx::Go | Sfx::Hit | Sfx::Throw | Sfx::Splat | Sfx::Boost
            );
        let Some((_, handle)) = sounds.clips.iter().find(|(s, _)| s == sfx) else {
            continue;
        };
        // The race behind the menus stays in the background
        let settings = if attract.is_some() && *sfx != Sfx::Click {
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(Attract::DUCK))
        } else {
            PlaybackSettings::DESPAWN
        };
        commands.spawn((AudioPlayer(handle.clone()), settings));
    }
    if wail && let Some((_, handle)) = sounds.clips.iter().find(|(s, _)| *s == Sfx::Wail) {
        commands.spawn((AudioPlayer(handle.clone()), PlaybackSettings::DESPAWN));
    }
}

fn volume(
    options: Res<Options>,
    mut global: ResMut<GlobalVolume>,
) {
    if options.is_changed() {
        global.volume = Volume::Linear(f32::from(options.volume) / 10.0);
    }
}

fn engine_start(
    mut commands: Commands,
    sounds: Res<Sounds>,
) {
    commands.spawn((
        Engine,
        AudioPlayer(sounds.engine.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.25)),
        DespawnOnExit(GameState::Race),
    ));
}

/// Pitch the hum with P1's speed, and chirp when a mini-turbo fires.
fn engine(
    sinks: Query<&AudioSink, With<Engine>>,
    karts: Query<(&Kart, &Human)>,
    mut sfx: MessageWriter<Sfx>,
) {
    let Some((kart, _)) = karts.iter().find(|(_, human)| human.0 == 0) else {
        return;
    };
    if kart.fired > 0 {
        sfx.write(Sfx::Turbo);
    }
    for sink in &sinks {
        sink.set_speed(0.7 + kart.speed.abs() / Kart::TOP_SPEED * 1.2);
    }
}
