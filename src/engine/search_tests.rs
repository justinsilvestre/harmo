use super::*;
use crate::engine::key_hooks::KeyHook;
use crate::engine::model::Function;
use crate::engine::rule::Rule;
use crate::engine::test_layers::{key_segment, layers_of};
use crate::pitch::tpc_from_name;
use crate::score::slicing::SliceNote;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::{Key, Mode};

fn reading(root: &str) -> Reading {
    let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
    let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
    Reading { chord, function: Function::diatonic(key) }
}

fn segment(root: &str, start: u32) -> KeySegment {
    let segment = key_segment(reading(root));
    KeySegment { heard: ChordSegment { start, ..segment.heard }, ..segment }
}

/// Two opening segments with the given roots (hypotheses 0 and 1), and two pending hypotheses that
/// move to G after the first and the second of them respectively, with scores -2 and -1.
struct Fixture {
    layers: Layers,
    pending: Pending,
}

fn fixture(first_root: &str, second_root: &str) -> Fixture {
    let segments = [segment(first_root, 0), segment(second_root, 0), segment("G", 1), segment("G", 2)];
    let mut layers = layers_of(&segments, &[NONE, NONE, 0, 1]);
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
/// whose last segments end on slice 2 with the given readings and scores.
fn parents(rival_hook: bool, readings: [Reading; 2], scores: [f32; 2]) -> Vec<u32> {
    let rule = Rule {
        id: "test.rival",
        layer: crate::engine::rule::Layer::Chord,
        summary: "",
        citations: &[],
        default_weight: 1.0,
        params: &[],
        hook: crate::engine::rule::Hook::Key(KeyHook::Rival(same_group)),
    };
    let rules = RuleSet::new(if rival_hook { vec![rule] } else { vec![] }, &Default::default()).unwrap();
    let mut layers = layers_of(&readings.map(key_segment), &[NONE, NONE]);
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
    let moment = Moment { slices: &slices, index: 3, layers: &layers };
    let opening = Opening { reading: reading("C"), fit: LayerScores::default(), chord_id: 0 };
    let exit_from = |b: u32| {
        let hearing = layers.hypotheses[b as usize].hearing;
        exit(&moment, b, chord_transitions(&moment, &[opening.reading.chord], hearing, &rules), &rules)
    };
    let exits: Vec<Exit> = [0, 1].into_iter().map(exit_from).collect();
    let found = predecessors(&opening, &moment, &exits, &rules);
    found.iter().map(|p| exits[p.exit.unwrap()].hypothesis).collect()
}

#[test]
fn a_new_segment_opens_after_the_best_hypothesis_alone() {
    assert_eq!(parents(false, [reading("G"), reading("F")], [0.0, -1.0]), [0]);
}

#[test]
fn a_new_segment_also_opens_after_the_best_rival_reading() {
    assert_eq!(parents(true, [reading("G"), reading("F")], [0.0, -1.0]), [0, 1]);
}

#[test]
fn a_rival_needs_another_reading() {
    assert_eq!(parents(true, [reading("G"), reading("G")], [0.0, -1.0]), [0]);
}

#[test]
fn the_same_chord_in_another_key_is_no_rival() {
    let in_g = Reading {
        function: Function::diatonic(Key::new(tpc_from_name("G").unwrap(), Mode::Major)),
        ..reading("G")
    };
    assert_eq!(parents(true, [reading("G"), in_g], [0.0, -1.0]), [0]);
}

/// Analyzes I–V–I in C major, with the given rule weights.
fn i_v_i(overrides: &[(&str, f32)]) -> Analysis {
    let note = |name: &str, midi: i32| SliceNote {
        tpc: tpc_from_name(name).unwrap(),
        midi,
        attacked: true,
        onset_level: 4,
    };
    let slice = |start: f64, names: [(&str, i32); 3]| Slice {
        start,
        duration: 1.0,
        notes: names.into_iter().map(|(name, midi)| note(name, midi)).collect(),
        measure: start as usize,
        metric_level: 4,
        phrase: 0,
        key_signature: None,
    };
    let (tonic, dominant) = ([("C", 48), ("E", 64), ("G", 67)], [("G", 43), ("B", 62), ("D", 67)]);
    let slices = [slice(0.0, tonic), slice(1.0, dominant), slice(2.0, tonic)];
    let weights = overrides.iter().map(|(id, weight)| (id.to_string(), *weight)).collect();
    let rules = RuleSet::new(crate::rules::default_rules(), &weights).unwrap();
    analyze(&slices, &rules, &Config::default())
}

/// The ids of the default rules of the key layer that score, so that they can be switched off.
fn key_scoring_rules() -> Vec<(&'static str, f32)> {
    let scores = |rule: &&Rule| {
        matches!(rule.hook.kind(), "key_slice" | "key_transition" | "key_departure" | "key_close")
    };
    crate::rules::default_rules().iter().filter(scores).map(|rule| (rule.id, 0.0)).collect()
}

#[test]
fn without_key_scores_a_hypothesis_scores_exactly_its_hearing() {
    let layers = i_v_i(&key_scoring_rules()).layers;
    let differs = |h: &Hypothesis| h.score != layers.hearings[h.hearing as usize].score;
    assert!(!layers.hypotheses.iter().any(differs));
}
