//! The engine core:
//! a left-to-right beam search in which every slice either extends the current chord segment
//! or opens a new one with a freshly proposed reading.
//!
//! The search keeps two layers. The chord layer holds hearings of the chords (`Hearing`),
//! scored by the chord rules alone. The key layer holds hypotheses (`Hypothesis`),
//! each of which reads one hearing's latest segment in a key and is scored by every rule.
//! Several hypotheses can share one hearing.
//!
//! The beam at each slice is the set of interpretations a listener might hold at that moment.
//! Hypotheses that open the same reading at the same slice are merged, keeping the best history.

use super::continuations::continuations;
use super::exits::{Exits, Predecessor, chord_close, key_close};
use super::layers::Layers;
use super::model::{Hearing, HearingId, Hypothesis};
use super::moment::Moment;
use super::pending::{Pending, PendingHearingId, commit};
use super::proposals::{Opening, Proposals, propose};
use super::pruning::{Pruned, prune};
use super::ruleset::RuleSet;
use crate::score::slicing::Slice;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Config {
    /// Number of hypotheses kept after each slice.
    pub beam_width: usize,
    /// Number of hearings of the chords kept after each slice,
    /// each ranked by the best hypothesis that interprets it.
    /// Zero keeps every hearing that some kept hypothesis interprets.
    pub hearing_width: usize,
    /// Number of new readings, ranked by their fit to the opening slice, considered per slice.
    pub candidate_limit: usize,
    /// Number of recent segments that identify a hypothesis when the beam is pruned.
    /// Of the hypotheses whose last `distinct_depth` segments the rules' distinguish hooks count
    /// as the same, only the best is kept. Zero, or a rule set without distinguish hooks,
    /// keeps them all.
    pub distinct_depth: usize,
    /// Keeps the beam of every slice, for tracing how readings rise and fall in the ranking.
    pub record_snapshots: bool,
    /// Keeps the hypotheses that pruning drops at every slice, with the reason,
    /// for explaining why readings leave the beam.
    pub record_pruning: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            beam_width: 80,
            hearing_width: 40,
            candidate_limit: 48,
            distinct_depth: 0,
            record_snapshots: false,
            record_pruning: false,
        }
    }
}

pub struct Analysis {
    /// Every hypothesis that survived pruning,
    /// and every hearing of the chords that one of them interprets.
    pub layers: Layers,
    /// The final hypotheses, best first, each with its score once its last segment is closed.
    /// Each is given by its index in the hypotheses.
    pub ranking: Vec<(u32, f32)>,
    /// The beam after each slice, best first. Empty unless `Config::record_snapshots` is set.
    pub snapshots: Vec<Vec<u32>>,
    /// The hypotheses dropped at each slice, best first. Empty unless `Config::record_pruning` is set.
    pub pruned: Vec<Vec<Pruned>>,
}

pub fn analyze(slices: &[Slice], rules: &RuleSet, config: &Config) -> Analysis {
    let mut layers = Layers::default();
    let mut beam: Vec<u32> = Vec::new();
    let mut snapshots = Vec::new();
    let mut pruned = Vec::new();
    for index in 0..slices.len() {
        let pending = step(&Moment { slices, index, layers: &layers }, &beam, rules, config);
        let (kept, dropped) = prune(&pending, &layers, rules, config);
        beam = commit(&mut layers, &pending.hearings, kept);
        if config.record_snapshots {
            snapshots.push(beam.clone());
        }
        if config.record_pruning {
            pruned.push(dropped);
        }
    }
    let ranking = rank_final(slices, &layers, &beam, rules);
    Analysis { layers, ranking, snapshots, pruned }
}

/// Proposes the hypotheses of one slice: every hypothesis in the beam extended by the slice,
/// and every new reading of the slice opened after the hypotheses that lead into it best.
fn step(moment: &Moment, beam: &[u32], rules: &RuleSet, config: &Config) -> Pending {
    let proposals = propose(moment, rules, config.candidate_limit);
    let exits = Exits::of(moment, beam, &proposals.chords, rules);
    let mut pending = continuations(moment, beam, rules);
    let mut opened =
        Opened { proposals: &proposals, exits: &exits, slice: moment.index as u32, hearings: HashMap::new() };
    for opening in &proposals.openings {
        for predecessor in exits.predecessors(opening, moment, rules) {
            opened.open(&mut pending, opening, &predecessor);
        }
    }
    pending
}

/// The new segments opened on one slice. Each new segment is heard in a new hearing,
/// shared by every new segment of the same chord after the same hearing.
struct Opened<'a> {
    proposals: &'a Proposals,
    exits: &'a Exits,
    slice: u32,
    /// The new hearing for each hearing left and each proposed chord, by its index.
    hearings: HashMap<(HearingId, usize), PendingHearingId>,
}

impl Opened<'_> {
    /// Adds a hypothesis that opens a new segment with the opening's reading after a predecessor.
    fn open(&mut self, pending: &mut Pending, opening: &Opening, predecessor: &Predecessor) {
        let hearing = self.hearing(pending, opening, predecessor);
        let (parent, function) = (self.exits.keys[predecessor.exit].hypothesis, opening.reading.function);
        let score = predecessor.score + opening.fit.total();
        pending.hypotheses.push(Hypothesis { parent, previous: parent, hearing, function, score });
    }

    /// Returns the hearing in which the opening's chord opens after the predecessor's hearing.
    fn hearing(
        &mut self,
        pending: &mut Pending,
        opening: &Opening,
        predecessor: &Predecessor,
    ) -> PendingHearingId {
        let chord_exit = &self.exits.chords[self.exits.keys[predecessor.exit].chord_exit];
        let segment = self.proposals.chords[opening.chord_index].segment;
        let slice = self.slice;
        *self.hearings.entry((chord_exit.hearing, opening.chord_index)).or_insert_with(|| {
            let score =
                chord_exit.score + chord_exit.close + predecessor.chord_transition + opening.fit.chord;
            pending.add_hearing(Hearing { previous: chord_exit.hearing, slice, segment, score })
        })
    }
}

/// Ranks the final hypotheses by their scores once their last segments are closed, best first.
fn rank_final(slices: &[Slice], layers: &Layers, beam: &[u32], rules: &RuleSet) -> Vec<(u32, f32)> {
    let end = slices.len();
    let finish = |hypothesis: u32| {
        let segment = layers.segment(hypothesis);
        let close = chord_close(slices, &segment.heard, end, rules) + key_close(slices, &segment, end, rules);
        (hypothesis, layers.hypotheses[hypothesis as usize].score + close)
    };
    let mut ranking: Vec<(u32, f32)> = beam.iter().map(|&hypothesis| finish(hypothesis)).collect();
    ranking.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranking
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
