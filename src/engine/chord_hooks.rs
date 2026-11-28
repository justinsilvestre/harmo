//! The hooks of the chord layer, and what they receive. They see only the chord layer's state,
//! never a key.

use super::bass::{Bass, BassNote};
use super::layers::{Layers, NO_LAYERS};
use super::model::{ChordHypothesisId, ChordSegment};
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;

pub type ProposeChordsFn = fn(&Slice) -> Vec<Chord>;
/// Scoring hooks receive the context and the values of their rule's parameters.
pub type ChordSliceFn = fn(&ChordSliceContext, &[f64]) -> f32;
pub type ChordTransitionFn = fn(&ChordTransitionContext, &[f64]) -> f32;
pub type ChordCloseFn = fn(&ChordCloseContext, &[f64]) -> f32;
/// Returns a segment's bass evidence after it hears one more chord tone.
pub type BassFn = fn(Bass, &BassNote, &[f64]) -> Bass;

#[derive(Clone, Copy)]
pub enum ChordHook {
    /// Proposes chords that a new segment opening on this slice might have.
    Propose(ProposeChordsFn),
    /// Scores how well one slice fits a chord.
    Slice(ChordSliceFn),
    /// Scores opening a new chord segment after the previous one.
    Transition(ChordTransitionFn),
    /// Scores a chord segment once its extent is known.
    Close(ChordCloseFn),
    /// Decides a segment's bass note, and so its inversion, as the segment hears each chord tone.
    /// Bass hooks run in rule order, each on the evidence the previous one returned.
    Bass(BassFn),
}

impl ChordHook {
    pub fn kind(&self) -> &'static str {
        match self {
            ChordHook::Propose(_) => "chord_propose",
            ChordHook::Slice(_) => "chord_slice",
            ChordHook::Transition(_) => "chord_transition",
            ChordHook::Close(_) => "chord_close",
            ChordHook::Bass(_) => "chord_bass",
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

impl<'a> ChordSliceContext<'a> {
    /// Returns the context for scoring slice `index` of the score against a chord.
    pub fn at(slices: &'a [Slice], index: usize, chord: Chord, is_opening: bool) -> ChordSliceContext<'a> {
        let (slice, next_slice) = (&slices[index], slices.get(index + 1));
        ChordSliceContext { slice, next_slice, slices, index, chord, is_opening }
    }
}

pub struct ChordTransitionContext<'a> {
    /// The chord segment being closed, or `None` at the start of the piece.
    pub previous: Option<&'a ChordSegment>,
    pub next: Chord,
    pub slice: &'a Slice,
    pub history: ChordHistory<'a>,
}

pub struct ChordCloseContext<'a> {
    pub segment: &'a ChordSegment,
    /// The slices covered by the segment.
    pub slices: &'a [Slice],
    /// The slice that opens the following segment, if any.
    pub next_slice: Option<&'a Slice>,
}

/// Read-only access to the chord segments preceding a transition, most recent first.
/// It gives only the chord layer's state.
#[derive(Clone, Copy)]
pub struct ChordHistory<'a> {
    layers: &'a Layers,
    /// Every slice of the score, so that a past segment's slices can be read from its `start`.
    pub slices: &'a [Slice],
    /// The last chord hypothesis of the most recent segment, or `ChordHypothesisId::NONE`.
    last: ChordHypothesisId,
}

impl<'a> ChordHistory<'a> {
    pub fn new(layers: &'a Layers, slices: &'a [Slice], last: ChordHypothesisId) -> ChordHistory<'a> {
        ChordHistory { layers, slices, last }
    }

    pub fn segments(self) -> impl Iterator<Item = ChordSegment> + 'a {
        let layers = self.layers;
        layers.chord.path_back(self.last).map(|id| layers.chord.get(id).unit)
    }

    /// A history with no segments before it.
    pub fn empty(slices: &'a [Slice]) -> ChordHistory<'a> {
        ChordHistory::new(&NO_LAYERS, slices, ChordHypothesisId::NONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Function, Reading};
    use crate::engine::test_layers::{key_segment, path_of};
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Quality;
    use crate::theory::key::{Key, Mode};

    #[test]
    fn a_chord_history_gives_the_chord_segments_most_recent_first() {
        let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
        let reading = |root: &str| Reading {
            chord: Chord::new(tpc_from_name(root).unwrap(), Quality::Major),
            function: Function::diatonic(key),
        };
        let path = path_of(&["C", "F", "G"].map(|root| key_segment(reading(root))));
        let history = ChordHistory::new(&path, &[], ChordHypothesisId::new(2));
        let starts: Vec<u32> = history.segments().map(|s| s.start).collect();
        assert_eq!(starts, [2, 1, 0]);
    }
}
