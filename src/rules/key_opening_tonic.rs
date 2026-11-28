//! Rules about the chord that opens a piece as evidence for a key.

use super::sources::{BURSTEIN_STRAUS_2016, KOSTKA_PAYNE_ALMEN_2023};
use crate::engine::key_hooks::{KeyHook, KeyTransitionContext};
use crate::engine::model::Reading;
use crate::engine::rule::{Citation, Hook, Layer, Rule};

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
    default_weight: 1.25,
    params: &[],
    hook: Hook::Key(KeyHook::Transition(opening_tonic)),
}];

fn opening_tonic(context: &KeyTransitionContext, _: &[f64]) -> f32 {
    if context.previous.is_none() && is_tonic(context.next) { 1.0 } else { 0.0 }
}

fn is_tonic(reading: &Reading) -> bool {
    reading.function.tonicized.is_none() && reading.chord.root == reading.key.tonic
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::engine::key_hooks::History;
    use crate::engine::model::{Function, KeySegment};
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    fn reading(root: &str) -> Reading {
        let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        Reading { key, chord, function: Function::DIATONIC }
    }

    /// Scores opening a C major segment read in C major, after the given segment if any.
    fn transition(previous: Option<&KeySegment>, root: &str) -> f32 {
        let slice = Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![],
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        };
        let history = History::empty(&[]);
        opening_tonic(&KeyTransitionContext { previous, next: &reading(root), slice: &slice, history }, &[])
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
        let previous = key_segment(reading("G"));
        assert_eq!(transition(Some(&previous), "C"), 0.0);
    }
}
