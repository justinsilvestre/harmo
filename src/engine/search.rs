//! The engine core: a left-to-right beam search in which every slice either extends the current
//! chord segment or opens a new one with a freshly proposed reading.
//!
//! The beam at each slice is the set of interpretations a listener might hold at that moment.
//! Hypotheses that open the same reading at the same slice are merged, keeping the best history.

use super::model::{NONE, Node, Reading, Segment};
use super::rule::{CloseContext, History, SliceContext, TransitionContext};
use super::ruleset::RuleSet;
use crate::score::slicing::Slice;
use crate::theory::key::all_keys;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Config {
    /// Number of hypotheses kept after each slice.
    pub beam_width: usize,
    /// Number of new readings, ranked by their fit to the opening slice, considered per slice.
    pub candidate_limit: usize,
    /// Keeps the beam of every slice, for tracing how readings rise and fall in the ranking.
    pub record_snapshots: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config { beam_width: 48, candidate_limit: 96, record_snapshots: false }
    }
}

pub struct Analysis {
    /// Every hypothesis that survived pruning; nodes point to their parents by index.
    pub arena: Vec<Node>,
    /// Final hypotheses as (arena index, score including the closing of the last segment), best first.
    pub ranking: Vec<(u32, f32)>,
    /// The beam after each slice, best first. Empty unless `Config::record_snapshots` is set.
    pub snapshots: Vec<Vec<u32>>,
}

pub fn analyze(slices: &[Slice], rules: &RuleSet, config: &Config) -> Analysis {
    let mut arena: Vec<Node> = Vec::new();
    let mut beam: Vec<u32> = Vec::new();
    let mut snapshots = Vec::new();
    for index in 0..slices.len() {
        let pending = step(slices, index, &arena, &beam, rules, config);
        beam = commit(&mut arena, prune(pending, config.beam_width));
        if config.record_snapshots {
            snapshots.push(beam.clone());
        }
    }
    let ranking = rank_final(slices, &arena, &beam, rules);
    Analysis { arena, ranking, snapshots }
}

fn step(
    slices: &[Slice],
    index: usize,
    arena: &[Node],
    beam: &[u32],
    rules: &RuleSet,
    config: &Config,
) -> Vec<Node> {
    let slice = &slices[index];
    let next_slice = slices.get(index + 1);
    let mut pending = continuations(slice, next_slice, index, arena, beam, rules);
    let closes: Vec<f32> =
        beam.iter().map(|&b| close_score(slices, &arena[b as usize], Some(slice), rules)).collect();
    for (reading, opening_score) in openings(slice, next_slice, rules, config.candidate_limit) {
        let best = best_predecessor(&reading, slice, arena, beam, &closes, rules);
        if let Some((parent, previous_score)) = best {
            let segment = open_segment(reading, index, parent, slice);
            pending.push(Node {
                parent,
                slice: index as u32,
                segment,
                score: previous_score + opening_score,
            });
        }
    }
    pending
}

fn continuations(
    slice: &Slice,
    next_slice: Option<&Slice>,
    index: usize,
    arena: &[Node],
    beam: &[u32],
    rules: &RuleSet,
) -> Vec<Node> {
    let mut cache: HashMap<Reading, f32> = HashMap::new();
    let mut extend_node = |b: u32| {
        let node = &arena[b as usize];
        let reading = node.segment.reading;
        let context = SliceContext { slice, next_slice, reading: &reading, is_opening: false };
        let fit = *cache.entry(reading).or_insert_with(|| rules.score_slice(&context));
        Node { parent: b, slice: index as u32, segment: extend(node.segment, slice), score: node.score + fit }
    };
    beam.iter().map(|&b| extend_node(b)).collect()
}

/// Proposes new readings for a segment opening on this slice, best-fitting first.
fn openings(slice: &Slice, next_slice: Option<&Slice>, rules: &RuleSet, limit: usize) -> Vec<(Reading, f32)> {
    let mut chords: Vec<_> = rules.proposers.iter().flat_map(|propose| propose(slice)).collect();
    chords.sort();
    chords.dedup();
    let mut readings: Vec<Reading> = Vec::new();
    for &chord in &chords {
        for &key in all_keys() {
            for interpret in &rules.interpreters {
                let interpretations = interpret(key, chord).into_iter();
                readings.extend(interpretations.map(|(tonicized, category)| Reading {
                    key,
                    chord,
                    tonicized,
                    category,
                }));
            }
        }
    }
    readings.sort();
    readings.dedup();
    let score =
        |reading: &Reading| rules.score_slice(&SliceContext { slice, next_slice, reading, is_opening: true });
    let mut scored: Vec<(Reading, f32)> = readings.into_iter().map(|r| (r, score(&r))).collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(limit);
    scored
}

/// Finds the hypothesis that best leads into a new reading, returning its index and the score
/// of the path up to the new segment (closing and transition scores included).
fn best_predecessor(
    reading: &Reading,
    slice: &Slice,
    arena: &[Node],
    beam: &[u32],
    closes: &[f32],
    rules: &RuleSet,
) -> Option<(u32, f32)> {
    let evaluate = |previous: Option<&Segment>, last: u32| {
        let context = TransitionContext { previous, next: reading, slice, history: History { arena, last } };
        rules.admits(&context).then(|| rules.score_transition(&context))
    };
    if beam.is_empty() {
        return evaluate(None, NONE).map(|score| (NONE, score));
    }
    let candidates = beam.iter().zip(closes).filter_map(|(&b, close)| {
        let node = &arena[b as usize];
        evaluate(Some(&node.segment), b).map(|transition| (b, node.score + close + transition))
    });
    candidates.max_by(|a, b| a.1.total_cmp(&b.1))
}

fn open_segment(reading: Reading, index: usize, previous: u32, slice: &Slice) -> Segment {
    let empty =
        Segment { reading, start: index as u32, previous, duration: 0.0, members_heard: 0, bass: None };
    extend(empty, slice)
}

fn extend(mut segment: Segment, slice: &Slice) -> Segment {
    segment.duration += slice.duration;
    for note in &slice.notes {
        let Some(member) = segment.reading.chord.member_index(note.tpc) else {
            continue;
        };
        segment.members_heard |= 1 << member;
        if segment.bass.is_none_or(|(midi, _)| note.midi < midi) {
            segment.bass = Some((note.midi, note.tpc));
        }
    }
    segment
}

fn close_score(slices: &[Slice], node: &Node, next_slice: Option<&Slice>, rules: &RuleSet) -> f32 {
    let span = &slices[node.segment.start as usize..=node.slice as usize];
    rules.score_close(&CloseContext { segment: &node.segment, slices: span, next_slice })
}

/// Keeps the best hypotheses. The sort is stable, so ties are broken by insertion order.
fn prune(mut pending: Vec<Node>, width: usize) -> Vec<Node> {
    pending.sort_by(|a, b| b.score.total_cmp(&a.score));
    pending.truncate(width);
    pending
}

fn commit(arena: &mut Vec<Node>, nodes: Vec<Node>) -> Vec<u32> {
    let first = arena.len() as u32;
    arena.extend(nodes);
    (first..arena.len() as u32).collect()
}

fn rank_final(slices: &[Slice], arena: &[Node], beam: &[u32], rules: &RuleSet) -> Vec<(u32, f32)> {
    let finish = |b: u32| (b, arena[b as usize].score + close_score(slices, &arena[b as usize], None, rules));
    let mut ranking: Vec<(u32, f32)> = beam.iter().map(|&b| finish(b)).collect();
    ranking.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranking
}
