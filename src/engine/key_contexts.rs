//! What the hooks of the key layer receive. They may read the chord layer's state,
//! as a later layer interprets an earlier one.

use super::layers::{Layers, NO_LAYERS};
use super::model::{KeySegment, NONE, Reading};
use crate::score::slicing::Slice;

/// A segment's place among rival readings of the same music.
/// The engine only compares the values for equality:
/// two segments are rivals when they have the same group and different members.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rival {
    pub group: u64,
    pub member: u64,
}

impl Rival {
    pub fn is_rival_of(self, other: Rival) -> bool {
        self.group == other.group && self.member != other.member
    }
}

pub struct KeySliceContext<'a> {
    pub slice: &'a Slice,
    /// The slice after this one in the score, if any.
    pub next_slice: Option<&'a Slice>,
    /// Every slice of the score, for rules that weigh the musical context around this slice.
    pub slices: &'a [Slice],
    /// The index of this slice in `slices`.
    pub index: usize,
    pub reading: &'a Reading,
    /// True when the slice opens the segment.
    pub is_opening: bool,
}

impl<'a> KeySliceContext<'a> {
    /// Returns the context for scoring slice `index` of the score against a reading.
    pub fn at(
        slices: &'a [Slice],
        index: usize,
        reading: &'a Reading,
        is_opening: bool,
    ) -> KeySliceContext<'a> {
        let (slice, next_slice) = (&slices[index], slices.get(index + 1));
        KeySliceContext { slice, next_slice, slices, index, reading, is_opening }
    }
}

pub struct KeyTransitionContext<'a> {
    /// The segment being closed, or `None` at the start of the piece.
    pub previous: Option<&'a KeySegment>,
    pub next: &'a Reading,
    pub slice: &'a Slice,
    pub history: History<'a>,
}

impl<'a> KeyTransitionContext<'a> {
    /// Returns the context for departure hooks if the transition leaves the previous segment's key.
    pub fn departure(&self) -> Option<KeyDepartureContext<'a>> {
        let previous = self.previous.filter(|p| p.function.key != self.next.function.key)?;
        Some(KeyDepartureContext { previous, slice: self.slice, history: self.history })
    }
}

pub struct KeyDepartureContext<'a> {
    /// The segment being closed.
    pub previous: &'a KeySegment,
    /// The slice that opens the new segment.
    pub slice: &'a Slice,
    pub history: History<'a>,
}

pub struct KeyCloseContext<'a> {
    pub segment: &'a KeySegment,
    /// The slices covered by the segment.
    pub slices: &'a [Slice],
    /// The slice that opens the following segment, if any.
    pub next_slice: Option<&'a Slice>,
}

pub struct RivalContext<'a> {
    pub segment: &'a KeySegment,
    /// The slices covered by the segment so far.
    pub slices: &'a [Slice],
    /// The slice before the segment, if any.
    pub previous_slice: Option<&'a Slice>,
}

/// Read-only access to the segments preceding a transition, most recent first,
/// as the key layer reads them.
#[derive(Clone, Copy)]
pub struct History<'a> {
    pub layers: &'a Layers,
    /// Every slice of the score, so that a past segment's slices can be read from its `start`.
    pub slices: &'a [Slice],
    /// Index in the hypotheses of the last hypothesis of the most recent segment, or `NONE`.
    pub last: u32,
}

impl<'a> History<'a> {
    pub fn segments(self) -> impl Iterator<Item = KeySegment> + 'a {
        let layers = self.layers;
        layers.path_back(self.last).map(|hypothesis| layers.segment(hypothesis))
    }

    /// A history with no segments before it.
    pub fn empty(slices: &'a [Slice]) -> History<'a> {
        History { layers: &NO_LAYERS, slices, last: NONE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::Function;
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    fn reading(tonic: &str) -> Reading {
        let tpc = tpc_from_name(tonic).unwrap();
        let chord = Chord::new(tpc, Quality::Major);
        Reading { chord, function: Function::diatonic(Key::new(tpc, Mode::Major)) }
    }

    fn slice() -> Slice {
        let (start, duration, notes, measure, metric_level, phrase) = (0.0, 1.0, vec![], 0, 0, 0);
        Slice { start, duration, notes, measure, metric_level, phrase, key_signature: None }
    }

    /// Reports whether a transition from the given previous key to C major is a departure.
    fn is_departure(previous: Option<&str>) -> bool {
        let (previous, next, slice) =
            (previous.map(|tonic| key_segment(reading(tonic))), reading("C"), slice());
        let history = History::empty(&[]);
        KeyTransitionContext { previous: previous.as_ref(), next: &next, slice: &slice, history }
            .departure()
            .is_some()
    }

    #[test]
    fn a_transition_to_another_key_is_a_departure() {
        assert!(is_departure(Some("G")));
    }

    #[test]
    fn a_transition_within_the_key_is_not_a_departure() {
        assert!(!is_departure(Some("C")));
    }

    #[test]
    fn the_opening_segment_departs_from_nothing() {
        assert!(!is_departure(None));
    }
}
