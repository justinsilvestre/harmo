//! Rules about the major tonic that ends a phrase in minor.

use super::key_phrase_close::ends_phrase;
use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::key_hooks::{KeyCloseContext, KeyHook};
use crate::engine::model::Reading;
use crate::engine::rule::{Citation, Hook, Layer, Rule};
use crate::theory::chord::Quality;
use crate::theory::key::Mode;

pub const RULES: &[Rule] = &[Rule {
    id: "function.picardy_third",
    layer: Layer::Function,
    summary: "Hear a major tonic triad that ends a phrase in minor as a Picardy third rather than \
              as a change of mode. A phrase ends at a fermata or at the end of the piece. The rule \
              repays the cost of the borrowed chord, so that the cadence stays in the minor key.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 208 (in the Baroque, a phrase or composition in minor often ends on a major tonic triad)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 357 (the Picardy third ended most compositions in minor from the early 1500s until around 1750)",
        },
    ],
    default_weight: 1.0,
    params: &[],
    hook: Hook::Key(KeyHook::Close(picardy_third)),
}];

fn picardy_third(context: &KeyCloseContext, _: &[f64]) -> f32 {
    if ends_phrase(context) && is_major_tonic_in_minor(&context.segment.reading()) { 1.0 } else { 0.0 }
}

fn is_major_tonic_in_minor(reading: &Reading) -> bool {
    let is_tonic = reading.chord.root == reading.key.tonic && reading.function.tonicized.is_none();
    reading.key.mode == Mode::Minor && is_tonic && reading.chord.quality == Quality::Major
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::engine::model::{Category, Function};
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    fn slice(phrase: usize) -> Slice {
        Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![],
            measure: 0,
            metric_level: 0,
            phrase,
            key_signature: None,
        }
    }

    /// Scores a G chord of the given quality, read in G of the given mode, followed by a slice in
    /// the given phrase.
    fn close(mode: Mode, quality: Quality, next_phrase: Option<usize>) -> f32 {
        let g = tpc_from_name("G").unwrap();
        let reading = Reading {
            key: Key::new(g, mode),
            chord: Chord::new(g, quality),
            function: Function { tonicized: None, category: Category::Mixture },
        };
        let segment = key_segment(reading);
        let next = next_phrase.map(slice);
        picardy_third(
            &KeyCloseContext { segment: &segment, slices: &[slice(0)], next_slice: next.as_ref() },
            &[],
        )
    }

    #[test]
    fn major_tonic_ending_a_piece_in_minor_is_a_picardy_third() {
        assert_eq!(close(Mode::Minor, Quality::Major, None), 1.0);
    }

    #[test]
    fn major_tonic_before_a_fermata_in_minor_is_a_picardy_third() {
        assert_eq!(close(Mode::Minor, Quality::Major, Some(1)), 1.0);
    }

    #[test]
    fn major_tonic_within_a_phrase_earns_nothing() {
        assert_eq!(close(Mode::Minor, Quality::Major, Some(0)), 0.0);
    }

    #[test]
    fn minor_tonic_earns_nothing() {
        assert_eq!(close(Mode::Minor, Quality::Minor, None), 0.0);
    }

    #[test]
    fn major_tonic_in_major_earns_nothing() {
        assert_eq!(close(Mode::Major, Quality::Major, None), 0.0);
    }
}
