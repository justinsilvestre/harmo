//! Roman numeral labels in the feature layout used by AugmentedNet.

use super::chord::{Chord, Quality};
use super::key::{Degree, Key, Mode};
use crate::pitch::Tpc;
use serde::Serialize;

/// A Roman numeral reading broken into separately evaluated features.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
pub struct Label {
    pub key: Key,
    pub tonicized: Option<Key>,
    /// Degree of the root in the tonicized key, or in the local key when nothing is tonicized.
    pub degree: Degree,
    /// Degree of the tonicized key's tonic in the local key (the "/V" of "V7/V").
    pub secondary: Option<Degree>,
    pub quality: Quality,
    pub inversion: u8,
    pub root: Tpc,
}

const NUMERALS: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];

impl Label {
    pub fn new(key: Key, tonicized: Option<Key>, chord: Chord, bass: Tpc) -> Label {
        Label {
            key,
            tonicized,
            degree: tonicized.unwrap_or(key).degree_of(chord.root),
            secondary: tonicized.map(|t| key.degree_of(t.tonic)),
            quality: chord.quality,
            inversion: chord.inversion(bass),
            root: chord.root,
        }
    }

    /// Returns the chord the label names, whatever its key.
    pub fn chord(&self) -> Chord {
        Chord::new(self.root, self.quality)
    }

    /// Returns the chord member in the bass, spelled as the chord spells it.
    pub fn bass(&self) -> Tpc {
        self.chord().members().nth(self.inversion as usize).unwrap_or(self.root)
    }

    /// Returns the key the numeral is read in: the tonicized key of an applied chord, or else the key.
    pub fn reference_key(&self) -> Key {
        self.tonicized.unwrap_or(self.key)
    }

    /// Formats the label as a Roman numeral figure such as "V65/V" or "Ger65".
    pub fn figure(&self) -> String {
        let reference = self.reference_key();
        let degree = displayed_degree(reference.mode, self.degree);
        let main =
            format!("{}{}", numeral(degree, self.quality), inversion_figure(self.quality, self.inversion));
        match self.secondary {
            Some(secondary) => format!("{main}/{}", secondary_numeral(reference.mode, secondary)),
            None => main,
        }
    }
}

/// Hides the sharp on the raised sixth and seventh degrees of minor keys,
/// following the usual convention that "viio" in minor implies the leading tone.
fn displayed_degree(mode: Mode, degree: Degree) -> Degree {
    let is_raised_in_minor = mode == Mode::Minor && degree.alteration == 1 && degree.number >= 6;
    if is_raised_in_minor { Degree { alteration: 0, ..degree } } else { degree }
}

fn numeral(degree: Degree, quality: Quality) -> String {
    let accidental =
        if degree.alteration > 0 { "#" } else { "b" }.repeat(degree.alteration.unsigned_abs() as usize);
    let upper = NUMERALS[degree.number as usize - 1];
    let lower = upper.to_lowercase();
    let body = match quality {
        Quality::Major | Quality::DominantSeventh | Quality::MajorSeventh => upper.to_string(),
        Quality::Augmented => format!("{upper}+"),
        Quality::Minor | Quality::MinorSeventh | Quality::MinorMajorSeventh => lower,
        Quality::Diminished | Quality::DiminishedSeventh => format!("{lower}o"),
        Quality::HalfDiminishedSeventh => format!("{lower}ø"),
        Quality::ItalianSixth => return "It".into(),
        Quality::FrenchSixth => return "Fr".into(),
        Quality::GermanSixth => return "Ger".into(),
        Quality::Other => format!("{upper}?"),
    };
    format!("{accidental}{body}")
}

fn inversion_figure(quality: Quality, inversion: u8) -> &'static str {
    let sevenths = ["7", "65", "43", "2"];
    let triads = ["", "6", "64", ""];
    match quality {
        Quality::ItalianSixth => "6",
        _ if quality.is_seventh() => sevenths[inversion as usize % 4],
        _ => triads[inversion as usize % 4],
    }
}

/// Spells the target of a tonicization in the case of its mode: upper case for a major target,
/// lower case for a minor one.
fn secondary_numeral(target_mode: Mode, secondary: Degree) -> String {
    let quality = if target_mode == Mode::Major { Quality::Major } else { Quality::Minor };
    numeral(secondary, quality)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;

    fn tpc(name: &str) -> Tpc {
        tpc_from_name(name).unwrap()
    }

    #[test]
    fn applied_dominant_has_secondary_degree() {
        let c = Key::parse("C", "major").unwrap();
        let g = Key::parse("G", "major").unwrap();
        let label = Label::new(c, Some(g), Chord::new(tpc("D"), Quality::DominantSeventh), tpc("F#"));
        assert_eq!(label.figure(), "V65/V");
    }

    #[test]
    fn minor_leading_tone_chord_is_raised_seven() {
        let c = Key::parse("C", "minor").unwrap();
        let label = Label::new(c, None, Chord::new(tpc("B"), Quality::DiminishedSeventh), tpc("B"));
        assert_eq!(label.figure(), "viio7");
    }

    #[test]
    fn applied_chord_to_minor_target_uses_lower_case() {
        let c = Key::parse("C", "major").unwrap();
        let d = Key::parse("D", "minor").unwrap();
        let label = Label::new(c, Some(d), Chord::new(tpc("A"), Quality::Major), tpc("A"));
        assert_eq!(label.figure(), "V/ii");
    }

    #[test]
    fn applied_chord_to_major_target_in_minor_uses_upper_case() {
        let f_sharp = Key::parse("F#", "minor").unwrap();
        let b = Key::parse("B", "major").unwrap();
        let label = Label::new(f_sharp, Some(b), Chord::new(tpc("A#"), Quality::DiminishedSeventh), tpc("E"));
        assert_eq!(label.figure(), "viio43/IV");
    }
}
