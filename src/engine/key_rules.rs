//! The key layer's hooks of a rule set, with their weights.

use super::key_contexts::{
    KeyCloseContext, KeyDepartureContext, KeySliceContext, KeyTransitionContext, Rival, RivalContext,
};
use super::key_hooks::{
    DistinguishFn, InterpretFn, KeyCloseFn, KeyDepartureFn, KeyHook, KeySliceFn, KeyTransitionFn,
    ProposeKeysFn, RivalCandidateFn, RivalFn,
};
use super::model::KeySegment;
use super::ruleset::Scorer;
use crate::score::slicing::Slice;
use crate::theory::key::Key;

/// A hook that scores transitions. Transition and departure hooks share one list in rule order,
/// so that their scores are summed in the same order whichever kind a rule uses.
#[derive(Clone, Copy)]
pub enum TransitionScorer {
    Each(KeyTransitionFn),
    Departure(KeyDepartureFn),
}

/// The key layer's hooks, grouped by kind, in rule order.
#[derive(Default)]
pub struct KeyRules {
    pub proposers: Vec<ProposeKeysFn>,
    pub interpreters: Vec<InterpretFn>,
    pub slice: Vec<Scorer<KeySliceFn>>,
    pub transition: Vec<Scorer<TransitionScorer>>,
    pub close: Vec<Scorer<KeyCloseFn>>,
    pub distinguish: Vec<(DistinguishFn, Box<[f64]>)>,
    pub rival: Vec<(RivalFn, Box<[f64]>)>,
    pub rival_candidate: Vec<(RivalCandidateFn, Box<[f64]>)>,
}

impl KeyRules {
    pub fn add(&mut self, hook: KeyHook, weight: f32, params: Box<[f64]>) {
        match hook {
            KeyHook::Propose(f) => self.proposers.push(f),
            KeyHook::Interpret(f) => self.interpreters.push(f),
            KeyHook::Slice(f) => self.slice.push((f, weight, params)),
            KeyHook::Transition(f) => self.transition.push((TransitionScorer::Each(f), weight, params)),
            KeyHook::Departure(f) => self.transition.push((TransitionScorer::Departure(f), weight, params)),
            KeyHook::Close(f) => self.close.push((f, weight, params)),
            KeyHook::Distinguish(f) => self.distinguish.push((f, params)),
            KeyHook::Rival(f) => self.rival.push((f, params)),
            KeyHook::RivalCandidate(f) => self.rival_candidate.push((f, params)),
        }
    }

    /// Returns every key that some proposer offers for a segment opening on the slice,
    /// in order and without repeats.
    pub fn propose(&self, slice: &Slice) -> Vec<Key> {
        let mut keys: Vec<Key> = self.proposers.iter().flat_map(|propose| propose(slice)).collect();
        keys.sort();
        keys.dedup();
        keys
    }

    pub fn score_slice(&self, context: &KeySliceContext) -> f32 {
        self.slice.iter().map(|(f, w, params)| w * f(context, params)).sum()
    }

    /// Returns the weighted score of each departure hook, in rule order, for `score_transition`.
    pub fn score_departures(&self, context: &KeyDepartureContext) -> Vec<f32> {
        let score = |(scorer, w, params): &Scorer<TransitionScorer>| match scorer {
            TransitionScorer::Departure(f) => Some(w * f(context, params)),
            TransitionScorer::Each(_) => None,
        };
        self.transition.iter().filter_map(score).collect()
    }

    /// Scores a transition, given the departure scores of its hypothesis from `score_departures`.
    /// They count only if the transition changes key; at the start of the piece there are none.
    pub fn score_transition(&self, context: &KeyTransitionContext, departures: &[f32]) -> f32 {
        let changes_key = context.departure().is_some();
        let mut departures = departures.iter();
        let score = |(scorer, w, params): &Scorer<TransitionScorer>| match scorer {
            TransitionScorer::Each(f) => w * f(context, params),
            TransitionScorer::Departure(_) => departures.next().filter(|_| changes_key).map_or(0.0, |d| *d),
        };
        self.transition.iter().map(score).sum()
    }

    pub fn score_close(&self, context: &KeyCloseContext) -> f32 {
        self.close.iter().map(|(f, w, params)| w * f(context, params)).sum()
    }

    /// Returns each rival hook's group for a segment, in rule order.
    pub fn rival_groups(&self, context: &RivalContext) -> Vec<Option<Rival>> {
        self.rival.iter().map(|(f, params)| f(context, params)).collect()
    }

    /// Appends each distinguish hook's value for the segment, in rule order.
    pub fn distinguish(&self, segment: &KeySegment, values: &mut Vec<u64>) {
        values.extend(self.distinguish.iter().map(|(f, params)| f(segment, params)));
    }
}
