//! Options persisted as JSON in the user data dir. Missing or corrupt means defaults.

use std::{fs, path::PathBuf, time::Duration};

use bevy::prelude::*;

use crate::Options;

const DEBOUNCE: Duration = Duration::from_secs(1);

/// The game's folder in the user data dir. A folder left by the game's old name is moved over
/// once, so options, music and the last-seen build survive the rename.
pub fn data_dir() -> Option<PathBuf> {
    let base = dirs::data_dir()?;
    let dir = base.join("super-slop-kart");
    let old = base.join("gobble-gp");
    if !cfg!(test) && !dir.exists() && old.is_dir() {
        let _ = fs::rename(old, &dir);
    }
    Some(dir)
}

fn path() -> Option<PathBuf> {
    Some(data_dir()?.join("options.json"))
}

fn parse(text: &str) -> Options {
    let mut o: Options = serde_json::from_str(text).unwrap_or_default();
    o.volume = o.volume.min(10);
    o.music = o.music.min(10);
    o.smoothing = o.smoothing.min(10);
    o
}

/// The saved options, or defaults. Never fails.
pub fn load() -> Options {
    if cfg!(test) {
        return Options::default();
    }
    let text = path().and_then(|p| fs::read_to_string(p).ok());
    text.map_or_else(Options::default, |t| parse(&t))
}

fn save(o: &Options) {
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(o) {
        let _ = fs::write(p, text);
    }
}

/// Writes the options a second after the last change.
pub fn autosave(
    options: Res<Options>,
    time: Res<Time>,
    mut dirty: Local<Option<Duration>>,
    mut first: Local<bool>,
) {
    if !*first {
        *first = true; // the initial insert counts as a change
        return;
    }
    if options.is_changed() {
        *dirty = Some(time.elapsed());
    }
    if dirty.is_some_and(|at| time.elapsed().saturating_sub(at) >= DEBOUNCE) {
        *dirty = None;
        save(&options);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Split;

    #[test]
    fn round_trip() {
        let o = Options {
            volume: 3,
            high_quality: false,
            smoothing: 9,
            shake: true,
            music: 0,
            split: Split::Diagonal,
        };
        let back = parse(&serde_json::to_string(&o).unwrap());
        assert_eq!(
            serde_json::to_string(&back).unwrap(),
            serde_json::to_string(&o).unwrap()
        );
    }

    #[test]
    fn corrupt_gives_defaults() {
        let d = serde_json::to_string(&Options::default()).unwrap();
        assert_eq!(serde_json::to_string(&parse("{not json")).unwrap(), d);
        assert_eq!(serde_json::to_string(&parse("")).unwrap(), d);
        assert_eq!(parse(r#"{"volume":200}"#).volume, 10);
    }
}
