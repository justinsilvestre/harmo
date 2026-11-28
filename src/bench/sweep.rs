//! Weight sweeps: evaluate many weight settings on pieces that are loaded only once.

use super::metrics::{AUGMENTEDNET_FEATURES, FEATURES};
use super::report::{SetRecord, summarize};
use super::runner::{BenchPiece, EvalConfig, evaluate};
use crate::engine::rule::Rule;
use crate::engine::ruleset::{RuleSet, Weights};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A sweep definition, read from JSON.
#[derive(Clone, Debug, Deserialize)]
pub struct SweepSpec {
    /// Benchmark sets to load.
    pub sets: Vec<String>,
    /// The set and feature (one of `FEATURES`,
    /// or "mean" for the mean of AugmentedNet's six) used to rank settings.
    pub objective_set: String,
    pub objective_feature: String,
    /// Weights shared by every setting.
    #[serde(default)]
    pub base: BTreeMap<String, f32>,
    /// Values to try for each rule id.
    pub grid: BTreeMap<String, Vec<f32>>,
    /// Varies one rule at a time from the base instead of trying every combination.
    #[serde(default)]
    pub one_at_a_time: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SweepResult {
    pub weights: BTreeMap<String, f32>,
    pub objective: f64,
    pub sets: Vec<SetRecord>,
}

pub fn settings(spec: &SweepSpec) -> Vec<BTreeMap<String, f32>> {
    if spec.one_at_a_time {
        let variations = spec.grid.iter().flat_map(|(id, values)| values.iter().map(move |v| (id, *v)));
        let vary = |(id, value): (&String, f32)| {
            let mut weights = spec.base.clone();
            weights.insert(id.clone(), value);
            weights
        };
        return std::iter::once(spec.base.clone()).chain(variations.map(vary)).collect();
    }
    let mut combinations = vec![spec.base.clone()];
    for (id, values) in &spec.grid {
        let mut extended = Vec::with_capacity(combinations.len() * values.len());
        for weights in &combinations {
            for value in values {
                let mut setting = weights.clone();
                setting.insert(id.clone(), *value);
                extended.push(setting);
            }
        }
        combinations = extended;
    }
    combinations
}

/// Runs every setting and returns the results, best objective first.
/// `on_result` is called after each setting, so progress can be shown during long sweeps.
pub fn run_sweep(
    spec: &SweepSpec,
    rules: &[Rule],
    pieces: &[BenchPiece],
    config: &EvalConfig,
    on_result: &mut dyn FnMut(usize, usize, &SweepResult),
) -> Result<Vec<SweepResult>, String> {
    let all = settings(spec);
    let mut results = Vec::with_capacity(all.len());
    for (index, weights) in all.iter().enumerate() {
        let overrides: Weights = weights.iter().map(|(k, v)| (k.clone(), *v)).collect();
        let ruleset = RuleSet::new(rules.to_vec(), &overrides)?;
        let sets = summarize(&evaluate(pieces, &ruleset, config), &spec.sets);
        let objective = objective(&sets, spec)?;
        let result = SweepResult { weights: weights.clone(), objective, sets };
        on_result(index + 1, all.len(), &result);
        results.push(result);
    }
    results.sort_by(|a, b| b.objective.total_cmp(&a.objective));
    Ok(results)
}

fn objective(sets: &[SetRecord], spec: &SweepSpec) -> Result<f64, String> {
    let set = sets.iter().find(|s| s.name == spec.objective_set);
    let set = set.ok_or_else(|| format!("objective set {} has no pieces", spec.objective_set))?;
    if spec.objective_feature == "mean" {
        let augmentednet = &set.rates[..AUGMENTEDNET_FEATURES];
        return Ok(augmentednet.iter().sum::<f64>() / AUGMENTEDNET_FEATURES as f64);
    }
    let index = FEATURES.iter().position(|f| *f == spec.objective_feature);
    let index = index.ok_or_else(|| format!("unknown objective feature {}", spec.objective_feature))?;
    Ok(set.rates[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(one_at_a_time: bool) -> SweepSpec {
        let grid =
            BTreeMap::from([("a".to_string(), vec![1.0, 2.0]), ("b".to_string(), vec![3.0, 4.0, 5.0])]);
        SweepSpec {
            sets: vec![],
            objective_set: "all".into(),
            objective_feature: "rn".into(),
            base: BTreeMap::new(),
            grid,
            one_at_a_time,
        }
    }

    #[test]
    fn grid_tries_every_combination() {
        assert_eq!(settings(&spec(false)).len(), 6);
    }

    #[test]
    fn one_at_a_time_includes_the_base() {
        assert_eq!(settings(&spec(true)).len(), 6);
    }
}
