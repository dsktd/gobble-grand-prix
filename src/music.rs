//! Music: the player's own files from the music folder, else the bundled bluegrass
//! soundtrack (see CREDITS.md). Cues crossfade; the final lap speeds the race tune up.

use bevy::audio::AudioSinkPlayback;
use bevy::audio::Volume;
use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    //
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(Update, (choose, advance, mix).chain());
}

/// Seconds a cue takes to fade in or out.
const FADE: f32 = 1.5;
/// Playback speed on the final lap: about a semitone up and 6% faster.
const FINAL_LAP_SPEED: f32 = 1.06;
/// Seconds the final-lap lift takes to ramp in.
const LIFT: f32 = 2.0;

const MENU: &[u8] = include_bytes!("../assets/music/menu.ogg");
const RACE: [&[u8]; 3] = [
    include_bytes!("../assets/music/race-gone-fishin.ogg"),
    include_bytes!("../assets/music/race-banjo-something.ogg"),
    include_bytes!("../assets/music/race-backfoot.ogg"),
];

/// Which piece fits the moment.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Cue {
    Menu,
    Race,
}

#[derive(Resource, Clone, Debug)]
struct Library {
    menu: Vec<Handle<AudioSource>>,
    race: Vec<Handle<AudioSource>>,
    /// Played from the player's folder, so a list played through rather than one loop.
    playlist: bool,
    /// Bundled race tune for the next race; each race gets the next one.
    next_race: usize,
}

impl Library {
    fn list(
        &self,
        cue: Cue,
    ) -> &[Handle<AudioSource>] {
        match cue {
            Cue::Menu => &self.menu,
            Cue::Race => &self.race,
        }
    }
}

#[derive(Component, Copy, Clone, Debug)]
struct Track {
    cue: Cue,
    index: usize,
    /// Current fade gain, 0 to 1.
    gain: f32,
    /// Fading out, despawned once silent.
    leaving: bool,
    speed: f32,
}

impl Track {
    const fn new(
        cue: Cue,
        index: usize,
        gain: f32,
    ) -> Self {
        Self {
            cue,
            index,
            gain,
            leaving: false,
            speed: 1.0,
        }
    }
}

/// `$XDG_DATA_HOME/gobble-gp/music`, usually `~/.local/share/gobble-gp/music`.
fn folder() -> Option<std::path::PathBuf> {
    Some(dirs::data_dir()?.join("gobble-gp").join("music"))
}

fn playable(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map_or_default(str::to_lowercase);
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
                .map_or_default(str::to_lowercase);
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

    let mut bundled = |bytes: &'static [u8]| {
        sources.add(AudioSource {
            bytes: bytes.into(),
        })
    };
    if menu.is_empty() {
        menu.push(bundled(MENU));
    }
    if race.is_empty() {
        race = RACE.into_iter().map(&mut bundled).collect();
    }
    commands.insert_resource(Library {
        menu,
        race,
        playlist,
        next_race: 0,
    });
}

fn wanted(state: &GameState) -> Cue {
    if *state == GameState::Race {
        Cue::Race
    } else {
        Cue::Menu
    }
}

fn on_final_lap(karts: &Query<(&Progress, &Human)>) -> bool {
    karts
        .iter()
        .find(|(_, human)| human.0 == 0)
        .is_some_and(|(progress, _)| progress.lap == LAPS)
}

fn volume(
    options: &Options,
    gain: f32,
) -> Volume {
    Volume::Linear(f32::from(options.music) / 10.0 * f32::from(options.volume) / 10.0 * 0.7 * gain)
}

/// Start the right piece when the cue changes, fading the old one out.
fn choose(
    mut commands: Commands,
    mut library: ResMut<Library>,
    options: Res<Options>,
    state: Res<State<GameState>>,
    mut playing: Query<&mut Track>,
) {
    let cue = wanted(state.get());
    if playing
        .iter()
        .any(|track| track.cue == cue && !track.leaving)
    {
        return;
    }
    for mut track in &mut playing {
        track.leaving = true;
    }
    let (index, settings) = if library.playlist && cue == Cue::Race {
        (0, PlaybackSettings::REMOVE)
    } else {
        let index = match cue {
            Cue::Menu => 0,
            Cue::Race => library.next_race % library.race.len(),
        };
        (index, PlaybackSettings::LOOP)
    };
    if cue == Cue::Race {
        library.next_race += 1;
    }
    commands.spawn((
        Track::new(cue, index, 0.0),
        AudioPlayer(library.list(cue)[index].clone()),
        settings.with_volume(volume(&options, 0.0)),
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
        commands.entity(id).despawn();
        if track.leaving {
            continue;
        }
        let list = library.list(track.cue);
        let index = (track.index + 1) % list.len();
        commands.spawn((
            Track::new(track.cue, index, 1.0),
            AudioPlayer(list[index].clone()),
            PlaybackSettings::REMOVE.with_volume(volume(&options, 1.0)),
        ));
    }
}

/// Fades, the final-lap lift and the volume options.
fn mix(
    mut commands: Commands,
    time: Res<Time>,
    options: Res<Options>,
    karts: Query<(&Progress, &Human)>,
    mut tracks: Query<(Entity, &mut Track, Option<&mut AudioSink>)>,
) {
    let dt = time.delta_secs();
    let lift = on_final_lap(&karts);
    for (id, mut track, sink) in &mut tracks {
        let target = if track.leaving { 0.0 } else { 1.0 };
        track.gain = step(track.gain, target, dt / FADE);
        if track.leaving && track.gain == 0.0 {
            commands.entity(id).despawn();
            continue;
        }
        let speed = if lift && track.cue == Cue::Race {
            FINAL_LAP_SPEED
        } else {
            1.0
        };
        track.speed = step(track.speed, speed, dt * (FINAL_LAP_SPEED - 1.0) / LIFT);
        if let Some(mut sink) = sink {
            sink.set_volume(volume(&options, track.gain));
            sink.set_speed(track.speed);
        }
    }
}

/// Move `from` towards `to` by at most `by`.
fn step(
    from: f32,
    to: f32,
    by: f32,
) -> f32 {
    if from < to {
        (from + by).min(to)
    } else {
        (from - by).max(to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_reaches_without_overshoot() {
        assert_eq!(step(0.0, 1.0, 0.4), 0.4);
        assert_eq!(step(0.9, 1.0, 0.4), 1.0);
        assert_eq!(step(0.3, 0.0, 0.4), 0.0);
    }

    #[test]
    fn bundled_music_is_ogg() {
        for bytes in RACE.into_iter().chain([MENU]) {
            assert_eq!(&bytes[..4], b"OggS");
        }
    }
}
