//! Rules about the key signature as evidence for the key of each passage.

use super::sources::{BURSTEIN_STRAUS_2016, KOSTKA_PAYNE_ALMEN_2023, TEMPERLEY_2001};
use crate::engine::rule::{Citation, Hook, Layer, Rule, SliceContext};
use crate::pitch::pitch_class;
use crate::score::model::KeySignature;
use crate::theory::key::{Key, Mode};

pub const RULES: &[Rule] = &[Rule {
    id: "key.signature",
    layer: Layer::Key,
    summary: "Prefer the keys of the key signature in force: its major key and its relative \
              minor. Each slice in either key earns its duration. The signature names the main key \
              of a piece and is usually kept through its modulations, so other keys must be \
              supported by the music itself.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 6 (a key signature indicates the key); p. 11 (a major key and its relative minor share a signature); p. 306 (the signature of the main key is usually kept through modulations)",
        },
        Citation {
            source: TEMPERLEY_2001,
            locator: "p. 362, n. 6 (the key signature shows the main key of a piece)",
        },
        Citation {
            source: BURSTEIN_STRAUS_2016,
            locator: "p. 368 (pieces usually begin and end in the same key, the main key)",
        },
    ],
    default_weight: 0.1,
    hook: Hook::Slice(signature_key),
}];

fn signature_key(context: &SliceContext) -> f32 {
    let is_signature_key = context.slice.key_signature.is_some_and(|s| is_key_of(s, context.reading.key));
    if is_signature_key { context.slice.duration as f32 } else { 0.0 }
}

/// Reports whether a key is the signature's major key or its relative minor, in its own mode.
fn is_key_of(signature: KeySignature, key: Key) -> bool {
    let tonic = match key.mode {
        Mode::Major => signature.fifths,
        Mode::Minor => signature.fifths + 3,
    };
    pitch_class(tonic) == pitch_class(key.tonic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Category, Reading};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::{Chord, Quality};

    /// Scores a two-beat slice under the given signature, read in the given key.
    fn score(fifths: Option<i32>, tonic: &str, mode: Mode) -> f32 {
        let key_signature = fifths.map(|fifths| KeySignature { start: 0.0, fifths });
        let slice = Slice {
            start: 0.0,
            duration: 2.0,
            notes: vec![],
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature,
        };
        let key = Key::new(tpc_from_name(tonic).unwrap(), mode);
        let chord = Chord::new(tpc_from_name("G").unwrap(), Quality::Major);
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let slices = [slice];
        signature_key(&SliceContext {
            slice: &slices[0],
            next_slice: None,
            slices: &slices,
            index: 0,
            reading: &reading,
            is_opening: true,
        })
    }

    #[test]
    fn major_key_of_the_signature_earns_the_slice_duration() {
        assert_eq!(score(Some(-3), "Eb", Mode::Major), 2.0);
    }

    #[test]
    fn relative_minor_of_the_signature_earns_the_slice_duration() {
        assert_eq!(score(Some(-3), "C", Mode::Minor), 2.0);
    }

    #[test]
    fn parallel_minor_of_the_signature_earns_nothing() {
        assert_eq!(score(Some(-3), "Eb", Mode::Minor), 0.0);
    }

    #[test]
    fn enharmonic_spelling_of_the_tonic_counts() {
        assert_eq!(score(Some(6), "Gb", Mode::Major), 2.0);
    }

    #[test]
    fn other_keys_earn_nothing() {
        assert_eq!(score(Some(-3), "Bb", Mode::Major), 0.0);
    }

    #[test]
    fn a_score_without_a_signature_earns_nothing() {
        assert_eq!(score(None, "C", Mode::Major), 0.0);
    }
}
