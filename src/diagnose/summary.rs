//! Diagnoses pieces in parallel and merges the results by benchmark set, ranked by frames lost.

use super::beam_diversity::{BeamTally, tally_beam};
use super::piece_context::PieceContext;
use super::piece_disagreements::tally_disagreements;
use super::piece_errors::tally_errors;
use super::tally::{ErrorTally, KindTally};
use super::tally_class::ClassTally;
use crate::bench::runner::{BenchPiece, EvalConfig};
use crate::engine::ruleset::RuleSet;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;

pub struct PieceDiagnosis {
    pub sets: Vec<String>,
    pub errors: ErrorTally,
    pub disagreements: BTreeMap<String, ClassTally>,
    pub beam: BeamTally,
}

/// A set's merged diagnosis. Kinds, groups and classes are listed by frames, most first.
#[derive(Clone, Debug, Serialize)]
pub struct SetDiagnosis {
    pub name: String,
    pub pieces: usize,
    pub frames: u64,
    pub correct: u64,
    pub groups: Vec<(String, u64)>,
    pub kinds: Vec<(String, KindTally)>,
    pub bars: u64,
    pub expert_changes: u64,
    pub engine_changes: u64,
    pub disagreements: Vec<(String, ClassTally)>,
    pub beam: BeamTally,
}

pub fn diagnose_pieces(pieces: &[BenchPiece], rules: &RuleSet, config: &EvalConfig) -> Vec<PieceDiagnosis> {
    let diagnose = |piece: &BenchPiece| {
        let context = PieceContext::new(piece, rules, config);
        PieceDiagnosis {
            sets: piece.sets.clone(),
            errors: tally_errors(&context),
            disagreements: tally_disagreements(&context, config),
            beam: tally_beam(&context.analysis, config.offer_count),
        }
    };
    pieces.par_iter().map(diagnose).collect()
}

/// Merges the diagnoses of each named set, leaving out sets without pieces.
pub fn summarize(diagnoses: &[PieceDiagnosis], set_names: &[String]) -> Vec<SetDiagnosis> {
    set_names.iter().map(|name| summarize_set(diagnoses, name)).filter(|s| s.pieces > 0).collect()
}

fn summarize_set(diagnoses: &[PieceDiagnosis], name: &str) -> SetDiagnosis {
    let members: Vec<&PieceDiagnosis> =
        diagnoses.iter().filter(|d| d.sets.iter().any(|s| s == name)).collect();
    let mut errors = ErrorTally::default();
    let mut disagreements = BTreeMap::new();
    let mut beam = BeamTally::default();
    for member in &members {
        errors.merge(&member.errors);
        for (class, tally) in &member.disagreements {
            disagreements.entry(class.clone()).or_insert_with(ClassTally::default).merge(tally);
        }
        beam.merge(&member.beam);
    }
    let mut groups: BTreeMap<String, u64> = BTreeMap::new();
    for kind in errors.kinds.values() {
        *groups.entry(kind.group.clone()).or_default() += kind.frames;
    }
    SetDiagnosis {
        name: name.to_string(),
        pieces: members.len(),
        frames: errors.frames,
        correct: errors.correct,
        groups: ranked(groups, |frames| *frames),
        kinds: ranked(errors.kinds, |t| t.frames),
        bars: errors.bars,
        expert_changes: errors.expert_changes,
        engine_changes: errors.engine_changes,
        disagreements: ranked(disagreements, |t| t.frames),
        beam,
    }
}

fn ranked<T>(map: BTreeMap<String, T>, frames: impl Fn(&T) -> u64) -> Vec<(String, T)> {
    let mut entries: Vec<(String, T)> = map.into_iter().collect();
    entries.sort_by_key(|(_, t)| std::cmp::Reverse(frames(t)));
    entries
}
