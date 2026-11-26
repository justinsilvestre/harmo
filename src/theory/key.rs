//! Keys and scale degrees.

use crate::pitch::{Tpc, letter_index, tpc_from_name, tpc_name};
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub enum Mode {
    Major,
    Minor,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub struct Key {
    pub tonic: Tpc,
    pub mode: Mode,
}

/// A scale degree (1 to 7) with an alteration relative to the key's natural scale.
/// In minor keys the reference scale is natural minor, so the leading tone is "#7".
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub struct Degree {
    pub number: u8,
    pub alteration: i8,
}

/// Fifths above the tonic for degrees 1 to 7.
const MAJOR_FIFTHS: [i32; 7] = [0, 2, 4, -1, 1, 3, 5];
const NATURAL_MINOR_FIFTHS: [i32; 7] = [0, 2, -3, -1, 1, -4, -2];

impl Key {
    pub fn new(tonic: Tpc, mode: Mode) -> Key {
        Key { tonic, mode }
    }

    /// Parses music21-style tonic names plus a mode, e.g. ("E-", "minor").
    pub fn parse(tonic: &str, mode: &str) -> Option<Key> {
        let mode = match mode {
            "major" => Mode::Major,
            "minor" => Mode::Minor,
            _ => return None,
        };
        Some(Key::new(tpc_from_name(tonic)?, mode))
    }

    /// Returns names such as "Eb" for major keys and "c#" for minor keys.
    pub fn name(self) -> String {
        let name = tpc_name(self.tonic);
        match self.mode {
            Mode::Major => name,
            Mode::Minor => name.to_lowercase(),
        }
    }

    fn scale_fifths(self) -> &'static [i32; 7] {
        match self.mode {
            Mode::Major => &MAJOR_FIFTHS,
            Mode::Minor => &NATURAL_MINOR_FIFTHS,
        }
    }

    /// Returns the scale degree of a spelled pitch in this key.
    pub fn degree_of(self, tpc: Tpc) -> Degree {
        let interval = tpc - self.tonic;
        let generic = letter_index(interval) as usize;
        let reference = self.scale_fifths()[generic];
        Degree { number: generic as u8 + 1, alteration: ((interval - reference) / 7) as i8 }
    }

    /// Returns the spelled pitch for a scale degree in this key.
    pub fn tpc_of(self, degree: Degree) -> Tpc {
        self.tonic + self.scale_fifths()[degree.number as usize - 1] + 7 * degree.alteration as i32
    }

    /// Reports whether a spelled pitch belongs to the natural scale of the key
    /// (plus raised 6 and 7 in minor).
    pub fn is_scale_tone(self, tpc: Tpc) -> bool {
        let degree = self.degree_of(tpc);
        degree.alteration == 0
            || (self.mode == Mode::Minor && degree.alteration == 1 && matches!(degree.number, 6 | 7))
    }
}

impl Degree {
    pub fn label(self) -> String {
        let prefix = if self.alteration > 0 { "#" } else { "b" };
        format!("{}{}", prefix.repeat(self.alteration.unsigned_abs() as usize), self.number)
    }
}

static ALL_KEYS: std::sync::LazyLock<Vec<Key>> = std::sync::LazyLock::new(|| {
    let majors = (-7..=7).map(|tonic| Key::new(tonic, Mode::Major));
    let minors = (-4..=10).map(|tonic| Key::new(tonic, Mode::Minor));
    majors.chain(minors).collect()
});

/// Returns the keys considered by the engine: major tonics Cb to C# and minor tonics ab to a#.
pub fn all_keys() -> &'static [Key] {
    &ALL_KEYS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str, mode: &str) -> Key {
        Key::parse(name, mode).unwrap()
    }

    #[test]
    fn leading_tone_in_minor_is_raised_seventh() {
        let degree = key("C", "minor").degree_of(tpc_from_name("B").unwrap());
        assert_eq!(degree.label(), "#7");
    }

    #[test]
    fn neapolitan_root_is_flat_two() {
        let degree = key("E-", "major").degree_of(tpc_from_name("F-").unwrap());
        assert_eq!(degree.label(), "b2");
    }

    #[test]
    fn degree_round_trips_to_spelling() {
        let k = key("F#", "minor");
        let tpc = tpc_from_name("E#").unwrap();
        assert_eq!(k.tpc_of(k.degree_of(tpc)), tpc);
    }

    #[test]
    fn minor_key_names_are_lowercase() {
        assert_eq!(key("C#", "minor").name(), "c#");
    }

    #[test]
    fn engine_considers_thirty_keys() {
        assert_eq!(all_keys().len(), 30);
    }
}
