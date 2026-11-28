//! End-to-end tests that the analysis document offers each span the readings the bench offers.

use harmo::bench::layer_frames::layer_offers_at;
use harmo::bench::layer_offers::ChordName;
use harmo::bench::runner::{EvalConfig, lagged_beam, offered_from};
use harmo::engine::path::best_segments;
use harmo::engine::ruleset::{RuleSet, Weights};
use harmo::engine::search::{Analysis, analyze};
use harmo::output::analysis::{AnalysisOutput, SegmentOutput, Status, build_output};
use harmo::output::offers::JointOfferOutput;
use harmo::pitch::tpc_name;
use harmo::rules::default_rules;
use harmo::score::musicxml::read_score;
use harmo::score::slicing::{Slice, slice_score};
use std::path::Path;

/// The cadence fixture analyzed with default settings, with its document.
struct Analyzed {
    slices: Vec<Slice>,
    analysis: Analysis,
    output: AnalysisOutput,
    config: EvalConfig,
}

fn analyze_cadence() -> Analyzed {
    let score = read_score(Path::new("tests/fixtures/cadence.musicxml")).unwrap();
    let slices = slice_score(&score);
    let rules = RuleSet::new(default_rules(), &Weights::new()).unwrap();
    let config = EvalConfig::default();
    let analysis = analyze(&slices, &rules, &config.engine);
    let output = build_output(&score, &slices, &analysis, &rules, &config);
    Analyzed { slices, analysis, output, config }
}

/// The first slice of each span of the best final hypothesis.
fn span_starts(analyzed: &Analyzed) -> Vec<usize> {
    best_segments(&analyzed.analysis, &analyzed.slices).iter().map(|s| s.slices.0).collect()
}

#[test]
fn each_span_offers_the_joint_readings_the_bench_offers_for_its_first_slice() {
    let analyzed = analyze_cadence();
    let bench = |at: usize| -> Vec<(String, String)> {
        let beam = lagged_beam(&analyzed.analysis, &analyzed.slices, at, analyzed.config.offer_lag).unwrap();
        let offered = offered_from(&analyzed.analysis, beam, at, &analyzed.config);
        offered.iter().map(|l| (l.key.name(), l.figure())).collect()
    };
    let expected: Vec<_> = span_starts(&analyzed).into_iter().map(bench).collect();
    let shown = |joint: &[JointOfferOutput]| -> Vec<(String, String)> {
        joint.iter().map(|o| (o.reading.key.clone(), o.reading.figure.clone())).collect()
    };
    let actual: Vec<_> = analyzed.output.segments.iter().map(|s| shown(&s.offered.joint)).collect();
    assert!(expected.iter().any(|offers| offers.len() > 1) && actual == expected);
}

#[test]
fn each_span_offers_the_keys_the_bench_offers_by_layer_for_its_first_slice() {
    let analyzed = analyze_cadence();
    let bench = |at: usize| -> Vec<String> {
        let offers = layer_offers_at(&analyzed.analysis, &analyzed.slices, at, &analyzed.config);
        offers.keys.iter().map(|k| k.name()).collect()
    };
    let expected: Vec<_> = span_starts(&analyzed).into_iter().map(bench).collect();
    let shown = |s: &SegmentOutput| -> Vec<String> { s.offered.keys.iter().map(|k| k.key.clone()).collect() };
    let actual: Vec<_> = analyzed.output.segments.iter().map(shown).collect();
    assert!(expected.iter().any(|keys| !keys.is_empty()) && actual == expected);
}

#[test]
fn each_span_offers_the_chords_the_bench_offers_by_layer_for_its_first_slice() {
    let analyzed = analyze_cadence();
    let bench = |at: usize| -> Vec<(String, u8)> {
        let offers = layer_offers_at(&analyzed.analysis, &analyzed.slices, at, &analyzed.config);
        let name = |c: &ChordName| (format!("{}{}", tpc_name(c.root), c.quality.label()), c.inversion);
        offers.chords.iter().map(name).collect()
    };
    let expected: Vec<_> = span_starts(&analyzed).into_iter().map(bench).collect();
    let shown = |s: &SegmentOutput| -> Vec<(String, u8)> {
        s.offered.chords.iter().map(|c| (c.chord.clone(), c.inversion)).collect()
    };
    let actual: Vec<_> = analyzed.output.segments.iter().map(shown).collect();
    assert!(expected.iter().any(|chords| !chords.is_empty()) && actual == expected);
}

#[test]
fn every_offered_reading_key_and_chord_has_a_trace() {
    let analyzed = analyze_cadence();
    let offered = analyzed.output.segments.iter().map(|s| &s.offered);
    let traces: Vec<usize> = offered
        .flat_map(|o| {
            let joint = o.joint.iter().map(|j| j.trace.len());
            let keys = o.keys.iter().map(|k| k.holder.trace.len());
            joint.chain(keys).chain(o.chords.iter().map(|c| c.holder.trace.len())).collect::<Vec<_>>()
        })
        .collect();
    assert!(!traces.is_empty() && traces.iter().all(|&count| count > 0));
}

#[test]
fn a_span_is_interpretive_exactly_when_another_reading_is_offered_for_it() {
    let analyzed = analyze_cadence();
    let agrees = |s: &SegmentOutput| {
        let has_rival = s.offered.joint.iter().any(|o| o.reading != s.reading);
        has_rival == (s.status == Status::Interpretive)
    };
    assert!(analyzed.output.segments.iter().all(agrees));
}

#[test]
fn the_best_offered_key_falls_short_of_the_beam_by_nothing() {
    let analyzed = analyze_cadence();
    let first_shortfalls = analyzed.output.segments.iter().filter_map(|s| s.offered.keys.first());
    assert!(first_shortfalls.map(|k| k.holder.shortfall).all(|shortfall| shortfall == 0.0));
}
