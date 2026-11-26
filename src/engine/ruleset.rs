//! A set of rules with weights, split by hook so the engine can call each group directly.

use super::rule::{
    AdmitFn, CloseContext, CloseFn, Hook, InterpretFn, ProposeFn, Rule, SliceContext, SliceFn,
    TransitionContext, TransitionFn,
};
use std::collections::HashMap;

pub type Weights = HashMap<String, f32>;

/// Rules grouped by hook, with weights applied and zero-weight rules left out.
pub struct RuleSet {
    pub rules: Vec<Rule>,
    pub weights: Vec<f32>,
    pub proposers: Vec<ProposeFn>,
    pub interpreters: Vec<InterpretFn>,
    pub slice_rules: Vec<(SliceFn, f32)>,
    pub transition_rules: Vec<(TransitionFn, f32)>,
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
                Hook::Transition(f) => self.transition_rules.push((f, weight)),
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

    pub fn score_transition(&self, context: &TransitionContext) -> f32 {
        self.transition_rules.iter().map(|(f, w)| w * f(context)).sum()
    }

    pub fn score_close(&self, context: &CloseContext) -> f32 {
        self.close_rules.iter().map(|(f, w)| w * f(context)).sum()
    }

    pub fn admits(&self, context: &TransitionContext) -> bool {
        self.admit_rules.iter().all(|f| f(context))
    }
}
