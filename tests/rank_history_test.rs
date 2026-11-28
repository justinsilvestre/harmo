//! End-to-end tests of rank histories and of the record of why readings leave the beam.

use harmo::engine::history::{RankChange, rank_histories, reading_at};
use harmo::engine::model::Function;
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
