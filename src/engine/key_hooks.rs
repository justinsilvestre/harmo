//! The hooks of the key layer, and what they receive. They may read the chord layer's state, as a
//! later layer interprets an earlier one.

use super::layers::{Layers, NO_LAYERS};
use super::model::{Function, KeySegment, NONE, Reading};
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;
use crate::theory::key::Key;

pub type ProposeKeysFn = fn(&Slice) -> Vec<Key>;
pub type InterpretFn = fn(Key, Chord) -> Vec<Function>;
/// Scoring hooks receive the context and the values of their rule's parameters.
pub type KeySliceFn = fn(&KeySliceContext, &[f64]) -> f32;
pub type KeyTransitionFn = fn(&KeyTransitionContext, &[f64]) -> f32;
pub type KeyDepartureFn = fn(&KeyDepartureContext, &[f64]) -> f32;
pub type KeyCloseFn = fn(&KeyCloseContext, &[f64]) -> f32;
pub type AdmitFn = fn(&KeyTransitionContext) -> bool;
/// Returns an opaque value for a segment; the engine only compares values for equality.
pub type DistinguishFn = fn(&KeySegment, &[f64]) -> u64;
/// Returns the rival group of a hypothesis's last segment, or `None` if it has no rival hearing.
pub type RivalFn = fn(&RivalContext, &[f64]) -> Option<u64>;
/// Returns a rival reading of a slice for a candidate reading of it, or `None`.
pub type RivalCandidateFn = fn(&Reading, &Slice, &[f64]) -> Option<Reading>;

#[derive(Clone, Copy)]
pub enum KeyHook {
    /// Proposes keys in which a new segment opening on this slice might be read.
    Propose(ProposeKeysFn),
    /// Lists the ways a chord can function in a key.
    Interpret(InterpretFn),
    /// Scores how well one slice fits a reading.
    Slice(KeySliceFn),
    /// Scores opening a new segment after the previous one.
    Transition(KeyTransitionFn),
    /// Scores opening a new segment in a different key from the previous one, from the hypothesis
    /// and the slice alone. Since the new reading plays no part, the engine scores it once per
    /// hypothesis and slice instead of once per new reading.
    Departure(KeyDepartureFn),
    /// Scores a segment once its extent is known.
    Close(KeyCloseFn),
    /// Rejects transitions outright (a hard constraint).
    Admit(AdmitFn),
    /// Says which segments count as the same interpretation when the beam is pruned. Of the
    /// hypotheses whose last `Config::distinct_depth` segments get the same values from every
    /// distinguish hook, only the best is kept.
    Distinguish(DistinguishFn),
    /// Names rival hearings of the same music. A new segment normally opens after the one hypothesis
    /// that leads into it best. When the hook puts that hypothesis's last segment in a group, the
    /// best hypothesis whose last segment has the same group but hears another chord there opens
    /// the new segment as well, so that the rival hearing stays in the search.
    Rival(RivalFn),
    /// Names a rival reading of a slice for each candidate reading kept for a segment opening on it.
    /// When the proposers and interpreters produce the rival reading but its fit to the slice falls
    /// below the candidate limit, it is kept as a candidate as well.
    RivalCandidate(RivalCandidateFn),
}

impl KeyHook {
    pub fn kind(&self) -> &'static str {
        match self {
            KeyHook::Propose(_) => "propose_keys",
            KeyHook::Interpret(_) => "interpret",
            KeyHook::Slice(_) => "key_slice",
            KeyHook::Transition(_) => "key_transition",
            KeyHook::Departure(_) => "key_departure",
            KeyHook::Close(_) => "key_close",
            KeyHook::Admit(_) => "admit",
            KeyHook::Distinguish(_) => "distinguish",
            KeyHook::Rival(_) => "rival",
            KeyHook::RivalCandidate(_) => "rival_candidate",
        }
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
        let previous = self.previous.filter(|p| p.key != self.next.key)?;
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

/// Read-only access to the segments preceding a transition, most recent first, as the key layer
/// reads them.
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
        let at = move |hypothesis: u32| (hypothesis != NONE).then(|| layers.segment(hypothesis));
        std::iter::successors(at(self.last), move |s| at(s.previous))
    }

    /// A history with no segments before it.
    pub fn empty(slices: &'a [Slice]) -> History<'a> {
        History { layers: &NO_LAYERS, slices, last: NONE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Quality;
    use crate::theory::key::Mode;

    fn reading(tonic: &str) -> Reading {
        let tpc = tpc_from_name(tonic).unwrap();
        let chord = Chord::new(tpc, Quality::Major);
        Reading { key: Key::new(tpc, Mode::Major), chord, function: Function::DIATONIC }
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
