use super::*;
use crate::engine::model::{ChordSegment, Function, Hearing, HearingId, Hypothesis, NONE, Reading};
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

/// A pending hypothesis reading G in C major, after hypothesis `previous`, that reads pending
/// hearing `hearing`.
fn to_g(previous: u32, hearing: u32, score: f32) -> PendingHypothesis {
    let (function, hearing) = (reading("G").function, PendingHearingId(hearing));
    Hypothesis { parent: previous, previous, hearing, function, score }
}

/// Two opening segments with the given roots (hypotheses 0 and 1), and two pending hypotheses that
/// move to G after the first and the second of them respectively, with scores -2 and -1. Each
/// reads a pending hearing of its own: the first opens on slice 1, the second on slice 2.
struct Fixture {
    layers: Layers,
    pending: Pending,
}

fn fixture(first_root: &str, second_root: &str) -> Fixture {
    let layers = layers_of(&[segment(first_root, 0), segment(second_root, 0)], &[NONE, NONE]);
    let hearing = |previous: u32, start: u32| Hearing {
        previous: HearingId(previous),
        slice: start,
        segment: segment("G", start).heard,
        score: 0.0,
    };
    let hearings = vec![hearing(0, 1), hearing(1, 2)];
    Fixture { layers, pending: Pending { hearings, hypotheses: vec![to_g(0, 0, -2.0), to_g(1, 1, -1.0)] } }
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
    kept.iter().map(|h| pending.hearing(h.hearing).segment.start).collect()
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

/// The fixture with a third pending hypothesis, the worst, that reads the same hearing as the best.
fn sharing() -> Fixture {
    let mut fixture = fixture("C", "F");
    fixture.pending.hypotheses.push(to_g(1, 1, -3.0));
    fixture
}

/// The search with room for one hearing of the chords in the beam.
fn one_hearing() -> Config {
    Config { hearing_width: 1, ..Config::default() }
}

#[test]
fn prune_records_hypotheses_cut_by_the_hearing_limit() {
    assert_eq!(dropped_causes(sharing(), &one_hearing()), [PruneCause::HearingWidth]);
}

#[test]
fn prune_keeps_hypotheses_that_read_a_kept_hearing_beyond_the_hearing_limit() {
    assert_eq!(pruned(sharing(), &distinguishing_rules(), &one_hearing()).0, [-1.0, -3.0]);
}
