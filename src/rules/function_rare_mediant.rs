//! A rule about the rare forms of the mediant triad: iii in major and III+ in minor.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::key_contexts::KeyCloseContext;
use crate::engine::key_hooks::KeyHook;
use crate::engine::model::Category;
use crate::engine::rule::{Citation, Hook, Rule, Topic};
use crate::theory::chord::Quality;
use crate::theory::key::Mode;

pub const RULES: &[Rule] = &[Rule {
    id: "function.rare_mediant",
    topic: Topic::Function,
    summary: "Avoid reading the mediant triad (iii) in major, the least used diatonic triad. A third or \
              sixth over the third degree in the bass is almost always I6 rather than iii. In minor, \
              the mediant is the major triad III; the augmented III+ is rare.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 101 (iii is not often used in major; over 3̂ in the bass the chord is almost always I6)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 263 (iii is the least often used diatonic triad)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 58 (the common diatonic triads in minor include III but no augmented triad)",
        },
    ],
    default_weight: 1.0,
    params: &[],
    hook: Hook::Key(KeyHook::Close(rare_mediant)),
}];

/// The mediant root in major, in fifths above the tonic.
const MEDIANT: i32 = 4;
/// The mediant root in minor, in fifths above the tonic.
const MINOR_MEDIANT: i32 = -3;

fn rare_mediant(context: &KeyCloseContext, _: &[f64]) -> f32 {
    let (chord, function) = (context.segment.heard.chord, context.segment.function);
    if function.category != Category::Diatonic || function.tonicized.is_some() {
        return 0.0;
    }
    let degree = chord.root - function.key.tonic;
    let is_rare = match function.key.mode {
        Mode::Major => chord.quality == Quality::Minor && degree == MEDIANT,
        Mode::Minor => chord.quality == Quality::Augmented && degree == MINOR_MEDIANT,
    };
    if is_rare { -1.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::engine::model::{Function, Reading};
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    /// Scores a diatonic chord read in the given key.
    fn score(tonic: &str, mode: &str, root: &str, quality: Quality) -> f32 {
        let key = Key::parse(tonic, mode).unwrap();
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        let reading = Reading { chord, function: Function::diatonic(key) };
        let segment = key_segment(reading);
        rare_mediant(&KeyCloseContext { segment: &segment, slices: &[], next_slice: None }, &[])
    }

    #[test]
    fn mediant_in_major_costs() {
        assert_eq!(score("C", "major", "E", Quality::Minor), -1.0);
    }

    #[test]
    fn tonic_in_major_is_free() {
        assert_eq!(score("C", "major", "C", Quality::Major), 0.0);
    }

    #[test]
    fn mediant_seventh_is_left_to_rare_seventh() {
        assert_eq!(score("C", "major", "E", Quality::MinorSeventh), 0.0);
    }

    #[test]
    fn augmented_mediant_in_minor_costs() {
        assert_eq!(score("A", "minor", "C", Quality::Augmented), -1.0);
    }

    #[test]
    fn mediant_in_minor_is_free() {
        assert_eq!(score("A", "minor", "C", Quality::Major), 0.0);
    }
}
