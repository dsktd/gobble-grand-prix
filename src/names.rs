//! A display name for every seat: the phone's chosen name, the Steam persona for the Deck's own
//! controls, a pad's product name, or a themed fallback. Never "Player 2".

use bevy::prelude::*;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::SeatNames;
    pub use super::sanitize;
}

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(LocalName(local_name()));
    app.init_resource::<SeatNames>();
    app.add_systems(PreUpdate, name_seats);
}

/// Names in seat order, rebuilt whenever the seats or phones change.
#[derive(Resource, Clone, Debug, Default)]
pub struct SeatNames(pub Vec<String>);
impl SeatNames {
    pub fn get(
        &self,
        slot: usize,
    ) -> String {
        self.0.get(slot).cloned().unwrap_or_else(|| fallback(slot))
    }
}

/// The person at this machine: Steam persona on the Deck, else the login name.
#[derive(Resource, Clone, Debug)]
struct LocalName(Option<String>);

/// Longest name shown anywhere.
const MAX: usize = 12;

/// Printable characters only: no control or bidi marks, trimmed, at most [`MAX`] characters.
pub fn sanitize(raw: &str) -> Option<String> {
    let bidi = |c: char| matches!(c, '\u{200E}' | '\u{200F}' | '\u{061C}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}');
    let clean: String = raw
        .chars()
        .filter(|c| !c.is_control() && !bidi(*c) && *c != '\u{FEFF}')
        .collect();
    let clean: String = clean.trim().chars().take(MAX).collect();
    let clean = clean.trim().to_string();
    (!clean.is_empty()).then_some(clean)
}

const THEMED: [&str; 4] = ["Chalk", "Cue", "Rack", "Scratch"];

fn fallback(slot: usize) -> String {
    format!("{} {}", THEMED[slot % THEMED.len()], slot + 1)
}

/// Most recent Steam login's persona name, read locally from loginusers.vdf.
fn steam_persona() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::Path::new(&home).join(".steam/steam/config/loginusers.vdf");
    let text = std::fs::read_to_string(path).ok()?;
    let mut persona = None;
    let mut first = None;
    for block in text.split('}') {
        let value = |key: &str| {
            let at = block.find(&format!("\"{key}\""))? + key.len() + 2;
            let rest = &block[at..];
            let start = rest.find('"')? + 1;
            let end = rest[start..].find('"')? + start;
            Some(rest[start..end].to_string())
        };
        let Some(name) = value("PersonaName") else {
            continue;
        };
        if value("MostRecent").as_deref() == Some("1") {
            persona = Some(name);
        } else if first.is_none() {
            first = Some(name);
        }
    }
    persona.or(first)
}

fn local_name() -> Option<String> {
    steam_persona()
        .and_then(|n| sanitize(&n))
        .or_else(|| std::env::var("USER").ok().and_then(|n| sanitize(&n)))
}

/// Shorten a pad's product name: "Xbox Wireless Controller" -> "Xbox pad".
fn pad_name(product: &str) -> Option<String> {
    let lower = product.to_lowercase();
    let short = if lower.contains("xbox") || lower.contains("x-box") {
        "Xbox pad".to_string()
    } else if lower.contains("8bitdo") {
        "8BitDo".to_string()
    } else if lower.contains("dualsense")
        || lower.contains("dualshock")
        || lower.contains("playstation")
    {
        "PS pad".to_string()
    } else if lower.contains("steam") {
        "Steam pad".to_string()
    } else {
        product
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ")
    };
    sanitize(&short)
}

fn name_seats(
    seats: Res<Seats>,
    phones: Res<Phones>,
    local: Res<LocalName>,
    pads: Query<&Name, With<Gamepad>>,
    mut names: ResMut<SeatNames>,
) {
    if !seats.is_changed() && !phones.is_changed() && !names.0.is_empty() {
        return;
    }
    let mut local_used = false;
    let fresh: Vec<String> = seats
        .0
        .iter()
        .enumerate()
        .map(|(slot, device)| {
            let found = match device {
                Device::Phone(id) => phones.name(*id),
                Device::Any | Device::Keyboard if !local_used => {
                    local_used = true;
                    local.0.clone()
                }
                Device::Gamepad(entity) if !local_used && seats.0.len() == 1 => {
                    local_used = true;
                    local.0.clone()
                }
                Device::Gamepad(entity) => {
                    pads.get(*entity).ok().and_then(|n| pad_name(n.as_str()))
                }
                _ => None,
            };
            found.unwrap_or_else(|| fallback(slot))
        })
        .collect();
    if names.0 != fresh {
        names.0 = fresh;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sanitize() {
        assert_eq!(sanitize("  Cal  ").as_deref(), Some("Cal"));
        assert_eq!(sanitize("a\u{202E}b\nc\u{0007}").as_deref(), Some("abc"));
        assert_eq!(
            sanitize("abcdefghijklmnop").as_deref(),
            Some("abcdefghijkl")
        );
        assert_eq!(sanitize(" \u{2066} "), None);
    }

    #[test]
    fn test_fallback_never_player() {
        for slot in 0..4 {
            assert!(!fallback(slot).starts_with("Player"));
        }
    }
}
