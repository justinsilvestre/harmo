use super::*;
use crate::bench::equivalence::{ExpertEquivalents, FrameSounds};
use crate::bench::runner::now_frames;
use crate::engine::ruleset::{RuleSet, Weights};
use crate::engine::search::{Config, analyze};
use crate::pitch::tpc_from_name;
use crate::rules::default_rules;
use crate::score::musicxml::read_score;
use crate::score::slicing::slice_score;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::Key;
use std::path::Path;

fn piece(labels: impl Fn(&Analysis, &BenchPiece) -> Vec<Option<Label>>) -> (Analysis, BenchPiece) {
    let score = read_score(Path::new("tests/fixtures/cadence.musicxml")).unwrap();
    let slices = slice_score(&score);
    let frames = (score.end() / FRAME).ceil() as usize;
    let sounds = FrameSounds::new(&slices, &frame_slices(&slices, frames));
    let rules = RuleSet::new(default_rules(), &Weights::new()).unwrap();
    let analysis = analyze(&slices, &rules, &Config::default());
    let mut piece = BenchPiece {
        id: "cadence".into(),
        sets: vec![],
        slices,
        measures: score.measures,
        analyses: vec![("main".into(), vec![None; frames])],
        sounds,
        equivalents: vec![],
    };
    let labels = labels(&analysis, &piece);
    piece.equivalents = vec![ExpertEquivalents::new(&labels, &piece.sounds)];
    piece.analyses[0].1 = labels;
    (analysis, piece)
}

fn rn_summary(analysis: &Analysis, piece: &BenchPiece) -> DelaySummary {
    offer_delays(analysis, piece, &EvalConfig::default())[0].summary("rn")
}

#[test]
fn readings_the_beam_holds_at_once_are_offered_within_a_beat() {
    let lag_zero = EvalConfig { offer_lag: 0.0, ..EvalConfig::default() };
    let (analysis, piece) = piece(|a, p| now_frames(a, p, p.analyses[0].1.len(), &lag_zero));
    assert_eq!(rn_summary(&analysis, &piece).within_beat, 1.0);
}

#[test]
fn a_reading_the_music_never_suggests_is_not_offered() {
    let remote = Label::new(
        Key::parse("F#", "major").unwrap(),
        None,
        Chord::new(tpc_from_name("C#").unwrap(), Quality::Major),
        tpc_from_name("C#").unwrap(),
    );
    let (analysis, piece) = piece(|_, p| vec![Some(remote); p.analyses[0].1.len()]);
    assert_eq!(rn_summary(&analysis, &piece).not_offered, 1.0);
}

#[test]
fn delays_count_toward_every_threshold_they_meet() {
    let mut delays = LayerDelays::default();
    delays.add(Some(0.0), 1.0, 4.0);
    delays.add(Some(3.0), 1.0, 4.0);
    delays.add(None, 1.0, 4.0);
    let summary = delays.summary("rn");
    assert_eq!((summary.within_beat, summary.within_bar), (1.0 / 3.0, 2.0 / 3.0));
}

#[test]
fn median_is_taken_over_offered_labels_only() {
    let mut delays = LayerDelays::default();
    delays.add(Some(0.0), 1.0, 4.0);
    delays.add(Some(2.0), 1.0, 4.0);
    delays.add(None, 1.0, 4.0);
    assert_eq!(delays.summary("rn").median, Some(1.0));
}
