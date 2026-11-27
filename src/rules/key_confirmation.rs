//! Rules about whether a key area was confirmed before the music leaves it.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::model::Segment;
use crate::engine::rule::{Citation, DepartureContext, Hook, Layer, Rule};
use crate::score::slicing::Slice;
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "key.unconfirmed_departure",
    layer: Layer::Key,
    summary: "Prefer not to leave a key area that no cadence has confirmed. Leaving a key area costs \
              unless it holds a phrase ending (a fermata), a root-position V–I with the tonic on a \
              strong beat, or a cadential six-four resolving to V. An area left unconfirmed is \
              better heard as a tonicization within the surrounding key. The piece's first key \
              area is exempt, since the opening establishes the main key.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 299 (a cadential six-four or V in the new key as evidence of a modulation)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 316 (short modulations that return at once to the first key are better called tonicizations)",
        },
    ],
    default_weight: 0.5,
    hook: Hook::Departure(unconfirmed_departure),
}];

/// The weakest metric level on which a cadential tonic may arrive (3 = mid-measure in duple meters).
const STRONG_BEAT: u8 = 3;

fn unconfirmed_departure(context: &DepartureContext) -> f32 {
    match find_confirmation(context) {
        Confirmation::Confirmed | Confirmation::FirstArea => 0.0,
        Confirmation::Unconfirmed => -1.0,
    }
}

enum Confirmation {
    Confirmed,
    /// Unconfirmed, but the area opens the piece.
    FirstArea,
    Unconfirmed,
}

/// Walks back through the key area being left, most recent segment first, and stops at the first
/// event that confirms it.
fn find_confirmation(context: &DepartureContext) -> Confirmation {
    let slices = context.history.slices;
    let key = context.previous.reading.key;
    let mut later: Option<&Segment> = None;
    for segment in context.history.segments() {
        if segment.reading.key != key {
            return Confirmation::Unconfirmed;
        }
        let is_confirming = |later: &Segment| {
            is_authentic_cadence(later, segment, slices) || is_cadential_six_four(later, segment)
        };
        if crosses_phrase_end(segment, slices, context.slice) || later.is_some_and(is_confirming) {
            return Confirmation::Confirmed;
        }
        later = Some(segment);
    }
    Confirmation::FirstArea
}

/// Reports whether a phrase ending falls between the segment's first slice and the given slice.
/// Phrase numbers never decrease, so an area crosses a phrase end exactly when one of its
/// segments does.
fn crosses_phrase_end(segment: &Segment, slices: &[Slice], slice: &Slice) -> bool {
    slices.get(segment.start as usize).is_some_and(|start| start.phrase != slice.phrase)
}

/// Reports whether a root-position dominant is followed by a root-position tonic on a strong beat.
fn is_authentic_cadence(later: &Segment, earlier: &Segment, slices: &[Slice]) -> bool {
    is_cadential_tonic(later, slices) && is_dominant(earlier) && is_in_root_position(earlier)
}

/// Reports whether a tonic six-four is followed by a dominant.
fn is_cadential_six_four(later: &Segment, earlier: &Segment) -> bool {
    is_dominant(later) && is_tonic_six_four(earlier)
}

fn is_cadential_tonic(segment: &Segment, slices: &[Slice]) -> bool {
    let is_strong = slices.get(segment.start as usize).is_some_and(|s| s.metric_level >= STRONG_BEAT);
    is_tonic(segment) && is_in_root_position(segment) && is_strong
}

fn is_tonic_six_four(segment: &Segment) -> bool {
    let key = segment.reading.key;
    is_tonic(segment) && segment.bass_tpc() == key.tonic + 1
}

/// Reports whether the segment is the tonic chord of its key, in any inversion.
pub fn is_tonic(segment: &Segment) -> bool {
    let reading = segment.reading;
    reading.tonicized.is_none() && reading.chord.root == reading.key.tonic
}

fn is_dominant(segment: &Segment) -> bool {
    let reading = segment.reading;
    reading.tonicized.is_none()
        && reading.chord.root == reading.key.tonic + 1
        && matches!(reading.chord.quality, Quality::Major | Quality::DominantSeventh)
}

fn is_in_root_position(segment: &Segment) -> bool {
    segment.bass_tpc() == segment.reading.chord.root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Node, Reading};
    use crate::engine::rule::History;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Chord;
    use crate::theory::key::{Key, Mode};

    fn tpc(name: &str) -> i32 {
        tpc_from_name(name).unwrap()
    }

    fn reading(key: &str, root: &str, quality: Quality) -> Reading {
        let key = Key::new(tpc(key), Mode::Major);
        Reading { key, chord: Chord::new(tpc(root), quality), tonicized: None, category: Category::Diatonic }
    }

    fn slice(metric_level: u8, phrase: usize) -> Slice {
        Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![],
            measure: 0,
            metric_level,
            phrase,
            key_signature: None,
        }
    }

    /// Builds a path of one-slice segments, each given as (key, root, quality, bass), one per slice.
    fn path(chords: &[(&str, &str, Quality, &str)]) -> Vec<Node> {
        let node = |(i, &(key, root, quality, bass)): (usize, &(&str, &str, Quality, &str))| {
            let segment = Segment {
                reading: reading(key, root, quality),
                start: i as u32,
                previous: if i == 0 { NONE } else { i as u32 - 1 },
                duration: 1.0,
                members_heard: 0b111,
                bass: Bass { lowest: Some((40, tpc(bass))), ..Bass::default() },
            };
            Node { parent: segment.previous, slice: i as u32, segment, score: 0.0 }
        };
        chords.iter().enumerate().map(node).collect()
    }

    /// Scores leaving the key of the path's last segment, with slices at the given metric levels and
    /// phrases (one more slice than the path, for the new segment).
    fn depart(chords: &[(&str, &str, Quality, &str)], slices: &[Slice]) -> f32 {
        let arena = path(chords);
        let history = History { arena: &arena, slices, last: arena.len() as u32 - 1 };
        unconfirmed_departure(&DepartureContext {
            previous: &arena[arena.len() - 1].segment,
            slice: &slices[chords.len()],
            history,
        })
    }

    fn strong(count: usize) -> Vec<Slice> {
        (0..count).map(|_| slice(4, 0)).collect()
    }

    const MAJOR: Quality = Quality::Major;

    #[test]
    fn leaving_an_uncadenced_area_costs() {
        let chords = [("C", "C", MAJOR, "C"), ("G", "D", MAJOR, "D"), ("G", "G", MAJOR, "B")];
        assert_eq!(depart(&chords, &strong(4)), -1.0);
    }

    #[test]
    fn leaving_an_area_confirmed_by_an_authentic_cadence_is_free() {
        let chords = [("C", "C", MAJOR, "C"), ("G", "D", MAJOR, "D"), ("G", "G", MAJOR, "G")];
        assert_eq!(depart(&chords, &strong(4)), 0.0);
    }

    #[test]
    fn authentic_cadence_with_the_tonic_on_a_weak_beat_does_not_confirm() {
        let chords = [("C", "C", MAJOR, "C"), ("G", "D", MAJOR, "D"), ("G", "G", MAJOR, "G")];
        let slices = [slice(4, 0), slice(4, 0), slice(1, 0), slice(4, 0)];
        assert_eq!(depart(&chords, &slices), -1.0);
    }

    #[test]
    fn leaving_an_area_confirmed_by_a_cadential_six_four_is_free() {
        let chords = [("C", "C", MAJOR, "C"), ("G", "G", MAJOR, "D"), ("G", "D", MAJOR, "D")];
        assert_eq!(depart(&chords, &strong(4)), 0.0);
    }

    #[test]
    fn leaving_an_area_after_a_fermata_is_free() {
        let chords = [("C", "C", MAJOR, "C"), ("G", "D", MAJOR, "D"), ("G", "G", MAJOR, "B")];
        let slices = [slice(4, 0), slice(4, 0), slice(4, 0), slice(4, 1)];
        assert_eq!(depart(&chords, &slices), 0.0);
    }

    #[test]
    fn leaving_the_first_area_of_the_piece_is_free() {
        let chords = [("G", "D", MAJOR, "D"), ("G", "G", MAJOR, "B")];
        assert_eq!(depart(&chords, &strong(3)), 0.0);
    }
}
