//! A set of rules with weights, split by hook so the engine can call each group directly.

use super::rule::{
    AdmitFn, CloseContext, CloseFn, DepartureContext, DepartureFn, Hook, InterpretFn, ProposeFn, Rule,
    SliceContext, SliceFn, TransitionContext, TransitionFn,
};
use std::collections::HashMap;

pub type Weights = HashMap<String, f32>;

/// A hook that scores transitions. Transition and departure hooks share one list in rule order,
/// so that their scores are summed in the same order whichever kind a rule uses.
#[derive(Clone, Copy)]
pub enum TransitionScorer {
    Each(TransitionFn),
    Departure(DepartureFn),
}

/// Rules grouped by hook, with weights applied and zero-weight rules left out.
pub struct RuleSet {
    pub rules: Vec<Rule>,
    pub weights: Vec<f32>,
    pub proposers: Vec<ProposeFn>,
    pub interpreters: Vec<InterpretFn>,
    pub slice_rules: Vec<(SliceFn, f32)>,
    pub transition_rules: Vec<(TransitionScorer, f32)>,
    pub close_rules: Vec<(CloseFn, f32)>,
    pub admit_rules: Vec<AdmitFn>,
}

impl RuleSet {
    /// Builds a rule set, overriding default weights by rule id.
    /// Ids in `overrides` that match no rule are returned as an error, to catch typos.
    pub fn new(rules: Vec<Rule>, overrides: &Weights) -> Result<RuleSet, String> {
        if let Some(unknown) = overrides.keys().find(|id| !rules.iter().any(|r| r.id == id.as_str())) {
            return Err(format!("unknown rule id in weights: {unknown}"));
        }
        let weights: Vec<f32> =
            rules.iter().map(|r| *overrides.get(r.id).unwrap_or(&r.default_weight)).collect();
        let mut set = RuleSet {
            rules,
            weights,
            proposers: vec![],
            interpreters: vec![],
            slice_rules: vec![],
            transition_rules: vec![],
            close_rules: vec![],
            admit_rules: vec![],
        };
        set.group_hooks();
        Ok(set)
    }

    fn group_hooks(&mut self) {
        for (rule, &weight) in self.rules.iter().zip(&self.weights) {
            if weight == 0.0 {
                continue;
            }
            match rule.hook {
                Hook::ProposeChords(f) => self.proposers.push(f),
                Hook::Interpret(f) => self.interpreters.push(f),
                Hook::Slice(f) => self.slice_rules.push((f, weight)),
                Hook::Transition(f) => self.transition_rules.push((TransitionScorer::Each(f), weight)),
                Hook::Departure(f) => self.transition_rules.push((TransitionScorer::Departure(f), weight)),
                Hook::Close(f) => self.close_rules.push((f, weight)),
                Hook::Admit(f) => self.admit_rules.push(f),
            }
        }
    }

    pub fn weight_of(&self, id: &str) -> Option<f32> {
        self.rules.iter().position(|r| r.id == id).map(|i| self.weights[i])
    }

    pub fn score_slice(&self, context: &SliceContext) -> f32 {
        self.slice_rules.iter().map(|(f, w)| w * f(context)).sum()
    }

    /// Returns the weighted score of each departure hook, in rule order, for `score_transition`.
    pub fn score_departures(&self, context: &DepartureContext) -> Vec<f32> {
        let score = |(scorer, w): &(TransitionScorer, f32)| match scorer {
            TransitionScorer::Departure(f) => Some(w * f(context)),
            TransitionScorer::Each(_) => None,
        };
        self.transition_rules.iter().filter_map(score).collect()
    }

    /// Scores a transition, given the departure scores of its hypothesis from `score_departures`.
    /// They count only if the transition changes key; at the start of the piece there are none.
    pub fn score_transition(&self, context: &TransitionContext, departures: &[f32]) -> f32 {
        let changes_key = context.departure().is_some();
        let mut departures = departures.iter();
        let score = |(scorer, w): &(TransitionScorer, f32)| match scorer {
            TransitionScorer::Each(f) => w * f(context),
            TransitionScorer::Departure(_) => departures.next().filter(|_| changes_key).map_or(0.0, |d| *d),
        };
        self.transition_rules.iter().map(score).sum()
    }

    pub fn score_close(&self, context: &CloseContext) -> f32 {
        self.close_rules.iter().map(|(f, w)| w * f(context)).sum()
    }

    pub fn admits(&self, context: &TransitionContext) -> bool {
        self.admit_rules.iter().all(|f| f(context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Reading, Segment};
    use crate::engine::rule::{History, Layer};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    fn departure_cost(_: &DepartureContext) -> f32 {
        -1.0
    }

    fn rule_set() -> RuleSet {
        let rule = Rule {
            id: "test.departure",
            layer: Layer::Key,
            summary: "",
            citations: &[],
            default_weight: 0.5,
            hook: Hook::Departure(departure_cost),
        };
        RuleSet::new(vec![rule], &Weights::new()).unwrap()
    }

    fn reading(tonic: &str) -> Reading {
        let tpc = tpc_from_name(tonic).unwrap();
        let chord = Chord::new(tpc, Quality::Major);
        Reading { key: Key::new(tpc, Mode::Major), chord, tonicized: None, category: Category::Diatonic }
    }

    fn segment(tonic: &str) -> Segment {
        let (start, previous, duration, members_heard) = (0, NONE, 1.0, 0);
        Segment { reading: reading(tonic), start, previous, duration, members_heard, bass: Bass::default() }
    }

    fn slice() -> Slice {
        let (start, duration, notes, measure, metric_level, phrase) = (0.0, 1.0, vec![], 0, 0, 0);
        Slice { start, duration, notes, measure, metric_level, phrase, key_signature: None }
    }

    /// Scores moving from a C major segment to a reading in the given key.
    fn transition_to(tonic: &str) -> f32 {
        let (rules, previous, slice) = (rule_set(), segment("C"), slice());
        let history = History { arena: &[], slices: &[], last: NONE };
        let departures =
            rules.score_departures(&DepartureContext { previous: &previous, slice: &slice, history });
        let context =
            TransitionContext { previous: Some(&previous), next: &reading(tonic), slice: &slice, history };
        rules.score_transition(&context, &departures)
    }

    #[test]
    fn departure_scores_count_when_the_key_changes() {
        assert_eq!(transition_to("G"), -0.5);
    }

    #[test]
    fn departure_scores_do_not_count_when_the_key_stays() {
        assert_eq!(transition_to("C"), 0.0);
    }
}
