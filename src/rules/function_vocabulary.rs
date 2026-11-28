//! The Roman numeral vocabulary: the ways a chord may function in a key.
//!
//! A chord with no entry for a key cannot be read in that key at all, so this table acts as a
//! hard constraint on readings.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::key_hooks::KeyHook;
use crate::engine::model::{Category, Function};
use crate::engine::rule::{Citation, Hook, Layer, Rule};
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::{Key, Mode};

pub const RULES: &[Rule] = &[Rule {
    id: "function.vocabulary",
    layer: Layer::Function,
    summary: "Read chords as diatonic triads and seventh chords, mixture chords, the Neapolitan, \
              augmented sixths, or applied dominants and leading-tone chords of diatonic triads. In minor, \
              the dominant seventh on the subtonic is read as V7/III, not as VII7.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 54–62 (diatonic chords in major and minor)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 256–260, 273–276 (secondary dominants and leading-tone chords)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 257, 263 (VII7 in minor usually functions as a dominant of III and is labeled V7/III)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 357–364 (mode mixture and the Neapolitan)",
        },
        Citation { source: KOSTKA_PAYNE_ALMEN_2023, locator: "pp. 382–389 (augmented sixth chords)" },
    ],
    default_weight: 1.0,
    params: &[],
    hook: Hook::Key(KeyHook::Interpret(interpret)),
}];

use Category::{Chromatic, Diatonic, Mixture};
use Quality::*;

/// (root in fifths above the tonic, quality, category).
type Entry = (i32, Quality, Category);

const MAJOR: &[Entry] = &[
    (0, Major, Diatonic),
    (2, Minor, Diatonic),
    (4, Minor, Diatonic),
    (-1, Major, Diatonic),
    (1, Major, Diatonic),
    (3, Minor, Diatonic),
    (5, Diminished, Diatonic),
    (0, MajorSeventh, Diatonic),
    (2, MinorSeventh, Diatonic),
    (4, MinorSeventh, Diatonic),
    (-1, MajorSeventh, Diatonic),
    (1, DominantSeventh, Diatonic),
    (3, MinorSeventh, Diatonic),
    (5, HalfDiminishedSeventh, Diatonic),
    (0, Minor, Mixture),
    (2, Diminished, Mixture),
    (2, HalfDiminishedSeventh, Mixture),
    (-3, Major, Mixture),
    (-3, MajorSeventh, Mixture),
    (-1, Minor, Mixture),
    (-1, MinorSeventh, Mixture),
    (1, Minor, Mixture),
    (-4, Major, Mixture),
    (-4, MajorSeventh, Mixture),
    (-2, Major, Mixture),
    (5, DiminishedSeventh, Mixture),
    (-5, Major, Chromatic),
    (1, Augmented, Chromatic),
    (6, ItalianSixth, Chromatic),
    (6, GermanSixth, Chromatic),
    (2, FrenchSixth, Chromatic),
];

/// Has no VII7, so that the dominant seventh on the subtonic is read only as V7/III.
const MINOR: &[Entry] = &[
    (0, Minor, Diatonic),
    (2, Diminished, Diatonic),
    (-3, Major, Diatonic),
    (-1, Minor, Diatonic),
    (1, Minor, Diatonic),
    (1, Major, Diatonic),
    (-4, Major, Diatonic),
    (-2, Major, Diatonic),
    (5, Diminished, Diatonic),
    (-3, Augmented, Diatonic),
    (-1, Major, Diatonic),
    (2, Minor, Diatonic),
    (0, MinorSeventh, Diatonic),
    (0, MinorMajorSeventh, Diatonic),
    (2, HalfDiminishedSeventh, Diatonic),
    (-3, MajorSeventh, Diatonic),
    (-1, MinorSeventh, Diatonic),
    (1, MinorSeventh, Diatonic),
    (1, DominantSeventh, Diatonic),
    (-4, MajorSeventh, Diatonic),
    (5, DiminishedSeventh, Diatonic),
    (5, HalfDiminishedSeventh, Diatonic),
    (0, Major, Mixture),
    (-5, Major, Chromatic),
    (6, ItalianSixth, Chromatic),
    (6, GermanSixth, Chromatic),
    (2, FrenchSixth, Chromatic),
];

/// Degrees that may be tonicized, as (tonic in fifths above the key's tonic, mode of the target).
const MAJOR_TARGETS: &[(i32, Mode)] =
    &[(2, Mode::Minor), (4, Mode::Minor), (-1, Mode::Major), (1, Mode::Major), (3, Mode::Minor)];
const MINOR_TARGETS: &[(i32, Mode)] =
    &[(-3, Mode::Major), (-1, Mode::Minor), (1, Mode::Major), (-4, Mode::Major), (-2, Mode::Major)];

/// Applied functions within the tonicized key: V, V7, viio, viio7 and viiø7.
const APPLIED: &[(i32, Quality)] =
    &[(1, Major), (1, DominantSeventh), (5, Diminished), (5, DiminishedSeventh), (5, HalfDiminishedSeventh)];

fn interpret(key: Key, chord: Chord) -> Vec<Function> {
    let (entries, targets) = match key.mode {
        Mode::Major => (MAJOR, MAJOR_TARGETS),
        Mode::Minor => (MINOR, MINOR_TARGETS),
    };
    let root = chord.root - key.tonic;
    let local = entries
        .iter()
        .filter(|(r, q, _)| *r == root && *q == chord.quality)
        .map(|&(_, _, category)| Function { tonicized: None, category });
    let mut readings: Vec<Function> = local.collect();
    if readings.iter().any(|function| function.category == Diatonic) {
        return readings;
    }
    for &(offset, mode) in targets {
        let is_applied = APPLIED.iter().any(|&(r, q)| r == root - offset && q == chord.quality);
        if is_applied {
            let tonicized = Some(Key::new(key.tonic + offset, mode));
            readings.push(Function { tonicized, category: Category::Applied });
        }
    }
    readings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;

    fn chord(root: &str, quality: Quality) -> Chord {
        Chord::new(tpc_from_name(root).unwrap(), quality)
    }

    #[test]
    fn secondary_dominant_tonicizes_the_dominant() {
        let c = Key::parse("C", "major").unwrap();
        let g = Key::parse("G", "major").unwrap();
        assert_eq!(
            interpret(c, chord("D", DominantSeventh)),
            vec![Function { tonicized: Some(g), category: Category::Applied }]
        );
    }

    #[test]
    fn diatonic_chords_are_not_also_read_as_applied() {
        let c = Key::parse("C", "major").unwrap();
        assert_eq!(interpret(c, chord("C", Major)), vec![Function { tonicized: None, category: Diatonic }]);
    }

    #[test]
    fn chords_outside_the_vocabulary_have_no_reading() {
        let c = Key::parse("C", "major").unwrap();
        assert!(interpret(c, chord("F#", Major)).is_empty());
    }

    #[test]
    fn subtonic_dominant_seventh_in_minor_tonicizes_the_mediant() {
        let c = Key::parse("C", "minor").unwrap();
        let e_flat = Key::parse("Eb", "major").unwrap();
        assert_eq!(
            interpret(c, chord("Bb", DominantSeventh)),
            vec![Function { tonicized: Some(e_flat), category: Category::Applied }]
        );
    }

    #[test]
    fn subtonic_triad_in_minor_stays_diatonic() {
        let c = Key::parse("C", "minor").unwrap();
        assert_eq!(interpret(c, chord("Bb", Major)), vec![Function { tonicized: None, category: Diatonic }]);
    }

    #[test]
    fn german_sixth_is_chromatic_in_minor() {
        let c = Key::parse("C", "minor").unwrap();
        assert_eq!(
            interpret(c, chord("F#", GermanSixth)),
            vec![Function { tonicized: None, category: Chromatic }]
        );
    }
}
