//! A set of rules with weights, split by layer and by hook so the engine can call each group
//! directly.

use super::chord_rules::ChordRules;
use super::key_rules::KeyRules;
use super::rule::{Hook, Rule};
use std::collections::HashMap;

/// Rule weights by rule id, and parameter values by `<rule id>.<parameter name>`.
pub type Weights = HashMap<String, f32>;

/// A scoring hook with its rule's weight and the values of its rule's parameters.
pub type Scorer<F> = (F, f32, Box<[f64]>);

/// Rules grouped by layer and hook, with weights applied and zero-weight rules left out.
pub struct RuleSet {
    pub rules: Vec<Rule>,
    pub weights: Vec<f32>,
    /// The values of each rule's parameters, in rule order.
    pub params: Vec<Vec<f64>>,
    pub chord: ChordRules,
    pub key: KeyRules,
}

impl RuleSet {
    /// Builds a rule set, overriding default weights by rule id and default parameter values by
    /// `<rule id>.<parameter name>`.
    /// Keys in `overrides` that match no rule or parameter are returned as an error, to catch typos.
    pub fn new(rules: Vec<Rule>, overrides: &Weights) -> Result<RuleSet, String> {
        let mut weights: Vec<f32> = rules.iter().map(|r| r.default_weight).collect();
        let mut params: Vec<Vec<f64>> = rules.iter().map(Rule::default_params).collect();
        for (key, &value) in overrides {
            if let Some(index) = rules.iter().position(|r| r.id == key.as_str()) {
                weights[index] = value;
            } else if let Some((index, param)) = find_param(&rules, key) {
                params[index][param] = f64::from(value);
            } else {
                return Err(format!("unknown rule id or parameter in weights: {key}"));
            }
        }
        let (mut chord, mut key) = (ChordRules::default(), KeyRules::default());
        for ((rule, &weight), params) in rules.iter().zip(&weights).zip(&params) {
            let params: Box<[f64]> = params.clone().into_boxed_slice();
            match rule.hook {
                _ if weight == 0.0 => {}
                Hook::Chord(hook) => chord.add(hook, weight, params),
                Hook::Key(hook) => key.add(hook, weight, params),
            }
        }
        Ok(RuleSet { rules, weights, params, chord, key })
    }

    pub fn weight_of(&self, id: &str) -> Option<f32> {
        self.rules.iter().position(|r| r.id == id).map(|i| self.weights[i])
    }

    /// Returns every parameter value that differs from its default, keyed by `<rule id>.<name>`.
    pub fn overridden_params(&self) -> Vec<(String, f64)> {
        let rules = self.rules.iter().zip(&self.params);
        let changed = |(rule, values): (&Rule, &Vec<f64>)| {
            let pairs = rule.params.iter().zip(values.iter());
            pairs
                .filter(|(p, v)| p.default != **v)
                .map(|(p, v)| (format!("{}.{}", rule.id, p.name), *v))
                .collect::<Vec<_>>()
        };
        rules.flat_map(changed).collect()
    }
}

/// Finds the rule and parameter that a key `<rule id>.<parameter name>` names, as their indices.
fn find_param(rules: &[Rule], key: &str) -> Option<(usize, usize)> {
    let (id, name) = key.rsplit_once('.')?;
    let index = rules.iter().position(|r| r.id == id)?;
    let param = rules[index].params.iter().position(|p| p.name == name)?;
    Some((index, param))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::engine::key_hooks::{History, KeyDepartureContext, KeyHook, KeyTransitionContext};
    use crate::engine::model::{Function, Reading};
    use crate::engine::rule::{Layer, Param};
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    const COST: Param = Param { index: 0, name: "cost", default: 1.0, unit: "", provenance: "" };

    fn departure_cost(_: &KeyDepartureContext, params: &[f64]) -> f32 {
        -COST.read(params) as f32
    }

    fn rule_set_with(overrides: &[(&str, f32)]) -> Result<RuleSet, String> {
        let rule = Rule {
            id: "test.departure",
            layer: Layer::Key,
            summary: "",
            citations: &[],
            default_weight: 0.5,
            params: &[COST],
            hook: Hook::Key(KeyHook::Departure(departure_cost)),
        };
        RuleSet::new(vec![rule], &overrides.iter().map(|(k, v)| (k.to_string(), *v)).collect())
    }

    fn rule_set() -> RuleSet {
        rule_set_with(&[]).unwrap()
    }

    fn reading(tonic: &str) -> Reading {
        let tpc = tpc_from_name(tonic).unwrap();
        let chord = Chord::new(tpc, Quality::Major);
        Reading { chord, function: Function::diatonic(Key::new(tpc, Mode::Major)) }
    }

    fn slice() -> Slice {
        let (start, duration, notes, measure, metric_level, phrase) = (0.0, 1.0, vec![], 0, 0, 0);
        Slice { start, duration, notes, measure, metric_level, phrase, key_signature: None }
    }

    /// Scores moving from a C major segment to a reading in the given key.
    fn transition_to(tonic: &str) -> f32 {
        transition_with(&rule_set(), tonic)
    }

    fn transition_with(rules: &RuleSet, tonic: &str) -> f32 {
        let (previous, slice) = (key_segment(reading("C")), slice());
        let history = History::empty(&[]);
        let departures =
            rules.key.score_departures(&KeyDepartureContext { previous: &previous, slice: &slice, history });
        let context =
            KeyTransitionContext { previous: Some(&previous), next: &reading(tonic), slice: &slice, history };
        rules.key.score_transition(&context, &departures)
    }

    #[test]
    fn departure_scores_count_when_the_key_changes() {
        assert_eq!(transition_to("G"), -0.5);
    }

    #[test]
    fn departure_scores_do_not_count_when_the_key_stays() {
        assert_eq!(transition_to("C"), 0.0);
    }

    #[test]
    fn a_parameter_is_overridden_by_rule_id_and_name() {
        let rules = rule_set_with(&[("test.departure.cost", 2.0)]).unwrap();
        assert_eq!(transition_with(&rules, "G"), -1.0);
    }

    #[test]
    fn an_unknown_parameter_is_an_error() {
        assert!(rule_set_with(&[("test.departure.costs", 2.0)]).is_err());
    }

    #[test]
    fn overridden_parameters_are_listed() {
        let rules = rule_set_with(&[("test.departure.cost", 2.0)]).unwrap();
        assert_eq!(rules.overridden_params(), vec![("test.departure.cost".to_string(), 2.0)]);
    }
}
