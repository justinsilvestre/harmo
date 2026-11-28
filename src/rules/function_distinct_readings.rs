//! Rules that tell the search which hypotheses hold the same interpretation.

use super::sources::TEMPERLEY_2001;
use crate::engine::key_hooks::KeyHook;
use crate::engine::model::{Category, KeySegment};
use crate::engine::rule::{Citation, Hook, Layer, Rule};
use crate::theory::chord::Quality;
use crate::theory::key::{Key, Mode};

pub const RULES: &[Rule] = &[Rule {
    id: "function.distinct_readings",
    layer: Layer::Function,
    summary: "Count two hypotheses as the same interpretation when they give their recent segments \
              the same readings (chord, key, tonicized key and category), whatever the segments' \
              boundaries. Of such hypotheses the search keeps only the best, leaving room for \
              interpretations that differ in their harmony. A listener's interpretation is the \
              sequence of chords and keys heard, and two analyses that differ only in where a \
              chord change falls by a slice or two are variants of one hearing.",
    citations: &[Citation {
        source: TEMPERLEY_2001,
        locator: "pp. 15-19 (the search keeps the best analysis for each state, where a state is the label of the current segment)",
    }],
    default_weight: 1.0,
    params: &[],
    hook: Hook::Key(KeyHook::Distinguish(reading_identity)),
}];

/// Packs the segment's whole reading into one value, so that two segments get the same value
/// exactly when their readings are equal.
fn reading_identity(segment: &KeySegment, _: &[f64]) -> u64 {
    let (chord, function) = (segment.heard.chord, segment.function);
    let tonicized = function.tonicized.map_or(0, |key| 1 << 17 | pack_key(key));
    let fields = [
        (pack_key(function.key), 17),
        (pack_tpc(chord.root), 16),
        (quality_index(chord.quality), 4),
        (tonicized, 18),
        (category_index(function.category), 2),
    ];
    fields.iter().fold(0, |packed, &(value, bits)| packed << bits | value)
}

fn pack_key(key: Key) -> u64 {
    pack_tpc(key.tonic) << 1 | (key.mode == Mode::Minor) as u64
}

/// Spellings lie far inside the 16-bit range, so the cast loses nothing.
fn pack_tpc(tpc: i32) -> u64 {
    u64::from(tpc as i16 as u16)
}

fn quality_index(quality: Quality) -> u64 {
    quality as u64
}

fn category_index(category: Category) -> u64 {
    category as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{ChordSegment, Function, Reading};
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Chord;

    fn segment(key: &str, root: &str, tonicized: Option<&str>, start: u32) -> KeySegment {
        let major = |name: &str| Key::new(tpc_from_name(name).unwrap(), Mode::Major);
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        let function =
            Function { key: major(key), tonicized: tonicized.map(major), category: Category::Diatonic };
        let reading = Reading { chord, function };
        let segment = key_segment(reading);
        KeySegment { heard: ChordSegment { start, ..segment.heard }, ..segment }
    }

    #[test]
    fn segments_with_equal_readings_and_other_boundaries_share_a_value() {
        assert_eq!(
            reading_identity(&segment("C", "G", None, 0), &[]),
            reading_identity(&segment("C", "G", None, 3), &[])
        );
    }

    #[test]
    fn segments_in_different_keys_differ() {
        assert_ne!(
            reading_identity(&segment("C", "G", None, 0), &[]),
            reading_identity(&segment("G", "G", None, 0), &[])
        );
    }

    #[test]
    fn segments_that_tonicize_different_keys_differ() {
        let applied = segment("C", "D", Some("G"), 0);
        assert_ne!(reading_identity(&applied, &[]), reading_identity(&segment("C", "D", None, 0), &[]));
    }

    #[test]
    fn segments_with_flat_and_sharp_roots_differ() {
        assert_ne!(
            reading_identity(&segment("C", "Bb", None, 0), &[]),
            reading_identity(&segment("C", "A#", None, 0), &[])
        );
    }
}
