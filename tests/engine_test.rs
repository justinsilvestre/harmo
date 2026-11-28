//! End-to-end tests: MusicXML in, analysis out.

use harmo::engine::path::best_segments;
use harmo::engine::ruleset::{RuleSet, Weights};
use harmo::engine::search::{Config, analyze};
use harmo::engine::trace::trace_segment;
use harmo::rules::default_rules;
use harmo::score::musicxml::read_score;
use harmo::score::slicing::{Slice, slice_score};
use std::path::Path;

fn cadence() -> Vec<Slice> {
    slice_score(&read_score(Path::new("tests/fixtures/cadence.musicxml")).unwrap())
}

fn default_ruleset() -> RuleSet {
    RuleSet::new(default_rules(), &Weights::new()).unwrap()
}

#[test]
fn cadence_is_read_as_i_iv_v7_i_in_c_major() {
    let slices = cadence();
    let analysis = analyze(&slices, &default_ruleset(), &Config::default());
    let readings: Vec<String> = best_segments(&analysis, &slices)
        .iter()
        .map(|s| format!("{}: {}", s.label.key.name(), s.label.figure()))
        .collect();
    assert_eq!(readings, ["C: I", "C: IV", "C: V7", "C: I"]);
}

#[test]
fn rule_trace_adds_up_to_the_hypothesis_score() {
    let slices = cadence();
    let rules = default_ruleset();
    let analysis = analyze(&slices, &rules, &Config::default());
    let total: f32 = best_segments(&analysis, &slices)
        .iter()
        .flat_map(|s| trace_segment(&analysis, &slices, &rules, s))
        .map(|c| c.weighted)
        .sum();
    assert!((total - analysis.ranking[0].1).abs() < 1e-3);
}

#[test]
fn unknown_rule_ids_in_weights_are_rejected() {
    let weights = Weights::from([("chord.no_such_rule".to_string(), 1.0)]);
    assert!(RuleSet::new(default_rules(), &weights).is_err());
}

#[test]
fn the_key_layer_ranks_its_hypotheses_at_every_slice() {
    let slices = cadence();
    let analysis = analyze(&slices, &default_ruleset(), &Config::default());
    assert_eq!(analysis.layers.key.rankings().count(), slices.len());
}
