//! Rules about the key signature as evidence for the key of each passage.

use super::sources::{
    BURSTEIN_STRAUS_2016, CAPLIN_2024, KOSTKA_PAYNE_ALMEN_2023, TEMPERLEY_2001, TEMPERLEY_2007,
};
use crate::engine::key_contexts::KeySliceContext;
use crate::engine::key_hooks::KeyHook;
use crate::engine::rule::{Citation, Hook, Rule, Topic};
use crate::pitch::pitch_class;
use crate::score::model::KeySignature;
use crate::score::slicing::Slice;
use crate::theory::key::{Key, Mode};

pub const RULES: &[Rule] = &[Rule {
    id: "key.signature",
    topic: Topic::Key,
    summary: "Prefer the keys of the key signature in force: its major key and its relative \
              minor. Each slice in either key earns its duration. The signature names the main key \
              of a piece and is usually kept through its modulations, so other keys must be \
              supported by the music itself. A piece whose lowest opening note is the second \
              degree of the signature's major scale, such as D under a signature without sharps or \
              flats, is read as having a Dorian signature, one flat short of its minor key, which \
              older music often uses. The signature then names that minor key alone.",
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
        Citation {
            source: CAPLIN_2024,
            locator: "p. 348, n. 98 (a composer's conventional use of a \"Dorian\" key signature, with one flat fewer than the home key)",
        },
        Citation {
            source: TEMPERLEY_2007,
            locator: "p. 201 (in modal styles a piece on the C major scale may have its tonal center on D, the Dorian mode)",
        },
    ],
    default_weight: 0.1,
    params: &[],
    hook: Hook::Key(KeyHook::Slice(signature_key)),
}];

/// The Dorian tonic's distance above the tonic of the signature's major key, in fifths.
const DORIAN_TONIC: i32 = 2;

fn signature_key(context: &KeySliceContext, _: &[f64]) -> f32 {
    let Some(signature) = context.slice.key_signature else { return 0.0 };
    let key = context.reading.function.key;
    let is_named = if opens_on_dorian_tonic(context.slices, signature) {
        key.mode == Mode::Minor && pitch_class(key.tonic) == pitch_class(signature.fifths + DORIAN_TONIC)
    } else {
        is_key_of(signature, key)
    };
    if is_named { context.slice.duration as f32 } else { 0.0 }
}

/// Reports whether a key is the signature's major key or its relative minor, in its own mode.
fn is_key_of(signature: KeySignature, key: Key) -> bool {
    let tonic = match key.mode {
        Mode::Major => signature.fifths,
        Mode::Minor => signature.fifths + 3,
    };
    pitch_class(tonic) == pitch_class(key.tonic)
}

/// Reports whether the piece opens under the given signature
/// and the lowest note of its first sounding slice is the signature's Dorian tonic.
fn opens_on_dorian_tonic(slices: &[Slice], signature: KeySignature) -> bool {
    let Some(first) = slices.iter().find(|s| !s.notes.is_empty()) else { return false };
    let is_opening_signature = first.key_signature.is_some_and(|s| s.fifths == signature.fifths);
    let lowest = first.notes.iter().min_by_key(|note| note.midi);
    is_opening_signature
        && lowest.is_some_and(|note| pitch_class(note.tpc) == pitch_class(signature.fifths + DORIAN_TONIC))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Function, Reading};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::{Chord, Quality};

    /// Scores a two-beat slice under the given signature, read in the given key.
    fn score(fifths: Option<i32>, tonic: &str, mode: Mode) -> f32 {
        score_after(fifths, None, fifths, (tonic, mode))
    }

    /// Scores a two-beat slice under the signature `fifths`, read in the given key,
    /// after a piece opens over `bass`, if any, under the signature `opening`.
    fn score_after(opening: Option<i32>, bass: Option<&str>, fifths: Option<i32>, key: (&str, Mode)) -> f32 {
        let note = |name: &str| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 50,
            attacked: true,
            onset_level: 4,
        };
        let slice = |start: f64, notes: Vec<SliceNote>, fifths: Option<i32>| Slice {
            start,
            duration: 2.0,
            notes,
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: fifths.map(|fifths| KeySignature { start: 0.0, fifths }),
        };
        let slices = [slice(0.0, bass.map(note).into_iter().collect(), opening), slice(2.0, vec![], fifths)];
        let key = Key::new(tpc_from_name(key.0).unwrap(), key.1);
        let chord = Chord::new(tpc_from_name("G").unwrap(), Quality::Major);
        let reading = Reading { chord, function: Function::diatonic(key) };
        signature_key(
            &KeySliceContext {
                slice: &slices[1],
                next_slice: None,
                slices: &slices,
                index: 1,
                reading: &reading,
                is_opening: false,
            },
            &[],
        )
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
    fn dorian_minor_earns_when_the_piece_opens_on_its_tonic() {
        assert_eq!(score_after(Some(-1), Some("G"), Some(-1), ("G", Mode::Minor)), 2.0);
    }

    #[test]
    fn major_key_earns_nothing_when_the_piece_opens_on_the_dorian_tonic() {
        assert_eq!(score_after(Some(-1), Some("G"), Some(-1), ("F", Mode::Major)), 0.0);
    }

    #[test]
    fn dorian_minor_earns_nothing_when_the_piece_opens_elsewhere() {
        assert_eq!(score_after(Some(-1), Some("F"), Some(-1), ("G", Mode::Minor)), 0.0);
    }

    #[test]
    fn a_later_signature_is_not_read_as_dorian() {
        assert_eq!(score_after(Some(1), Some("E"), Some(2), ("E", Mode::Minor)), 0.0);
    }

    #[test]
    fn a_score_without_a_signature_earns_nothing() {
        assert_eq!(score(None, "C", Mode::Major), 0.0);
    }
}
