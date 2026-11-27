//! The engine core: a left-to-right beam search in which every slice either extends the current
//! chord segment or opens a new one with a freshly proposed reading.
//!
//! The beam at each slice is the set of interpretations a listener might hold at that moment.
//! Hypotheses that open the same reading at the same slice are merged, keeping the best history.
//! Hypotheses that differ only in where their recent segments begin compete for one place in the beam,
//! so that the beam holds more distinct interpretations.

use super::bass::{Bass, BassNote, is_downbeat};
use super::model::{NONE, Node, Reading, Segment};
use super::rule::{CloseContext, History, SliceContext, TransitionContext};
use super::ruleset::RuleSet;
use crate::score::slicing::Slice;
use crate::theory::key::{Key, keys_for_spellings};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Config {
    /// Number of hypotheses kept after each slice.
    pub beam_width: usize,
    /// Number of new readings, ranked by their fit to the opening slice, considered per slice.
    pub candidate_limit: usize,
    /// Number of recent segments whose readings identify a hypothesis when the beam is pruned.
    /// Of the hypotheses that give their last `distinct_depth` segments the same readings, differing
    /// only in where those segments begin, only the best is kept. Zero keeps them all.
    pub distinct_depth: usize,
    /// Keeps the beam of every slice, for tracing how readings rise and fall in the ranking.
    pub record_snapshots: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config { beam_width: 48, candidate_limit: 96, distinct_depth: 3, record_snapshots: false }
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
    let keys = keys_for_slices(slices);
    for index in 0..slices.len() {
        let pending = step(&keys, slices, index, &arena, &beam, rules, config);
        let kept = prune(pending, &arena, config);
        beam = commit(&mut arena, kept);
        if config.record_snapshots {
            snapshots.push(beam.clone());
        }
    }
    let ranking = rank_final(slices, &arena, &beam, rules);
    Analysis { arena, ranking, snapshots }
}

/// Returns the keys to consider for a score, given the pitches its slices spell.
fn keys_for_slices(slices: &[Slice]) -> Vec<Key> {
    let mut spelled: Vec<_> = slices.iter().flat_map(|s| s.notes.iter().map(|n| n.tpc)).collect();
    spelled.sort();
    spelled.dedup();
    keys_for_spellings(&spelled)
}

fn step(
    keys: &[Key],
    slices: &[Slice],
    index: usize,
    arena: &[Node],
    beam: &[u32],
    rules: &RuleSet,
    config: &Config,
) -> Vec<Node> {
    let slice = &slices[index];
    let mut pending = continuations(slices, index, arena, beam, rules);
    let closes: Vec<f32> =
        beam.iter().map(|&b| close_score(slices, &arena[b as usize], Some(slice), rules)).collect();
    for (reading, opening_score) in openings(keys, slices, index, rules, config.candidate_limit) {
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

fn continuations(slices: &[Slice], index: usize, arena: &[Node], beam: &[u32], rules: &RuleSet) -> Vec<Node> {
    let (slice, next_slice) = (&slices[index], slices.get(index + 1));
    let mut cache: HashMap<Reading, f32> = HashMap::new();
    let mut extend_node = |b: u32| {
        let node = &arena[b as usize];
        let reading = node.segment.reading;
        let context = SliceContext { slice, next_slice, slices, index, reading: &reading, is_opening: false };
        let fit = *cache.entry(reading).or_insert_with(|| rules.score_slice(&context));
        Node { parent: b, slice: index as u32, segment: extend(node.segment, slice), score: node.score + fit }
    };
    beam.iter().map(|&b| extend_node(b)).collect()
}

/// Proposes new readings for a segment opening on this slice, best-fitting first.
fn openings(
    keys: &[Key],
    slices: &[Slice],
    index: usize,
    rules: &RuleSet,
    limit: usize,
) -> Vec<(Reading, f32)> {
    let (slice, next_slice) = (&slices[index], slices.get(index + 1));
    let mut chords: Vec<_> = rules.proposers.iter().flat_map(|propose| propose(slice)).collect();
    chords.sort();
    chords.dedup();
    let mut readings: Vec<Reading> = Vec::new();
    for &chord in &chords {
        for &key in keys {
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
    let score = |reading: &Reading| {
        rules.score_slice(&SliceContext { slice, next_slice, slices, index, reading, is_opening: true })
    };
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
    let empty = Segment {
        reading,
        start: index as u32,
        previous,
        duration: 0.0,
        members_heard: 0,
        bass: Bass::default(),
    };
    extend(empty, slice)
}

fn extend(mut segment: Segment, slice: &Slice) -> Segment {
    let lowest = slice.bass().map(|n| n.midi);
    segment.duration += slice.duration;
    for note in &slice.notes {
        let Some(member) = segment.reading.chord.member_index(note.tpc) else {
            continue;
        };
        segment.members_heard |= 1 << member;
        let is_downbeat_bass = is_downbeat(slice.metric_level) && lowest == Some(note.midi);
        segment.bass =
            segment.bass.hear(&BassNote { midi: note.midi, tpc: note.tpc, member, is_downbeat_bass });
    }
    segment
}

fn close_score(slices: &[Slice], node: &Node, next_slice: Option<&Slice>, rules: &RuleSet) -> f32 {
    let span = &slices[node.segment.start as usize..=node.slice as usize];
    rules.score_close(&CloseContext { segment: &node.segment, slices: span, next_slice })
}

/// Keeps the best hypotheses, skipping any whose recent readings repeat those of a better one
/// (see `Config::distinct_depth`). The sort is stable, so ties are broken by insertion order.
fn prune(mut pending: Vec<Node>, arena: &[Node], config: &Config) -> Vec<Node> {
    pending.sort_by(|a, b| b.score.total_cmp(&a.score));
    if config.distinct_depth == 0 {
        pending.truncate(config.beam_width);
        return pending;
    }
    let mut seen: HashSet<Vec<Reading>> = HashSet::new();
    let mut kept = Vec::with_capacity(config.beam_width);
    for node in pending {
        if kept.len() == config.beam_width {
            break;
        }
        if seen.insert(recent_readings(&node.segment, arena, config.distinct_depth)) {
            kept.push(node);
        }
    }
    kept
}

/// Returns the readings of a segment and of up to `depth - 1` segments before it, most recent first.
fn recent_readings(segment: &Segment, arena: &[Node], depth: usize) -> Vec<Reading> {
    let mut readings = Vec::with_capacity(depth);
    let mut current = segment;
    loop {
        readings.push(current.reading);
        if readings.len() == depth || current.previous == NONE {
            return readings;
        }
        current = &arena[current.previous as usize].segment;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::Category;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    fn reading(root: &str) -> Reading {
        let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        Reading { key, chord, tonicized: None, category: Category::Diatonic }
    }

    fn node(root: &str, start: u32, previous: u32, score: f32) -> Node {
        let segment = Segment {
            reading: reading(root),
            start,
            previous,
            duration: 1.0,
            members_heard: 0,
            bass: Bass::default(),
        };
        Node { parent: previous, slice: 2, segment, score }
    }

    /// Two opening segments with the given roots: one ending on slice 0, the other on slice 1.
    fn arena(first_root: &str, second_root: &str) -> Vec<Node> {
        vec![node(first_root, 0, NONE, 0.0), node(second_root, 0, NONE, 0.0)]
    }

    /// Two hypotheses that move to G after the first and the second arena segment respectively.
    fn pending() -> Vec<Node> {
        vec![node("G", 1, 0, -2.0), node("G", 2, 1, -1.0)]
    }

    fn kept_starts(arena: &[Node], config: &Config) -> Vec<u32> {
        prune(pending(), arena, config).iter().map(|n| n.segment.start).collect()
    }

    #[test]
    fn prune_keeps_only_the_best_of_hypotheses_with_the_same_recent_readings() {
        assert_eq!(kept_starts(&arena("C", "C"), &Config::default()), [2]);
    }

    #[test]
    fn prune_keeps_hypotheses_whose_earlier_readings_differ() {
        assert_eq!(kept_starts(&arena("C", "F"), &Config::default()), [2, 1]);
    }

    #[test]
    fn prune_keeps_every_boundary_variant_when_distinct_depth_is_zero() {
        let config = Config { distinct_depth: 0, ..Config::default() };
        assert_eq!(kept_starts(&arena("C", "C"), &config), [2, 1]);
    }

    #[test]
    fn prune_looks_back_no_further_than_distinct_depth() {
        let config = Config { distinct_depth: 1, ..Config::default() };
        assert_eq!(kept_starts(&arena("C", "F"), &config), [2]);
    }

    #[test]
    fn prune_keeps_at_most_beam_width_hypotheses() {
        let config = Config { beam_width: 1, ..Config::default() };
        assert_eq!(kept_starts(&arena("C", "F"), &config), [2]);
    }
}
