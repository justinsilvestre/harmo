//! The chord layer's hooks of a rule set, with their weights.

use super::bass::{Bass, BassNote};
use super::chord_hooks::{
    BassFn, ChordCloseContext, ChordCloseFn, ChordHook, ChordSliceContext, ChordSliceFn,
    ChordTransitionContext, ChordTransitionFn, ProposeChordsFn,
};
use super::ruleset::Scorer;
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;

/// The chord layer's hooks, grouped by kind, in rule order.
#[derive(Default)]
pub struct ChordRules {
    pub proposers: Vec<ProposeChordsFn>,
    pub slice: Vec<Scorer<ChordSliceFn>>,
    pub transition: Vec<Scorer<ChordTransitionFn>>,
    pub close: Vec<Scorer<ChordCloseFn>>,
    pub bass: Vec<(BassFn, Box<[f64]>)>,
}

impl ChordRules {
    pub fn add(&mut self, hook: ChordHook, weight: f32, params: Box<[f64]>) {
        match hook {
            ChordHook::Propose(f) => self.proposers.push(f),
            ChordHook::Slice(f) => self.slice.push((f, weight, params)),
            ChordHook::Transition(f) => self.transition.push((f, weight, params)),
            ChordHook::Close(f) => self.close.push((f, weight, params)),
            ChordHook::Bass(f) => self.bass.push((f, params)),
        }
    }

    /// Returns every chord that some proposer offers for a segment opening on the slice,
    /// in order and without repeats.
    pub fn propose(&self, slice: &Slice) -> Vec<Chord> {
        let mut chords: Vec<Chord> = self.proposers.iter().flat_map(|propose| propose(slice)).collect();
        chords.sort();
        chords.dedup();
        chords
    }

    pub fn score_slice(&self, context: &ChordSliceContext) -> f32 {
        self.slice.iter().map(|(f, w, params)| w * f(context, params)).sum()
    }

    pub fn score_transition(&self, context: &ChordTransitionContext) -> f32 {
        self.transition.iter().map(|(f, w, params)| w * f(context, params)).sum()
    }

    pub fn score_close(&self, context: &ChordCloseContext) -> f32 {
        self.close.iter().map(|(f, w, params)| w * f(context, params)).sum()
    }

    /// Returns a segment's bass evidence after the bass hooks hear one more chord tone.
    pub fn hear_bass(&self, bass: Bass, note: &BassNote) -> Bass {
        self.bass.iter().fold(bass, |bass, (f, params)| f(bass, note, params))
    }
}
