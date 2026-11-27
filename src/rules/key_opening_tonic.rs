//! Rules about the chord that opens a piece as evidence for a key.

use super::sources::{BURSTEIN_STRAUS_2016, KOSTKA_PAYNE_ALMEN_2023};
use crate::engine::model::Reading;
use crate::engine::rule::{Citation, Hook, Layer, Rule, TransitionContext};

pub const RULES: &[Rule] = &[Rule {
    id: "key.opening_tonic",
    layer: Layer::Key,
    summary: "Prefer keys in which the first chord of the piece is the tonic. A piece begins in its \
              main key, and its first chord is usually that key's tonic.",
    citations: &[
        Citation {
            source: BURSTEIN_STRAUS_2016,
            locator: "p. 368 (pieces usually begin and end in the same key, the main key)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 180 (the first chord should be a tonic or a dominant triad)",
        },
    ],
    default_weight: 1.0,
    hook: Hook::Transition(opening_tonic),
}];

fn opening_tonic(context: &TransitionContext) -> f32 {
    if context.previous.is_none() && is_tonic(context.next) { 1.0 } else { 0.0 }
}

fn is_tonic(reading: &Reading) -> bool {
    reading.tonicized.is_none() && reading.chord.root == reading.key.tonic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Segment};
    use crate::engine::rule::History;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    fn reading(root: &str) -> Reading {
        let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        Reading { key, chord, tonicized: None, category: Category::Diatonic }
    }

    /// Scores opening a C major segment read in C major, after the given segment if any.
    fn transition(previous: Option<&Segment>, root: &str) -> f32 {
        let slice =
            Slice { start: 0.0, duration: 1.0, notes: vec![], measure: 0, metric_level: 0, phrase: 0 };
        let history = History { arena: &[], last: NONE };
        opening_tonic(&TransitionContext { previous, next: &reading(root), slice: &slice, history })
    }

    #[test]
    fn piece_opening_on_the_tonic_earns_credit() {
        assert_eq!(transition(None, "C"), 1.0);
    }

    #[test]
    fn piece_opening_on_another_degree_earns_nothing() {
        assert_eq!(transition(None, "G"), 0.0);
    }

    #[test]
    fn tonic_after_the_opening_earns_nothing() {
        let previous = Segment {
            reading: reading("G"),
            start: 0,
            previous: NONE,
            duration: 1.0,
            members_heard: 0b111,
            bass: Bass::default(),
        };
        assert_eq!(transition(Some(&previous), "C"), 0.0);
    }
}
