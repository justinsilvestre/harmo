use super::*;
use crate::engine::model::Function;
use crate::engine::rule::{Rule, arenas_of};
use crate::pitch::tpc_from_name;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::{Key, Mode};

fn reading(root: &str) -> Reading {
    let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
    let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
    Reading { key, chord, function: Function::DIATONIC }
}

fn segment(root: &str, start: u32, previous: u32) -> Segment {
    Segment {
        reading: reading(root),
        start,
        previous,
        duration: 1.0,
        members_heard: 0,
        bass: Bass::default(),
    }
}

/// Two opening segments with the given roots (hypotheses 0 and 1), and two pending hypotheses that
/// move to G after the first and the second of them respectively, with scores -2 and -1.
struct Fixture {
    layers: Layers,
    pending: Pending,
}

fn fixture(first_root: &str, second_root: &str) -> Fixture {
    let segments =
        [segment(first_root, 0, NONE), segment(second_root, 0, NONE), segment("G", 1, 0), segment("G", 2, 1)];
    let mut layers = arenas_of(&segments);
    let mut hypotheses = layers.hypotheses.split_off(2);
    let hearings = layers.hearings.split_off(2);
    (hypotheses[0].hearing, hypotheses[0].score) = (0, -2.0);
    (hypotheses[1].hearing, hypotheses[1].score) = (1, -1.0);
    Fixture { layers, pending: Pending { hearings, hypotheses } }
}

/// A rule set whose only rule counts segments with equal readings as the same.
fn distinguishing_rules() -> RuleSet {
    let rules = crate::rules::function_distinct_readings::RULES.to_vec();
    RuleSet::new(rules, &Default::default()).unwrap()
}

fn pruned(fixture: Fixture, rules: &RuleSet, config: &Config) -> (Vec<u32>, Vec<PruneCause>) {
    let Fixture { layers, pending } = fixture;
    let (kept, dropped) = prune(&pending, &layers, rules, config);
    let starts = kept.iter().map(|n| pending.hearings[n.hearing as usize].segment.start).collect();
    (starts, dropped.iter().map(|p| p.cause).collect())
}

fn kept_starts(fixture: Fixture, config: &Config) -> Vec<u32> {
    pruned(fixture, &distinguishing_rules(), config).0
}

#[test]
fn prune_keeps_only_the_best_of_hypotheses_with_the_same_recent_readings() {
    assert_eq!(kept_starts(fixture("C", "C"), &merging()), [2]);
}

#[test]
fn prune_keeps_hypotheses_whose_earlier_readings_differ() {
    assert_eq!(kept_starts(fixture("C", "F"), &merging()), [2, 1]);
}

/// The search with merging turned on, at the depth used before round 8.
fn merging() -> Config {
    Config { distinct_depth: 4, ..Config::default() }
}

#[test]
fn prune_keeps_every_boundary_variant_when_distinct_depth_is_zero() {
    let config = Config { distinct_depth: 0, ..Config::default() };
    assert_eq!(kept_starts(fixture("C", "C"), &config), [2, 1]);
}

#[test]
fn prune_keeps_every_boundary_variant_without_distinguish_rules() {
    let rules = RuleSet::new(vec![], &Default::default()).unwrap();
    assert_eq!(pruned(fixture("C", "C"), &rules, &Config::default()).0, [2, 1]);
}

#[test]
fn prune_looks_back_no_further_than_distinct_depth() {
    let config = Config { distinct_depth: 1, ..Config::default() };
    assert_eq!(kept_starts(fixture("C", "F"), &config), [2]);
}

fn dropped_causes(fixture: Fixture, config: &Config) -> Vec<PruneCause> {
    let config = Config { record_pruning: true, ..config.clone() };
    pruned(fixture, &distinguishing_rules(), &config).1
}

#[test]
fn prune_records_the_hypothesis_kept_in_place_of_a_merged_one() {
    assert_eq!(dropped_causes(fixture("C", "C"), &merging()), [PruneCause::Merged { keeper: 2 }]);
}

#[test]
fn prune_records_hypotheses_cut_by_beam_width() {
    let config = Config { beam_width: 1, ..Config::default() };
    assert_eq!(dropped_causes(fixture("C", "F"), &config), [PruneCause::Width]);
}

#[test]
fn prune_keeps_at_most_beam_width_hypotheses() {
    let config = Config { beam_width: 1, ..Config::default() };
    assert_eq!(kept_starts(fixture("C", "F"), &config), [2]);
}

#[test]
fn hearings_that_no_kept_hypothesis_interprets_are_not_committed() {
    let Fixture { mut layers, pending } = fixture("C", "F");
    let kept = vec![pending.hypotheses[1]];
    commit(&mut layers, &pending.hearings, kept);
    let committed = (layers.hearings.len(), layers.hypotheses[2].hearing, layers.hearings[2].segment.start);
    assert_eq!(committed, (3, 2, 2));
}

fn same_group(_: &RivalContext, _: &[f64]) -> Option<u64> {
    Some(0)
}

/// Returns the parents that a new reading on slice 3 opens after, from a beam of two hypotheses
/// whose last segments end on slice 2 with the given roots and scores.
fn parents(rival_hook: bool, roots: [&str; 2], scores: [f32; 2]) -> Vec<u32> {
    let rule = Rule {
        id: "test.rival",
        layer: crate::engine::rule::Layer::Chord,
        summary: "",
        citations: &[],
        default_weight: 1.0,
        params: &[],
        hook: crate::engine::rule::Hook::Rival(same_group),
    };
    let rules = RuleSet::new(if rival_hook { vec![rule] } else { vec![] }, &Default::default()).unwrap();
    let mut layers = arenas_of(&[segment(roots[0], 0, NONE), segment(roots[1], 0, NONE)]);
    for (i, score) in scores.into_iter().enumerate() {
        (layers.hypotheses[i].score, layers.hypotheses[i].slice, layers.hearings[i].slice) = (score, 2, 2);
    }
    let slice = Slice {
        start: 0.0,
        duration: 1.0,
        notes: vec![],
        measure: 0,
        metric_level: 0,
        phrase: 0,
        key_signature: None,
    };
    let slices = vec![slice; 4];
    let beam = [0, 1];
    let exits: Vec<Exit> = beam.iter().map(|&b| exit(&slices, 3, &layers, b, &rules)).collect();
    let opening = Opening { reading: reading("C"), fit: 0.0, chord_fit: 0.0, chord_id: 0 };
    let transitions =
        chord_transitions(&slices, 3, &[opening.reading.chord], &exits, &layers.hearings, &rules);
    let found = predecessors(&opening, &slices, 3, &layers, &beam, &exits, &transitions, &rules);
    found.iter().map(|&(position, _)| beam[position.unwrap()]).collect()
}

#[test]
fn a_new_segment_opens_after_the_best_hypothesis_alone() {
    assert_eq!(parents(false, ["G", "F"], [0.0, -1.0]), [0]);
}

#[test]
fn a_new_segment_also_opens_after_the_best_rival_reading() {
    assert_eq!(parents(true, ["G", "F"], [0.0, -1.0]), [0, 1]);
}

#[test]
fn a_rival_needs_another_reading() {
    assert_eq!(parents(true, ["G", "G"], [0.0, -1.0]), [0]);
}
