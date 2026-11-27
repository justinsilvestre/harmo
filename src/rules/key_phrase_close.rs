//! Rules about the chord that ends a phrase as evidence for a key.

use super::sources::{KOSTKA_PAYNE_ALMEN_2023, SCHULTZ_ET_AL_2018};
use crate::engine::rule::{Citation, CloseContext, Hook, Layer, Rule};
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "key.phrase_close",
    layer: Layer::Key,
    summary: "Prefer keys in which the chord that ends a phrase is the tonic, or else the dominant. \
              A phrase ends at a fermata or at the end of the piece, and its last chord is a \
              cadence: an authentic cadence ends on the tonic and a half cadence on the dominant, \
              which is the less final of the two.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 145–148 (a cadence is a harmonic goal; the authentic cadence ends on I, the half cadence on V)",
        },
        Citation {
            source: SCHULTZ_ET_AL_2018,
            locator: "p. 69 (a phrase of a Bach chorale ends where the voices reach a fermata)",
        },
    ],
    default_weight: 1.0,
    hook: Hook::Close(phrase_close),
}];

/// The score of a phrase that ends on the dominant, relative to one that ends on the tonic.
const HALF_CADENCE: f32 = 0.5;

fn phrase_close(context: &CloseContext) -> f32 {
    let reading = context.segment.reading;
    if !ends_phrase(context) || reading.tonicized.is_some() {
        return 0.0;
    }
    let root = reading.chord.root;
    let is_major = matches!(reading.chord.quality, Quality::Major | Quality::DominantSeventh);
    if root == reading.key.tonic {
        1.0
    } else if root == reading.key.tonic + 1 && is_major {
        HALF_CADENCE
    } else {
        0.0
    }
}

/// Reports whether a segment ends a phrase: it reaches a fermata or the end of the piece.
pub fn ends_phrase(context: &CloseContext) -> bool {
    let Some(last) = context.slices.last() else { return false };
    context.next_slice.is_none_or(|next| next.phrase != last.phrase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Reading, Segment};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::Chord;
    use crate::theory::key::{Key, Mode};

    fn slice(phrase: usize) -> Slice {
        Slice { start: 0.0, duration: 1.0, notes: vec![], measure: 0, metric_level: 0, phrase }
    }

    /// Scores a chord with the given root, read in C major, followed by a slice in the given phrase.
    fn close(root: &str, next_phrase: Option<usize>) -> f32 {
        let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let segment = Segment {
            reading,
            start: 0,
            previous: NONE,
            duration: 1.0,
            members_heard: 0b111,
            bass: Bass::default(),
        };
        let next = next_phrase.map(slice);
        phrase_close(&CloseContext { segment: &segment, slices: &[slice(0)], next_slice: next.as_ref() })
    }

    #[test]
    fn tonic_before_a_fermata_ends_the_phrase() {
        assert_eq!(close("C", Some(1)), 1.0);
    }

    #[test]
    fn dominant_at_the_end_of_the_piece_is_a_half_cadence() {
        assert_eq!(close("G", None), HALF_CADENCE);
    }

    #[test]
    fn subdominant_does_not_end_a_phrase() {
        assert_eq!(close("F", Some(1)), 0.0);
    }

    #[test]
    fn tonic_within_a_phrase_earns_nothing() {
        assert_eq!(close("C", Some(0)), 0.0);
    }
}
