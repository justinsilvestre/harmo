//! Tests the results document on a small synthetic record.

use super::markdown::render;
use super::record::{ResultsRecord, RunHeader};
use crate::bench::metrics::AgreementSummary;
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
        rates: vec![0.5; 11],
        mean_offered: Some(3.0),
        agreement,
    }
}

fn agreement() -> AgreementSummary {
    let rates = vec![0.9; 7];
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
        groups: vec![("inversion".into(), 2)],
        kinds: vec![("inversion".into(), inversion_errors())],
        bars: 2,
        expert_changes: 4,
        engine_changes: 3,
        disagreements: vec![("seventh vs triad".into(), class)],
        beam: Default::default(),
        key_dependence: Default::default(),
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
    assert!(document().contains("| inversion | 25.0 | 2 | I6 vs I (2) |"));
}

#[test]
fn totals_the_disagreement_classes() {
    assert!(document().contains("| all | 4 | 50.0 | 25.0 |"));
}

#[test]
fn gives_the_length_of_wrong_spans_in_quarter_notes() {
    assert!(document().contains("| inversion | p | 4 | 0.2 | C: I6 | C: I |"));
}
