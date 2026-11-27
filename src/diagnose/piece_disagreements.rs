//! Classifies the frames where a piece's analysts disagree, and checks why the engine does not
//! offer both of their readings.

use super::disagreement_class::{DisagreementClass, classify_disagreement};
use super::missing_reading::{MissingReason, missing_reason};
use super::piece_context::PieceContext;
use super::piece_errors::describe;
use super::spans::{Span, group_spans};
use super::tally::{Example, keep_longest};
use super::tally_class::ClassTally;
use crate::bench::metrics::is_rn_match;
use crate::bench::runner::{EvalConfig, offered_frames};
use crate::theory::label::Label;
use std::collections::BTreeMap;

/// A disagreement frame: its class, the two analysts' labels and the top reading.
type DisagreementFrame = (DisagreementClass, Label, Label, Option<Label>);

/// How the engine fared on one disagreement frame.
struct Outcome {
    top_matches_either: bool,
    both_offered: bool,
    missing: Vec<MissingReason>,
}

/// Tallies disagreement frames for every pair of analysts, as the benchmark counts them.
pub fn tally_disagreements(context: &PieceContext, config: &EvalConfig) -> BTreeMap<String, ClassTally> {
    let mut tallies: BTreeMap<String, ClassTally> = BTreeMap::new();
    let count = context.piece.analyses.len();
    if count < 2 {
        return tallies;
    }
    let offered = offered_frames(&context.analysis, context.piece, context.top.len(), config);
    let pairs = (0..count).flat_map(|first| (first + 1..count).map(move |second| (first, second)));
    for pair in pairs {
        let scanned = scan_pair(context, pair, &offered, config);
        for (_, (class, ..), outcome) in &scanned {
            add_outcome(tallies.entry(class.name().to_string()).or_default(), outcome);
        }
        let records: Vec<_> = scanned.into_iter().map(|(frame, record, _)| (frame, record)).collect();
        add_examples(context, &records, &mut tallies);
    }
    tallies
}

fn scan_pair(
    context: &PieceContext,
    (first, second): (usize, usize),
    offered: &[Vec<Label>],
    config: &EvalConfig,
) -> Vec<(usize, DisagreementFrame, Outcome)> {
    let analyses = &context.piece.analyses;
    let mut scanned = Vec::new();
    for frame in 0..context.top.len() {
        let (Some(a), Some(b)) = (analyses[first].1[frame], analyses[second].1[frame]) else {
            continue;
        };
        if is_rn_match(&a, &b) {
            continue;
        }
        let top = context.top[frame];
        let is_offered = |label: &Label| offered[frame].iter().any(|o| is_rn_match(o, label));
        let missing = [(first, a), (second, b)].into_iter().filter(|(_, label)| !is_offered(label));
        let outcome = Outcome {
            top_matches_either: top.is_some_and(|t| is_rn_match(&t, &a) || is_rn_match(&t, &b)),
            both_offered: is_offered(&a) && is_offered(&b),
            missing: missing
                .map(|(analyst, label)| missing_reason(context, (analyst, frame), &label, config))
                .collect(),
        };
        scanned.push((frame, (classify_disagreement(&a, &b), a, b, top), outcome));
    }
    scanned
}

fn add_outcome(tally: &mut ClassTally, outcome: &Outcome) {
    tally.frames += 1;
    tally.top_matches_either += outcome.top_matches_either as u64;
    tally.both_offered += outcome.both_offered as u64;
    for reason in &outcome.missing {
        match *reason {
            MissingReason::Unproducible => tally.missing_unproducible += 1,
            MissingReason::OutsideBeam => tally.missing_outside_beam += 1,
            MissingReason::BelowCutoff { rank, gap } => {
                tally.missing_below_cutoff += 1;
                tally.below_cutoff_rank += rank as u64;
                tally.below_cutoff_gap += gap as f64;
            }
        }
    }
}

fn add_examples(
    context: &PieceContext,
    records: &[(usize, DisagreementFrame)],
    tallies: &mut BTreeMap<String, ClassTally>,
) {
    for Span { start, frames, value: (class, a, b, top) } in group_spans(records) {
        let example = Example {
            piece: context.piece.id.clone(),
            measure: context.measure_of(start),
            frames: frames as u64,
            expert: format!("{} | {}", describe(Some(&a)), describe(Some(&b))),
            engine: describe(top.as_ref()),
        };
        keep_longest(&mut tallies.entry(class.name().to_string()).or_default().examples, &[example]);
    }
}
