use super::*;
use crate::engine::hypothesis::Hypothesis;
use crate::engine::model::{
    ChordHypothesis, ChordHypothesisId, ChordSegment, Function, KeyHypothesisId, Reading,
};
use crate::engine::pending::{PendingChordHypothesisId, Proposed};
use crate::engine::test_layers::{key_segment, layers_of};
use crate::pitch::tpc_from_name;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::{Key, Mode};

fn reading(root: &str) -> Reading {
    let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
    Reading {
        chord: Chord::new(tpc_from_name(root).unwrap(), Quality::Major),
        function: Function::diatonic(key),
    }
}

fn segment(root: &str, start: u32) -> KeySegment {
    let segment = key_segment(reading(root));
    KeySegment { heard: ChordSegment { start, ..segment.heard }, ..segment }
}

/// A pending hypothesis reading G in C major, after hypothesis `previous`,
/// that reads pending chord hypothesis `chord_hypothesis`.
fn to_g(previous: u32, chord_hypothesis: u32, score: f32) -> PendingKeyHypothesis {
    let (unit, reads) = (reading("G").function, PendingChordHypothesisId::new(chord_hypothesis));
    let previous = KeyHypothesisId::new(previous);
    Hypothesis { parent: previous, previous, reads, unit, score }
}

/// Two opening segments with the given roots (hypotheses 0 and 1),
/// and two pending hypotheses that move to G after the first and the second of them respectively,
/// with scores -2 and -1. Each reads a pending chord hypothesis of its own: the first opens on slice 1,
/// the second on slice 2.
struct Fixture {
    layers: Layers,
    pending: Pending,
}

fn fixture(first_root: &str, second_root: &str) -> Fixture {
    let layers = layers_of(&[segment(first_root, 0), segment(second_root, 0)], &[KeyHypothesisId::NONE; 2]);
    let chord_hypothesis = |previous: u32, start: u32| ChordHypothesis {
        parent: ChordHypothesisId::new(previous),
        previous: ChordHypothesisId::new(previous),
        reads: start,
        unit: segment("G", start).heard,
        score: 0.0,
    };
    let chord = Proposed { hypotheses: vec![chord_hypothesis(0, 1), chord_hypothesis(1, 2)] };
    let key = Proposed { hypotheses: vec![to_g(0, 0, -2.0), to_g(1, 1, -1.0)] };
    Fixture { layers, pending: Pending { chord, key } }
}

/// A rule set whose only rule counts segments with equal readings as the same.
fn distinguishing_rules() -> RuleSet {
    let rules = crate::rules::function_distinct_readings::RULES.to_vec();
    RuleSet::new(rules, &Default::default()).unwrap()
}

/// Returns the scores of the hypotheses that pruning keeps, and the causes of those it drops.
fn pruned(fixture: Fixture, rules: &RuleSet, config: &Config) -> (Vec<f32>, Vec<PruneCause>) {
    let Fixture { layers, pending } = fixture;
    let (kept, dropped) = prune(&pending, &layers, rules, config);
    (kept.iter().map(|h| h.score).collect(), dropped.iter().map(|p| p.cause).collect())
}

/// Returns the start of the last segment of each hypothesis that pruning keeps.
fn kept_starts(fixture: Fixture, config: &Config) -> Vec<u32> {
    let Fixture { layers, pending } = fixture;
    let (kept, _) = prune(&pending, &layers, &distinguishing_rules(), config);
    kept.iter().map(|h| pending.chord.get(h.reads).unit.start).collect()
}

/// The search with merging turned on, at the depth used before round 8.
fn merging() -> Config {
    Config { distinct_depth: 4, ..Config::default() }
}

fn dropped_causes(fixture: Fixture, config: &Config) -> Vec<PruneCause> {
    let config = Config { record_pruning: true, ..config.clone() };
    pruned(fixture, &distinguishing_rules(), &config).1
}

#[test]
fn prune_keeps_only_the_best_of_hypotheses_with_the_same_recent_readings() {
    assert_eq!(kept_starts(fixture("C", "C"), &merging()), [2]);
}

#[test]
fn prune_keeps_hypotheses_whose_earlier_readings_differ() {
    assert_eq!(kept_starts(fixture("C", "F"), &merging()), [2, 1]);
}

#[test]
fn prune_keeps_every_boundary_variant_when_distinct_depth_is_zero() {
    let config = Config { distinct_depth: 0, ..Config::default() };
    assert_eq!(kept_starts(fixture("C", "C"), &config), [2, 1]);
}

#[test]
fn prune_keeps_every_boundary_variant_without_distinguish_rules() {
    let rules = RuleSet::new(vec![], &Default::default()).unwrap();
    assert_eq!(pruned(fixture("C", "C"), &rules, &merging()).0, [-1.0, -2.0]);
}

#[test]
fn prune_looks_back_no_further_than_distinct_depth() {
    let config = Config { distinct_depth: 1, ..Config::default() };
    assert_eq!(kept_starts(fixture("C", "F"), &config), [2]);
}

#[test]
fn prune_records_the_hypothesis_kept_in_place_of_a_merged_one() {
    assert_eq!(
        dropped_causes(fixture("C", "C"), &merging()),
        [PruneCause::Merged { keeper: KeyHypothesisId::new(2) }]
    );
}

#[test]
fn prune_records_hypotheses_cut_by_beam_width() {
    let config = Config { beam_width: 1, ..Config::default() };
    assert_eq!(dropped_causes(fixture("C", "F"), &config), [PruneCause::Width { layer: Layer::Key }]);
}

#[test]
fn prune_keeps_at_most_beam_width_hypotheses() {
    let config = Config { beam_width: 1, ..Config::default() };
    assert_eq!(kept_starts(fixture("C", "F"), &config), [2]);
}

/// The fixture with a third pending hypothesis, the worst, that reads the same chord hypothesis as the best.
fn sharing() -> Fixture {
    let mut fixture = fixture("C", "F");
    fixture.pending.key.hypotheses.push(to_g(1, 1, -3.0));
    fixture
}

/// The search with room for one chord hypothesis in the beam.
fn one_chord_hypothesis() -> Config {
    Config { chord_hypothesis_width: 1, ..Config::default() }
}

#[test]
fn prune_records_hypotheses_cut_by_the_chord_hypothesis_limit() {
    assert_eq!(
        dropped_causes(sharing(), &one_chord_hypothesis()),
        [PruneCause::Width { layer: Layer::Chord }]
    );
}

#[test]
fn prune_keeps_hypotheses_that_read_a_kept_chord_hypothesis_beyond_the_chord_hypothesis_limit() {
    assert_eq!(pruned(sharing(), &distinguishing_rules(), &one_chord_hypothesis()).0, [-1.0, -3.0]);
}
