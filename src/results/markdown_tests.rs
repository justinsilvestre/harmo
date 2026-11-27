//! Tests the results document on a small synthetic record.

use super::markdown::render;
use super::record::{ResultsRecord, RunHeader};
use crate::bench::metrics::AgreementSummary;
use crate::bench::offer_delay::DelaySummary;
use crate::bench::report::SetRecord;
use crate::diagnose::summary::SetDiagnosis;
use crate::diagnose::tally::{Example, KindTally};
use crate::diagnose::tally_class::ClassTally;

fn header() -> RunHeader {
    RunHeader {
        commit: "abc1234".into(),
        has_uncommitted_changes: false,
        date: "2026-01-01".into(),
        settings: [("offer_count".to_string(), 4.0)].into_iter().collect(),
        weights: None,
        pieces: 2,
        failures: vec![("k311-2".into(), "no main analysis".into())],
        threads: 1,
        bench_seconds: 1.0,
        diagnose_seconds: 1.0,
        total_seconds: 2.0,
    }
}

fn set_record(name: &str, agreement: Option<AgreementSummary>) -> SetRecord {
    SetRecord {
        name: name.into(),
        pieces: 1,
        frames: 8,
        rates: vec![0.5; 13],
        mean_offered: Some(3.0),
        agreement,
        equivalence_credits: Some([("timing".to_string(), 2)].into_iter().collect()),
        offer_delays: Some(vec![delays(); 3]),
    }
}

fn delays() -> DelaySummary {
    DelaySummary {
        layer: "rn".into(),
        labels: 10,
        median: Some(0.5),
        within_beat: 0.6,
        within_bar: 0.7,
        within_two_bars: 0.8,
        not_offered: 0.1,
    }
}

fn agreement() -> AgreementSummary {
    let rates = vec![0.9; 8];
    AgreementSummary {
        analyst_agreement: rates.clone(),
        target_consensus_rate: rates.clone(),
        consensus_rate: rates.clone(),
        disagreement_coverage: rates.clone(),
        disagreement_top_matches_either: rates,
        mean_offered: 3.0,
    }
}

fn inversion_errors() -> KindTally {
    let example = Example {
        piece: "p".into(),
        measure: "4".into(),
        frames: 2,
        expert: "C: I6".into(),
        engine: "C: I".into(),
    };
    KindTally {
        group: "inversion".into(),
        frames: 2,
        equivalent: 1,
        pairs: [("I6 vs I".to_string(), 2)].into_iter().collect(),
        examples: vec![example],
    }
}

fn diagnosis() -> SetDiagnosis {
    let class = ClassTally { frames: 4, top_matches_either: 2, both_offered: 1, ..ClassTally::default() };
    SetDiagnosis {
        name: "tune".into(),
        pieces: 1,
        frames: 8,
        correct: 6,
        equivalent: 7,
        credits: Default::default(),
        groups: vec![("inversion".into(), 2)],
        kinds: vec![("inversion".into(), inversion_errors())],
        bars: 2,
        expert_changes: 4,
        engine_changes: 3,
        disagreements: vec![("seventh vs triad".into(), class)],
        beam: Default::default(),
        key_dependence: Default::default(),
        monophony: Default::default(),
    }
}

fn document() -> String {
    let accuracy = vec![set_record("anet-test-wir", None), set_record("agreement", Some(agreement()))];
    render(&ResultsRecord { header: header(), accuracy, diagnoses: vec![diagnosis()] })
}

#[test]
fn gives_augmentednet_figures_in_brackets() {
    assert!(document().contains("| WiR | 50.0 (81.8) | 50.0 (69.2) |"));
}

#[test]
fn lists_pieces_that_failed_to_load() {
    assert!(document().contains("`k311-2` (no main analysis)"));
}

#[test]
fn writes_agreement_rows_as_statements() {
    assert!(document().contains("| The two analysts give the same label | 90.0 | 90.0 |"));
}

#[test]
fn ranks_error_kinds_with_their_share_and_pairs() {
    assert!(document().contains("| inversion | 25.0 | 2 | 50.0 | I6 vs I (2) |"));
}

#[test]
fn gives_the_share_of_frames_each_equivalence_credits() {
    assert!(document().contains("| WiR (test) | 50.0 | 50.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 25.0 |"));
}

#[test]
fn heads_the_agreement_table_with_the_equivalents_column() {
    assert!(document().contains("| inversion | root | RN | chord | RN eq |"));
}

#[test]
fn totals_the_disagreement_classes() {
    assert!(document().contains("| all | 4 | 50.0 | 25.0 |"));
}

#[test]
fn gives_the_length_of_wrong_spans_in_quarter_notes() {
    assert!(document().contains("| inversion | p | 4 | 0.2 | C: I6 | C: I |"));
}

#[test]
fn gives_how_soon_readings_are_offered() {
    assert!(document().contains("| agreement | 10 | 0.50 | 60.0 | 70.0 | 80.0 | 10.0 |"));
}
