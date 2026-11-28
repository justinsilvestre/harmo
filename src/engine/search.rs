//! The engine core:
//! a left-to-right beam search in which every slice either extends the current chord segment
//! or opens a new one with a freshly proposed reading.
//!
//! The search keeps two layers. The chord layer holds chord hypotheses (`ChordHypothesis`),
//! scored by the chord rules alone. The key layer holds key hypotheses (`KeyHypothesis`),
//! each of which reads one chord hypothesis's latest segment in a key
//! and is scored by the key layer's rules alone.
//! Several key hypotheses can share one chord hypothesis. A key hypothesis is a complete
//! interpretation, scored by the sum of its own score and its chord hypothesis's,
//! so the beam holds key hypotheses, and "hypothesis" alone means one of them.
//!
//! The beam at each slice is the set of interpretations a listener might hold at that moment.
//! Hypotheses that open the same reading at the same slice are merged, keeping the best history.

use super::continuations::continuations;
use super::exits::{Exits, Predecessor, chord_close, key_close};
use super::hypothesis::Hypothesis;
use super::layers::Layers;
use super::model::{ChordHypothesisId, KeyHypothesisId};
use super::moment::Moment;
use super::pending::{Pending, PendingChordHypothesisId, commit};
use super::proposals::{Opening, Proposals, propose};
use super::pruning::{Pruned, prune};
use super::rule::Layer;
use super::ruleset::RuleSet;
use crate::score::slicing::Slice;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Config {
    /// Number of hypotheses kept after each slice.
    pub beam_width: usize,
    /// Number of chord hypotheses kept after each slice,
    /// each ranked by the best hypothesis that interprets it.
    /// Zero keeps every chord hypothesis that some kept hypothesis interprets.
    pub chord_hypothesis_width: usize,
    /// Number of new readings, ranked by their fit to the opening slice, considered per slice.
    pub candidate_limit: usize,
    /// Number of recent segments that identify a hypothesis when the beam is pruned.
    /// Of the hypotheses whose last `distinct_depth` segments the rules' distinguish hooks count
    /// as the same, only the best is kept. Zero, or a rule set without distinguish hooks,
    /// keeps them all.
    pub distinct_depth: usize,
    /// Keeps the hypotheses that pruning drops at every slice, with the reason,
    /// for explaining why readings leave the beam.
    pub record_pruning: bool,
}

impl Config {
    /// Returns the most hypotheses of a layer that the beam may hold after each slice:
    /// for the top layer, the beam width. Zero sets no limit.
    pub fn width(&self, layer: Layer) -> usize {
        match layer {
            Layer::Chord => self.chord_hypothesis_width,
            Layer::Key => self.beam_width,
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            beam_width: 80,
            chord_hypothesis_width: 40,
            candidate_limit: 48,
            distinct_depth: 0,
            record_pruning: false,
        }
    }
}

pub struct Analysis {
    /// Every hypothesis that survived pruning,
    /// and every chord hypothesis that one of them interprets.
    /// The key layer's ranking at each slice is the beam after it.
    pub layers: Layers,
    /// The final hypotheses, best first, each with its score once its last segment is closed.
    pub ranking: Vec<(KeyHypothesisId, f32)>,
    /// The hypotheses dropped at each slice, best first. Empty unless `Config::record_pruning` is set.
    pub pruned: Vec<Vec<Pruned>>,
}

pub fn analyze(slices: &[Slice], rules: &RuleSet, config: &Config) -> Analysis {
    let mut layers = Layers::default();
    let mut beam: Vec<KeyHypothesisId> = Vec::new();
    let mut pruned = Vec::new();
    for index in 0..slices.len() {
        let pending = step(&Moment { slices, index, layers: &layers }, &beam, rules, config);
        let (kept, dropped) = prune(&pending, &layers, rules, config);
        beam = commit(&mut layers, &pending.chord, kept);
        if config.record_pruning {
            pruned.push(dropped);
        }
    }
    let ranking = rank_final(slices, &layers, &beam, rules);
    Analysis { layers, ranking, pruned }
}

/// Proposes the hypotheses of one slice: every hypothesis in the beam extended by the slice,
/// and every new reading of the slice opened after the hypotheses that lead into it best.
fn step(moment: &Moment, beam: &[KeyHypothesisId], rules: &RuleSet, config: &Config) -> Pending {
    let proposals = propose(moment, rules, config.candidate_limit);
    let exits = Exits::of(moment, beam, &proposals.chords, rules);
    let mut pending = continuations(moment, beam, rules);
    let mut opened = Opened {
        proposals: &proposals,
        exits: &exits,
        slice: moment.index as u32,
        chord_hypotheses: HashMap::new(),
    };
    for opening in &proposals.openings {
        for predecessor in exits.predecessors(opening, moment, rules) {
            opened.open(&mut pending, opening, &predecessor);
        }
    }
    pending
}

/// The new segments opened on one slice. Each new segment is heard in a new chord hypothesis,
/// shared by every new segment of the same chord after the same chord hypothesis.
struct Opened<'a> {
    proposals: &'a Proposals,
    exits: &'a Exits,
    slice: u32,
    /// The new chord hypothesis for each chord hypothesis left and each proposed chord, by its index.
    chord_hypotheses: HashMap<(ChordHypothesisId, usize), PendingChordHypothesisId>,
}

impl Opened<'_> {
    /// Adds a hypothesis that opens a new segment with the opening's reading after a predecessor.
    fn open(&mut self, pending: &mut Pending, opening: &Opening, predecessor: &Predecessor) {
        let reads = self.chord_hypothesis(pending, opening, predecessor);
        let (parent, unit) = (self.exits.keys[predecessor.exit].hypothesis, opening.reading.function);
        let score = predecessor.score.key + opening.fit.key;
        pending.key.add(Hypothesis { parent, previous: parent, reads, unit, score });
    }

    /// Returns the chord hypothesis in which the opening's chord opens after the predecessor's chord hypothesis.
    fn chord_hypothesis(
        &mut self,
        pending: &mut Pending,
        opening: &Opening,
        predecessor: &Predecessor,
    ) -> PendingChordHypothesisId {
        let chord_exit = &self.exits.chords[self.exits.keys[predecessor.exit].chord_exit];
        let unit = self.proposals.chords[opening.chord_index].segment;
        let reads = self.slice;
        *self.chord_hypotheses.entry((chord_exit.chord_hypothesis, opening.chord_index)).or_insert_with(
            || {
                let score = predecessor.score.chord + opening.fit.chord;
                let parent = chord_exit.chord_hypothesis;
                pending.chord.add(Hypothesis { parent, previous: parent, reads, unit, score })
            },
        )
    }
}

/// Ranks the final hypotheses by their scores once their last segments are closed, best first.
fn rank_final(
    slices: &[Slice],
    layers: &Layers,
    beam: &[KeyHypothesisId],
    rules: &RuleSet,
) -> Vec<(KeyHypothesisId, f32)> {
    let end = slices.len();
    let finish = |hypothesis: KeyHypothesisId| {
        let (segment, key_hypothesis) = (layers.segment(hypothesis), layers.key.get(hypothesis));
        let chord =
            layers.chord.get(key_hypothesis.reads).score + chord_close(slices, &segment.heard, end, rules);
        let key = key_hypothesis.score + key_close(slices, &segment, end, rules);
        (hypothesis, chord + key)
    };
    let mut ranking: Vec<(KeyHypothesisId, f32)> =
        beam.iter().map(|&hypothesis| finish(hypothesis)).collect();
    ranking.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranking
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
