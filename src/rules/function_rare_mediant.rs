//! A rule about the mediant triad, the least used diatonic triad in major.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::model::Category;
use crate::engine::rule::{Citation, CloseContext, Hook, Layer, Rule};
use crate::theory::chord::Quality;
use crate::theory::key::Mode;

pub const RULES: &[Rule] = &[Rule {
    id: "function.rare_mediant",
    layer: Layer::Function,
    summary: "Avoid reading the mediant triad (iii) in major, the least used diatonic triad. A third or \
              sixth over the third degree in the bass is almost always I6 rather than iii.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 101 (iii is not often used in major; over 3̂ in the bass the chord is almost always I6)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 263 (iii is the least often used diatonic triad)",
        },
    ],
    default_weight: 1.0,
    hook: Hook::Close(rare_mediant),
}];

/// The mediant root, in fifths above the tonic.
const MEDIANT: i32 = 4;

fn rare_mediant(context: &CloseContext) -> f32 {
    let reading = context.segment.reading;
    let is_mediant = reading.key.mode == Mode::Major
        && reading.category == Category::Diatonic
        && reading.tonicized.is_none()
        && reading.chord.quality == Quality::Minor
        && reading.chord.root - reading.key.tonic == MEDIANT;
    if is_mediant { -1.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{NONE, Reading, Segment};
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    /// Scores a diatonic chord read in the given key.
    fn score(tonic: &str, mode: &str, root: &str, quality: Quality) -> f32 {
        let key = Key::parse(tonic, mode).unwrap();
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let segment = Segment {
            reading,
            start: 0,
            previous: NONE,
            duration: 1.0,
            members_heard: 0b111,
            bass: Bass::default(),
        };
        rare_mediant(&CloseContext { segment: &segment, slices: &[], next_slice: None })
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
    fn mediant_in_minor_is_free() {
        assert_eq!(score("A", "minor", "C", Quality::Major), 0.0);
    }
}
