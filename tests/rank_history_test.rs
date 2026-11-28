//! End-to-end tests of rank histories and of the record of why readings leave the beam.

use harmo::engine::history::{
    ChordReading, chord_reading_at, layer_rank_histories, rank_histories, reading_at, segment_at,
};
use harmo::engine::model::{ChordSegment, Function};
use harmo::engine::rank_history::RankChange;
use harmo::engine::ranking::Ranking;
use harmo::engine::ruleset::{RuleSet, Weights};
use harmo::engine::search::{Analysis, Config, analyze};
use harmo::rules::default_rules;
use harmo::score::musicxml::read_score;
use harmo::score::slicing::slice_score;
use std::path::Path;

/// A beam narrow enough that the cadence fixture loses readings to pruning.
fn narrow(record_pruning: bool) -> Config {
    Config { beam_width: 8, record_pruning, ..Config::default() }
}

fn analyze_cadence(config: &Config) -> (Analysis, usize) {
    let slices = slice_score(&read_score(Path::new("tests/fixtures/cadence.musicxml")).unwrap());
    let rules = RuleSet::new(default_rules(), &Weights::new()).unwrap();
    (analyze(&slices, &rules, config), slices.len())
}

/// Every rank change of every reading, for each slice of the piece.
fn all_histories(analysis: &Analysis, slices: usize) -> Vec<Vec<RankChange>> {
    (0..slices).flat_map(|at| rank_histories(analysis, at)).map(|h| h.changes).collect()
}

/// Ranks found by looking up each hypothesis's path, slice by slice.
fn walked_ranks(analysis: &Analysis, at: usize) -> Vec<Vec<Option<usize>>> {
    let histories = rank_histories(analysis, at);
    let rank_of = |beam: Ranking<Function>, h: &harmo::engine::history::ReadingHistory| {
        beam.iter().position(|n| reading_at(analysis, n, at) == Some(h.reading)).map(|r| r + 1)
    };
    let later = || analysis.layers.key.rankings().skip(at);
    histories.iter().map(|h| later().map(|beam| rank_of(beam, h)).collect()).collect()
}

/// Expands a history of rank changes into a rank per slice, from `at` to the end.
fn expanded_ranks(changes: &[RankChange], at: usize, slices: usize) -> Vec<Option<usize>> {
    let rank_at = |slice: usize| changes.iter().rev().find(|c| c.slice <= slice).and_then(|c| c.rank);
    (at..slices).map(rank_at).collect()
}

#[test]
fn rank_histories_match_the_ranks_found_by_walking_each_path() {
    let (analysis, slices) = analyze_cadence(&narrow(false));
    let expanded = |at: usize| -> Vec<Vec<Option<usize>>> {
        rank_histories(&analysis, at).iter().map(|h| expanded_ranks(&h.changes, at, slices)).collect()
    };
    assert!((0..slices).all(|at| expanded(at) == walked_ranks(&analysis, at)));
}

#[test]
fn readings_that_leave_the_beam_say_why() {
    let (analysis, slices) = analyze_cadence(&narrow(true));
    let histories = all_histories(&analysis, slices);
    let ends = histories.iter().filter_map(|h| h.last()).filter(|c| c.rank.is_none());
    assert!(ends.clone().count() > 0 && ends.clone().all(|c| c.left_beam.is_some()));
}

#[test]
fn recording_pruning_leaves_the_ranking_unchanged() {
    let ranking = |config: &Config| analyze_cadence(config).0.ranking;
    assert_eq!(ranking(&narrow(true)), ranking(&narrow(false)));
}

/// Ranks of each key-layer history's function, found by looking up each hypothesis's path, slice by slice.
fn walked_function_ranks(analysis: &Analysis, at: usize) -> Vec<Vec<Option<usize>>> {
    let function_of = |n| segment_at(analysis, n, at).map(|s| s.function);
    let rank_of = |beam: Ranking<Function>, function: Function| {
        beam.iter().position(|n| function_of(n) == Some(function)).map(|r| r + 1)
    };
    let histories = layer_rank_histories(analysis, at).key;
    let later = || analysis.layers.key.rankings().skip(at);
    histories.iter().map(|h| later().map(|beam| rank_of(beam, h.reading)).collect()).collect()
}

/// Ranks of each chord-layer history's reading, found by walking each chord hypothesis's parents
/// back to its ancestor at `at`, whose bass is the one heard by `at`.
fn walked_chord_ranks(analysis: &Analysis, at: usize) -> Vec<Vec<Option<usize>>> {
    let chord = &analysis.layers.chord;
    let first = chord.ranking(at).unwrap();
    let reading_of =
        |n| chord.ancestor_in(n, first).and_then(|ancestor| chord_reading_at(analysis, ancestor, at));
    let rank_of = |ranking: Ranking<ChordSegment>, reading: ChordReading| {
        ranking.iter().position(|n| reading_of(n) == Some(reading)).map(|r| r + 1)
    };
    let histories = layer_rank_histories(analysis, at).chord;
    let later = || analysis.layers.chord.rankings().skip(at);
    histories.iter().map(|h| later().map(|ranking| rank_of(ranking, h.reading)).collect()).collect()
}

#[test]
fn key_layer_histories_match_the_ranks_found_by_walking_each_path() {
    let (analysis, slices) = analyze_cadence(&narrow(false));
    let expanded = |at: usize| -> Vec<Vec<Option<usize>>> {
        let histories = layer_rank_histories(&analysis, at).key;
        histories.iter().map(|h| expanded_ranks(&h.changes, at, slices)).collect()
    };
    assert!((0..slices).all(|at| expanded(at) == walked_function_ranks(&analysis, at)));
}

#[test]
fn chord_layer_histories_match_the_ranks_found_by_walking_each_path() {
    let (analysis, slices) = analyze_cadence(&narrow(false));
    let expanded = |at: usize| -> Vec<Vec<Option<usize>>> {
        let histories = layer_rank_histories(&analysis, at).chord;
        histories.iter().map(|h| expanded_ranks(&h.changes, at, slices)).collect()
    };
    assert!((0..slices).all(|at| expanded(at) == walked_chord_ranks(&analysis, at)));
}
