//! Pruning: which of the hypotheses proposed at a slice the beam keeps,
//! and records of those it drops, with the reason.
//!
//! A hypothesis that is kept always yields a continuation at the next slice,
//! so it can only leave the search by being pruned.
//! A reading that the beam holds for some slice therefore leaves it exactly
//! when pruning drops the last hypotheses that hold it.

use super::layers::Layers;
use super::model::{KeyHypothesisId, KeySegment};
use super::pending::{Pending, PendingChordHypothesisId, PendingKeyHypothesis};
use super::rule::Layer;
use super::ruleset::RuleSet;
use super::search::Config;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Why pruning dropped a hypothesis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PruneCause {
    /// Keeping the hypothesis would take a layer past its width (see `Config::width`):
    /// for the top layer, the beam was already full of better hypotheses;
    /// for a lower layer, the hypotheses kept already read as many of its hypotheses as it may hold,
    /// and this hypothesis reads another.
    Width { layer: Layer },
    /// A better hypothesis whose last `Config::distinct_depth` segments the distinguish rules count
    /// as the same was kept instead.
    Merged { keeper: KeyHypothesisId },
}

impl PruneCause {
    pub fn name(self) -> &'static str {
        match self {
            PruneCause::Width { layer: Layer::Key } => "beam_width",
            PruneCause::Width { layer: Layer::Chord } => "chord_hypothesis_width",
            PruneCause::Merged { .. } => "distinct_depth",
        }
    }
}

/// A hypothesis dropped by pruning at the slice it reached.
#[derive(Clone, Copy, Debug)]
pub struct Pruned {
    /// The hypothesis that it extended or opened after.
    pub parent: KeyHypothesisId,
    /// The last hypothesis of the segment before its last segment, or `KeyHypothesisId::NONE`.
    pub previous: KeyHypothesisId,
    /// The score of its complete interpretation.
    pub score: f32,
    /// Its last segment.
    pub segment: KeySegment,
    /// Its rank (1 = best) among all the hypotheses that competed for the beam at that slice.
    pub rank: usize,
    pub cause: PruneCause,
}

/// How a reading left the beam: the best of the hypotheses that held it, as it was pruned.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Departure {
    /// The reading's rank in the beam one slice earlier.
    pub last_rank: usize,
    /// Its best hypothesis's rank among all the hypotheses that competed for the beam.
    pub competing_rank: usize,
    /// How far that hypothesis's score fell below the best competing hypothesis.
    pub score_gap: f32,
    /// `beam_width`, `chord_hypothesis_width` or `distinct_depth`.
    pub cause: &'static str,
    /// For `distinct_depth`, the rank in the new beam of the hypothesis kept in its place.
    pub merged_into_rank: Option<usize>,
    /// That hypothesis.
    #[serde(skip)]
    pub merged_into: Option<KeyHypothesisId>,
}

/// Keeps the best of the pending hypotheses, in order,
/// skipping any that would take the beam past its limit
/// of chord hypotheses and any whose recent segments the rules count
/// as the same as those of a better one (see `Config::distinct_depth`). The sort is stable,
/// so ties are broken by the order in which the hypotheses were proposed.
/// Also returns the dropped hypotheses when `Config::record_pruning` is set.
pub fn prune(
    pending: &Pending,
    layers: &Layers,
    rules: &RuleSet,
    config: &Config,
) -> (Vec<PendingKeyHypothesis>, Vec<Pruned>) {
    let mut ranked: Vec<(f32, PendingKeyHypothesis)> =
        pending.key.hypotheses.iter().map(|h| (pending.score(h), *h)).collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let merges = config.distinct_depth > 0 && !rules.key.distinguish.is_empty();
    let pruner = Pruner { pending, layers, rules, config, merges };
    let mut kept = Kept::new(layers.key.hypotheses.len() as u32, config);
    let mut dropped = Vec::new();
    for (rank, (score, hypothesis)) in ranked.into_iter().enumerate() {
        match pruner.judge(&kept, &hypothesis) {
            Ok(signature) => kept.add(hypothesis, signature),
            Err(PruneCause::Width { layer: Layer::TOP }) if !config.record_pruning => break,
            Err(cause) if config.record_pruning => {
                dropped.push(pruner.record(&hypothesis, score, rank + 1, cause))
            }
            Err(_) => {}
        }
    }
    (kept.key_hypotheses, dropped)
}

/// What pruning judges each hypothesis by.
struct Pruner<'a> {
    pending: &'a Pending,
    layers: &'a Layers,
    rules: &'a RuleSet,
    config: &'a Config,
    /// Whether hypotheses whose recent segments the rules count as the same are merged.
    merges: bool,
}

/// The hypotheses kept so far, best first, with what later ones are compared against.
struct Kept {
    key_hypotheses: Vec<PendingKeyHypothesis>,
    /// The chord hypotheses that the kept hypotheses read.
    chord_hypotheses: Held<PendingChordHypothesisId>,
    /// The id that each kept signature's hypothesis will have once stored.
    signatures: HashMap<Vec<u64>, KeyHypothesisId>,
    /// The id that the first kept hypothesis will have once stored.
    first: u32,
}

impl Pruner<'_> {
    /// Returns why the hypothesis cannot join the beam, or else its signature if hypotheses are merged.
    fn judge(&self, kept: &Kept, hypothesis: &PendingKeyHypothesis) -> Result<Option<Vec<u64>>, PruneCause> {
        if kept.key_hypotheses.len() == self.config.width(Layer::Key) {
            return Err(PruneCause::Width { layer: Layer::Key });
        }
        if !kept.chord_hypotheses.admits(hypothesis.reads) {
            return Err(PruneCause::Width { layer: Layer::Chord });
        }
        if !self.merges {
            return Ok(None);
        }
        let signature = self.signature(hypothesis);
        match kept.signatures.get(&signature) {
            Some(&keeper) => Err(PruneCause::Merged { keeper }),
            None => Ok(Some(signature)),
        }
    }

    /// Returns the distinguish hooks' values for a hypothesis's last segment
    /// and for up to `distinct_depth - 1` segments before it, most recent first.
    fn signature(&self, hypothesis: &PendingKeyHypothesis) -> Vec<u64> {
        let (layers, key_rules) = (self.layers, &self.rules.key);
        let depth = self.config.distinct_depth;
        let mut values = Vec::with_capacity(depth * key_rules.distinguish.len());
        key_rules.distinguish(&self.pending.segment(hypothesis), &mut values);
        for earlier in layers.key.path_back(hypothesis.previous).take(depth - 1) {
            key_rules.distinguish(&layers.segment(earlier), &mut values);
        }
        values
    }

    /// Records a dropped hypothesis, given the score of its complete interpretation.
    fn record(
        &self,
        hypothesis: &PendingKeyHypothesis,
        score: f32,
        rank: usize,
        cause: PruneCause,
    ) -> Pruned {
        let (parent, previous) = (hypothesis.parent, hypothesis.previous);
        Pruned { parent, previous, score, segment: self.pending.segment(hypothesis), rank, cause }
    }
}

impl Kept {
    fn new(first: u32, config: &Config) -> Kept {
        let key_hypotheses = Vec::with_capacity(config.width(Layer::Key));
        let chord_hypotheses = Held::new(config.width(Layer::Chord));
        Kept { key_hypotheses, chord_hypotheses, signatures: HashMap::new(), first }
    }

    fn add(&mut self, hypothesis: PendingKeyHypothesis, signature: Option<Vec<u64>>) {
        if let Some(signature) = signature {
            self.signatures
                .insert(signature, KeyHypothesisId::new(self.first + self.key_hypotheses.len() as u32));
        }
        self.chord_hypotheses.add(hypothesis.reads);
        self.key_hypotheses.push(hypothesis);
    }
}

/// The hypotheses of a lower layer that the kept hypotheses read, and how many the layer may hold.
struct Held<Id> {
    ids: HashSet<Id>,
    /// Zero sets no limit.
    width: usize,
}

impl<Id: Copy + Eq + Hash> Held<Id> {
    fn new(width: usize) -> Held<Id> {
        Held { ids: HashSet::new(), width }
    }

    /// Reports whether a kept hypothesis may read the hypothesis without exceeding the layer's width.
    fn admits(&self, id: Id) -> bool {
        self.width == 0 || self.ids.len() < self.width || self.ids.contains(&id)
    }

    fn add(&mut self, id: Id) {
        self.ids.insert(id);
    }
}

#[cfg(test)]
#[path = "pruning_tests.rs"]
mod tests;
