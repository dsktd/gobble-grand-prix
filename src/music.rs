//! Music: the player's own files from the music folder, else an original synthesised
//! harvest-fair soundtrack (fiddle, banjo, bouncy bass, snare shuffle).

use bevy::audio::AudioSinkPlayback;
use bevy::audio::Volume;
use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(Update, (choose, advance, level));
}

/// Which piece fits the moment.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Cue {
    Menu,
    Race,
    FinalLap,
}

#[derive(Resource, Clone, Debug)]
struct Library {
    menu: Vec<Handle<AudioSource>>,
    race: Vec<Handle<AudioSource>>,
    final_lap: Vec<Handle<AudioSource>>,
    /// Played from the player's folder, so a list rather than one loop.
    playlist: bool,
}

#[derive(Component, Copy, Clone, Debug)]
struct Track {
    cue: Cue,
    index: usize,
}

/// `$XDG_DATA_HOME/gobble-gp/music`, usually `~/.local/share/gobble-gp/music`.
fn folder() -> Option<std::path::PathBuf> {
    Some(dirs::data_dir()?.join("gobble-gp").join("music"))
}

fn playable(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();
    matches!(ext.as_str(), "ogg" | "mp3" | "flac" | "wav")
}

fn setup(
    mut commands: Commands,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let mut menu = Vec::new();
    let mut race = Vec::new();
    let mut other = Vec::new();
    if let Some(dir) = folder()
        && let Ok(entries) = std::fs::read_dir(&dir)
    {
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| playable(p))
            .collect();
        paths.sort();
        for path in paths {
            let Ok(bytes) = std::fs::read(&path) else {
                warn!("Cannot read {}", path.display());
                continue;
            };
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_lowercase)
                .unwrap_or_default();
            let handle = sources.add(AudioSource {
                bytes: bytes.into(),
            });
            match stem.as_str() {
                "menu" => menu.push(handle),
                "race" => race.push(handle),
                _ => other.push(handle),
            }
            info!("Music {}", path.display());
        }
    }
    // Unnamed files become the race playlist when nothing is called menu or race
    if menu.is_empty() && race.is_empty() {
        race = other;
    }
    let playlist = !race.is_empty();

    let mut synth = |song: Song| {
        sources.add(AudioSource {
            bytes: wav(&song.render()).into(),
        })
    };
    if menu.is_empty() {
        menu.push(synth(Song::MENU));
    }
    let final_lap = if race.is_empty() {
        vec![synth(Song::FINAL_LAP)]
    } else {
        race.clone()
    };
    if race.is_empty() {
        race.push(synth(Song::RACE));
    }
    commands.insert_resource(Library {
        menu,
        race,
        final_lap,
        playlist,
    });
}

fn wanted(
    state: &GameState,
    karts: &Query<(&Progress, &Human)>,
) -> Cue {
    if *state != GameState::Race {
        return Cue::Menu;
    }
    let last = karts
        .iter()
        .find(|(_, human)| human.0 == 0)
        .is_some_and(|(progress, _)| progress.lap == LAPS);
    if last { Cue::FinalLap } else { Cue::Race }
}

fn volume(options: &Options) -> Volume {
    Volume::Linear(f32::from(options.music) / 10.0 * f32::from(options.volume) / 10.0 * 0.6)
}

/// Swap the playing piece when the cue changes.
fn choose(
    mut commands: Commands,
    library: Res<Library>,
    options: Res<Options>,
    state: Res<State<GameState>>,
    karts: Query<(&Progress, &Human)>,
    playing: Query<(Entity, &Track)>,
) {
    let cue = wanted(state.get(), &karts);
    // Your own race playlist plays through the final lap too
    let cue = match (cue, library.playlist) {
        (Cue::FinalLap, true) => Cue::Race,
        _ => cue,
    };
    if playing.iter().any(|(_, track)| track.cue == cue) {
        return;
    }
    for (id, _) in &playing {
        commands.entity(id).despawn();
    }
    let list = match cue {
        Cue::Menu => &library.menu,
        Cue::Race => &library.race,
        Cue::FinalLap => &library.final_lap,
    };
    let looped = list.len() == 1;
    let settings = if looped {
        PlaybackSettings::LOOP
    } else {
        PlaybackSettings::REMOVE
    };
    commands.spawn((
        Track { cue, index: 0 },
        AudioPlayer(list[0].clone()),
        settings.with_volume(volume(&options)),
    ));
}

/// Move a playlist on when a song ends.
fn advance(
    mut commands: Commands,
    library: Res<Library>,
    options: Res<Options>,
    tracks: Query<(Entity, &Track), Without<AudioPlayer>>,
) {
    for (id, track) in &tracks {
        let list = match track.cue {
            Cue::Menu => &library.menu,
            Cue::Race => &library.race,
            Cue::FinalLap => &library.final_lap,
        };
        let index = (track.index + 1) % list.len();
        commands.entity(id).despawn();
        commands.spawn((
            Track {
                cue: track.cue,
                index,
            },
            AudioPlayer(list[index].clone()),
            PlaybackSettings::REMOVE.with_volume(volume(&options)),
        ));
    }
}

fn level(
    options: Res<Options>,
    sinks: Query<&mut AudioSink, With<Track>>,
) {
    if !options.is_changed() {
        return;
    }
    for mut sink in sinks {
        sink.set_volume(volume(&options));
    }
}

/// A synthesised piece: tempo, key shift and the parts it plays.
#[derive(Copy, Clone, Debug)]
struct Song {
    bpm: f32,
    /// Semitones above G.
    key: i32,
    /// Bars of the progression to play, as scale-degree roots (0 = I).
    chords: &'static [i32],
    /// Melody as scale degrees per eighth note; 99 is a rest.
    tune: &'static [i32],
    banjo: bool,
}

const REST: i32 = 99;

impl Song {
    const MENU: Self = Self {
        bpm: 104.0,
        key: 0,
        chords: &[0, 3, 4, 0, 5, 3, 4, 4],
        tune: &[
            4, REST, 2, 4, 5, 4, 2, 0, //
            3, REST, 5, 3, 7, 5, 3, REST, //
            4, 5, 6, 4, 8, 6, 4, 2, //
            2, REST, 4, 2, 0, REST, REST, REST, //
            5, 7, 9, 7, 5, 4, 2, 4, //
            3, 5, 7, 5, 3, 2, 0, 2, //
            1, 3, 4, 6, 8, 6, 4, 3, //
            4, REST, 1, REST, 4, 3, 1, REST,
        ],
        banjo: false,
    };
    const RACE: Self = Self {
        bpm: 138.0,
        key: 0,
        chords: &[0, 0, 3, 3, 4, 4, 0, 4],
        tune: &[
            0, 2, 4, 7, 6, 4, 2, 4, //
            7, 6, 7, 9, 7, 4, 2, 0, //
            3, 5, 7, 8, 7, 5, 3, 5, //
            8, 7, 5, 3, 5, 3, 1, 3, //
            4, 6, 8, 6, 4, 6, 8, 11, //
            8, 6, 4, 3, 1, 3, 4, 6, //
            7, 4, 2, 4, 7, 9, 7, 4, //
            1, 3, 4, 6, 4, 3, 1, REST,
        ],
        banjo: true,
    };
    /// The race tune, 10% faster and lifted a whole step.
    const FINAL_LAP: Self = Self {
        bpm: 138.0 * 1.1,
        key: 2,
        ..Self::RACE
    };

    fn render(&self) -> Vec<f32> {
        let rate = RATE as f32;
        let beat = 60.0 / self.bpm;
        let bars = self.chords.len();
        let length = (bars as f32 * 4.0 * beat * rate) as usize;
        let mut out = vec![0.0f32; length];
        let major = [0, 2, 4, 5, 7, 9, 11];
        // Scale degree to frequency, G3 = 196 Hz as degree 0
        let freq = |degree: i32, octave: i32| {
            let semis = major[degree.rem_euclid(7) as usize]
                + 12 * (degree.div_euclid(7) + octave)
                + self.key;
            196.0 * 2f32.powf(semis as f32 / 12.0)
        };
        let mut add = |start: f32, dur: f32, voice: &dyn Fn(f32, f32) -> f32| {
            let s0 = (start * rate) as usize;
            let n = (dur * rate) as usize;
            for i in 0..n {
                let at = (s0 + i) % length;
                let t = i as f32 / rate;
                out[at] += voice(t, i as f32 / n as f32);
            }
        };

        for (bar, root) in self.chords.iter().enumerate() {
            let t0 = bar as f32 * 4.0 * beat;
            for q in 0..4 {
                let tq = t0 + q as f32 * beat;
                // Bouncy bass: root, fifth, with a short octave pop
                let degree = match q % 2 {
                    0 => *root,
                    _ => *root + 4,
                };
                let f = freq(degree, -1);
                add(tq, beat * 0.7, &|t, p| {
                    let env = (1.0 - p).powf(1.5);
                    ((t * f).fract() * 2.0 - 1.0).signum() * 0.10 * env
                        + (t * f * std::f32::consts::TAU).sin() * 0.16 * env
                });
                // Kick on 1 and 3, snare on 2 and 4
                match q % 2 {
                    0 => add(tq, 0.18, &|t, p| {
                        (t * (110.0 - 70.0 * p) * std::f32::consts::TAU).sin() * 0.30 * (1.0 - p)
                    }),
                    _ => add(tq, 0.16, &|t, p| noise(t) * 0.16 * (1.0 - p).powi(2)),
                }
                // Shuffled ghost snare on the swung off-beat
                add(tq + beat * 0.66, 0.06, &|t, p| {
                    noise(t * 3.1) * 0.05 * (1.0 - p)
                });
            }
            // Banjo rolls: plucked chord tones in sixteenths
            if self.banjo {
                let tones = [*root, *root + 2, *root + 4, *root + 7];
                for s in 0..16 {
                    let f = freq(tones[(s * 3 + bar) % 4], 1);
                    add(t0 + s as f32 * beat / 4.0, beat * 0.5, &|t, p| {
                        pluck(t, f) * 0.07 * (1.0 - p).powi(3)
                    });
                }
            } else {
                // Gentle strum on the menu
                for (k, d) in [*root, *root + 2, *root + 4].iter().enumerate() {
                    let f = freq(*d, 0);
                    add(t0 + k as f32 * 0.03, beat * 3.5, &|t, p| {
                        pluck(t, f) * 0.06 * (1.0 - p).powi(2)
                    });
                }
            }
        }
        // Fiddle melody: bowed saw with vibrato and a soft attack
        for (i, degree) in self.tune.iter().enumerate() {
            if *degree == REST {
                continue;
            }
            let f = freq(*degree, 1);
            let start = i as f32 * beat / 2.0;
            add(start, beat * 0.48, &|t, p| {
                let vib =
                    1.0 + 0.006 * (t * 5.5 * std::f32::consts::TAU).sin() * (t * 4.0).min(1.0);
                let phase = (t * f * vib).fract();
                let saw = phase * 2.0 - 1.0;
                let body = saw * 0.6 + (phase * std::f32::consts::TAU * 2.0).sin() * 0.3;
                let env = (p * 12.0).min(1.0) * (1.0 - p).powf(0.5);
                body * 0.09 * env
            });
        }
        let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(0.001);
        out.iter().map(|s| s / peak * 0.8).collect()
    }
}

/// A banjo-ish pluck: bright partials that decay fast.
fn pluck(
    t: f32,
    f: f32,
) -> f32 {
    let tau = std::f32::consts::TAU;
    ((t * f * tau).sin()
        + (t * f * 2.0 * tau).sin() * 0.5 * (-t * 18.0).exp()
        + (t * f * 3.0 * tau).sin() * 0.3 * (-t * 30.0).exp())
        * (-t * 6.0).exp()
}

/// Cheap deterministic noise.
fn noise(t: f32) -> f32 {
    let x = (t * 12_345.679).sin() * 43_758.547;
    x.fract() * 2.0 - 1.0
}
