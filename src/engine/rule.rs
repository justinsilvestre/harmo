//! The interface between the engine and the rules it hooks into.

use super::bass::{Bass, BassNote};
use super::layers::{Layers, NO_LAYERS};
use super::model::{ChordSegment, Function, NONE, Reading, Segment};
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;
use crate::theory::key::Key;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Layer {
    Chord,
    Key,
    Function,
}

/// A pointer into the source catalog, e.g. a page range in a textbook.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Citation {
    pub source: &'static str,
    pub locator: &'static str,
}

/// A rule: one hook into the engine, documented with its sources.
///
/// Scoring hooks return unweighted evidence; the engine multiplies it by the rule's weight.
/// Generator and constraint hooks are switched off by a weight of zero.
#[derive(Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub layer: Layer,
    pub summary: &'static str,
    pub citations: &'static [Citation],
    pub default_weight: f32,
    /// The constants inside the rule that can be overridden at runtime, in the order of their `index`.
    pub params: &'static [Param],
    pub hook: Hook,
}

impl Rule {
    /// Returns the default value of each of the rule's parameters, in order.
    pub fn default_params(&self) -> Vec<f64> {
        self.params.iter().map(|p| p.default).collect()
    }
}

/// A constant inside a rule, such as a threshold, a duration or a ratio, that can be overridden
/// at runtime as `<rule id>.<name>` beside the rule weights.
///
/// A scoring hook receives the values of its rule's parameters in order and reads each one with `read`.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Param {
    /// The parameter's position in its rule's `params`.
    pub index: usize,
    pub name: &'static str,
    pub default: f64,
    pub unit: &'static str,
    /// Where the default comes from: "cited: <source, page>", or "measured: <sweep or commit>"
    /// with the values tried and what they gave.
    pub provenance: &'static str,
}

impl Param {
    #[inline]
    pub fn read(&self, values: &[f64]) -> f64 {
        values[self.index]
    }
}

pub type ProposeFn = fn(&Slice) -> Vec<Chord>;
pub type ProposeKeysFn = fn(&Slice) -> Vec<Key>;
pub type InterpretFn = fn(Key, Chord) -> Vec<Function>;
/// Scoring hooks receive the context and the values of their rule's parameters.
/// Hooks of the chord layer receive only the chord layer's state, never a key.
pub type ChordSliceFn = fn(&ChordSliceContext, &[f64]) -> f32;
pub type ChordTransitionFn = fn(&ChordTransitionContext, &[f64]) -> f32;
pub type ChordCloseFn = fn(&ChordCloseContext, &[f64]) -> f32;
pub type SliceFn = fn(&SliceContext, &[f64]) -> f32;
pub type TransitionFn = fn(&TransitionContext, &[f64]) -> f32;
pub type DepartureFn = fn(&DepartureContext, &[f64]) -> f32;
pub type CloseFn = fn(&CloseContext, &[f64]) -> f32;
pub type AdmitFn = fn(&TransitionContext) -> bool;
/// Returns a segment's bass evidence after it hears one more chord tone.
pub type BassFn = fn(Bass, &BassNote, &[f64]) -> Bass;
/// Returns an opaque value for a segment; the engine only compares values for equality.
pub type DistinguishFn = fn(&Segment, &[f64]) -> u64;
/// Returns the rival group of a hypothesis's last segment, or `None` if it has no rival reading.
pub type RivalFn = fn(&RivalContext, &[f64]) -> Option<u64>;
/// Returns a rival reading of a slice for a candidate reading of it, or `None`.
pub type RivalCandidateFn = fn(&Reading, &Slice, &[f64]) -> Option<Reading>;

#[derive(Clone, Copy)]
pub enum Hook {
    /// Proposes chords that a new segment opening on this slice might have.
    ProposeChords(ProposeFn),
    /// Proposes keys in which a new segment opening on this slice might be read.
    ProposeKeys(ProposeKeysFn),
    /// Lists the ways a chord can function in a key.
    Interpret(InterpretFn),
    /// Scores how well one slice fits a chord, in the chord layer.
    ChordSlice(ChordSliceFn),
    /// Scores opening a new chord segment after the previous one, in the chord layer.
    ChordTransition(ChordTransitionFn),
    /// Scores a chord segment once its extent is known, in the chord layer.
    ChordClose(ChordCloseFn),
    /// Scores how well one slice fits a reading, in the key layer.
    Slice(SliceFn),
    /// Scores opening a new segment after the previous one.
    Transition(TransitionFn),
    /// Scores opening a new segment in a different key from the previous one, from the hypothesis
    /// and the slice alone. Since the new reading plays no part, the engine scores it once per
    /// hypothesis and slice instead of once per new reading.
    Departure(DepartureFn),
    /// Scores a segment once its extent is known.
    Close(CloseFn),
    /// Rejects transitions outright (a hard constraint).
    Admit(AdmitFn),
    /// Says which segments count as the same interpretation when the beam is pruned. Of the
    /// hypotheses whose last `Config::distinct_depth` segments get the same values from every
    /// distinguish hook, only the best is kept.
    Distinguish(DistinguishFn),
    /// Decides a segment's bass note, and so its inversion, as the segment hears each chord tone.
    /// Bass hooks run in rule order, each on the evidence the previous one returned.
    Bass(BassFn),
    /// Names rival readings of the same music. A new segment normally opens after the one hypothesis
    /// that leads into it best. When the hook puts that hypothesis's last segment in a group, the
    /// best hypothesis whose last segment has the same group but another reading opens the new
    /// segment as well, so that the rival reading stays in the search as a hypothesis of its own.
    Rival(RivalFn),
    /// Names a rival reading of a slice for each candidate reading kept for a segment opening on it.
    /// When the proposers and interpreters produce the rival reading but its fit to the slice falls
    /// below the candidate limit, it is kept as a candidate as well.
    RivalCandidate(RivalCandidateFn),
}

impl Hook {
    pub fn kind(&self) -> &'static str {
        match self {
            Hook::ProposeChords(_) => "propose_chords",
            Hook::ProposeKeys(_) => "propose_keys",
            Hook::Interpret(_) => "interpret",
            Hook::ChordSlice(_) => "chord_slice",
            Hook::ChordTransition(_) => "chord_transition",
            Hook::ChordClose(_) => "chord_close",
            Hook::Slice(_) => "slice",
            Hook::Transition(_) => "transition",
            Hook::Departure(_) => "departure",
            Hook::Close(_) => "close",
            Hook::Admit(_) => "admit",
            Hook::Distinguish(_) => "distinguish",
            Hook::Bass(_) => "bass",
            Hook::Rival(_) => "rival",
            Hook::RivalCandidate(_) => "rival_candidate",
        }
    }
}

pub struct ChordSliceContext<'a> {
    pub slice: &'a Slice,
    /// The slice after this one in the score, if any.
    pub next_slice: Option<&'a Slice>,
    /// Every slice of the score, for rules that weigh the musical context around this slice.
    pub slices: &'a [Slice],
    /// The index of this slice in `slices`.
    pub index: usize,
    pub chord: Chord,
    /// True when the slice opens the segment.
    pub is_opening: bool,
}

pub struct ChordTransitionContext<'a> {
    /// The chord segment being closed, or `None` at the start of the piece.
    pub previous: Option<&'a ChordSegment>,
    pub next: Chord,
    pub slice: &'a Slice,
    /// Every slice of the score, so that the previous segment's slices can be read from its `start`.
    pub slices: &'a [Slice],
}

pub struct ChordCloseContext<'a> {
    pub segment: &'a ChordSegment,
    /// The slices covered by the segment.
    pub slices: &'a [Slice],
    /// The slice that opens the following segment, if any.
    pub next_slice: Option<&'a Slice>,
}

pub struct SliceContext<'a> {
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

pub struct TransitionContext<'a> {
    /// The segment being closed, or `None` at the start of the piece.
    pub previous: Option<&'a Segment>,
    pub next: &'a Reading,
    pub slice: &'a Slice,
    pub history: History<'a>,
}

impl<'a> TransitionContext<'a> {
    /// Returns the context for departure hooks if the transition leaves the previous segment's key.
    pub fn departure(&self) -> Option<DepartureContext<'a>> {
        let previous = self.previous.filter(|p| p.reading.key != self.next.key)?;
        Some(DepartureContext { previous, slice: self.slice, history: self.history })
    }
}

pub struct DepartureContext<'a> {
    /// The segment being closed.
    pub previous: &'a Segment,
    /// The slice that opens the new segment.
    pub slice: &'a Slice,
    pub history: History<'a>,
}

pub struct CloseContext<'a> {
    pub segment: &'a Segment,
    /// The slices covered by the segment.
    pub slices: &'a [Slice],
    /// The slice that opens the following segment, if any.
    pub next_slice: Option<&'a Slice>,
}

pub struct RivalContext<'a> {
    pub segment: &'a Segment,
    /// The slices covered by the segment so far.
    pub slices: &'a [Slice],
    /// The slice before the segment, if any.
    pub previous_slice: Option<&'a Slice>,
}

/// Read-only access to the segments preceding a transition, most recent first, as both layers
/// read them.
#[derive(Clone, Copy)]
pub struct History<'a> {
    pub layers: &'a Layers,
    /// Every slice of the score, so that a past segment's slices can be read from its `start`.
    pub slices: &'a [Slice],
    /// Index in the hypotheses of the last hypothesis of the most recent segment, or `NONE`.
    pub last: u32,
}

impl<'a> History<'a> {
    pub fn segments(self) -> impl Iterator<Item = Segment> + 'a {
        let layers = self.layers;
        let at = move |hypothesis: u32| (hypothesis != NONE).then(|| layers.segment(hypothesis));
        std::iter::successors(at(self.last), move |s| at(s.previous))
    }

    /// A history with no segments before it.
    pub fn empty(slices: &'a [Slice]) -> History<'a> {
        History { layers: &NO_LAYERS, slices, last: NONE }
    }
}

/// Builds the arenas of a chain of segments, for tests: segment i becomes node i, whose `previous`
/// is taken from the segment.
#[cfg(test)]
pub fn arenas_of(segments: &[Segment]) -> Layers {
    use super::model::{Hearing, Hypothesis};
    let node = |(i, s): (usize, &Segment)| Hypothesis {
        parent: s.previous,
        slice: i as u32,
        hearing: i as u32,
        key: s.reading.key,
        function: s.reading.function,
        previous: s.previous,
        score: 0.0,
    };
    let chord = |(i, s): (usize, &Segment)| Hearing {
        parent: s.previous,
        slice: i as u32,
        segment: s.chord_segment(s.previous),
        score: 0.0,
    };
    let hypotheses = segments.iter().enumerate().map(node).collect();
    Layers { hearings: segments.iter().enumerate().map(chord).collect(), hypotheses }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Quality;
    use crate::theory::key::Mode;

    fn reading(tonic: &str) -> Reading {
        let tpc = tpc_from_name(tonic).unwrap();
        let chord = Chord::new(tpc, Quality::Major);
        Reading { key: Key::new(tpc, Mode::Major), chord, function: Function::DIATONIC }
    }

    fn segment(tonic: &str) -> Segment {
        let (start, previous, duration, members_heard) = (0, NONE, 1.0, 0);
        Segment { reading: reading(tonic), start, previous, duration, members_heard, bass: Bass::default() }
    }

    fn slice() -> Slice {
        let (start, duration, notes, measure, metric_level, phrase) = (0.0, 1.0, vec![], 0, 0, 0);
        Slice { start, duration, notes, measure, metric_level, phrase, key_signature: None }
    }

    /// Reports whether a transition from the given previous key to C major is a departure.
    fn is_departure(previous: Option<&str>) -> bool {
        let (previous, next, slice) = (previous.map(segment), reading("C"), slice());
        let history = History::empty(&[]);
        TransitionContext { previous: previous.as_ref(), next: &next, slice: &slice, history }
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
