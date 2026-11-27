//! The engine core: a left-to-right beam search in which every slice either extends the current
//! chord segment or opens a new one with a freshly proposed reading.
//!
//! The beam at each slice is the set of interpretations a listener might hold at that moment.
//! Hypotheses that open the same reading at the same slice are merged, keeping the best history.
//! Hypotheses whose recent segments the rules count as the same interpretation compete for one place
//! in the beam, so that the beam holds more distinct interpretations.

use super::bass::{Bass, BassNote};
use super::model::{NONE, Node, Reading, Segment};
use super::pruning::{PruneCause, Pruned};
use super::rule::{CloseContext, DepartureContext, History, RivalContext, SliceContext, TransitionContext};
use super::ruleset::RuleSet;
use crate::score::slicing::Slice;
use crate::theory::key::Key;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

#[derive(Clone, Debug)]
pub struct Config {
    /// Number of hypotheses kept after each slice.
    pub beam_width: usize,
    /// Number of new readings, ranked by their fit to the opening slice, considered per slice.
    pub candidate_limit: usize,
    /// Number of recent segments that identify a hypothesis when the beam is pruned.
    /// Of the hypotheses whose last `distinct_depth` segments the rules' distinguish hooks count as
    /// the same, only the best is kept. Zero, or a rule set without distinguish hooks, keeps them all.
    pub distinct_depth: usize,
    /// Keeps the beam of every slice, for tracing how readings rise and fall in the ranking.
    pub record_snapshots: bool,
    /// Keeps the hypotheses that pruning drops at every slice, with the reason, for explaining
    /// why readings leave the beam.
    pub record_pruning: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            beam_width: 80,
            candidate_limit: 48,
            distinct_depth: 0,
            record_snapshots: false,
            record_pruning: false,
        }
    }
}

pub struct Analysis {
    /// Every hypothesis that survived pruning; nodes point to their parents by index.
    pub arena: Vec<Node>,
    /// Final hypotheses as (arena index, score including the closing of the last segment), best first.
    pub ranking: Vec<(u32, f32)>,
    /// The beam after each slice, best first. Empty unless `Config::record_snapshots` is set.
    pub snapshots: Vec<Vec<u32>>,
    /// The hypotheses dropped at each slice, best first. Empty unless `Config::record_pruning` is set.
    pub pruned: Vec<Vec<Pruned>>,
}

pub fn analyze(slices: &[Slice], rules: &RuleSet, config: &Config) -> Analysis {
    let mut arena: Vec<Node> = Vec::new();
    let mut beam: Vec<u32> = Vec::new();
    let mut snapshots = Vec::new();
    let mut pruned = Vec::new();
    for index in 0..slices.len() {
        let keys = rules.propose_keys(&slices[index]);
        let pending = step(&keys, slices, index, &arena, &beam, rules, config);
        let (kept, dropped) = prune(pending, &arena, rules, config);
        beam = commit(&mut arena, kept);
        if config.record_snapshots {
            snapshots.push(beam.clone());
        }
        if config.record_pruning {
            pruned.push(dropped);
        }
    }
    let ranking = rank_final(slices, &arena, &beam, rules);
    Analysis { arena, ranking, snapshots, pruned }
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
    let exits: Vec<Exit> = beam.iter().map(|&b| exit(slices, index, arena, b, rules)).collect();
    for (reading, opening_score) in openings(keys, slices, index, rules, config.candidate_limit) {
        for (parent, previous_score) in predecessors(&reading, slices, index, arena, beam, &exits, rules) {
            let segment = open_segment(reading, index, parent, slice, rules);
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
        Node {
            parent: b,
            slice: index as u32,
            segment: extend(node.segment, slice, rules),
            score: node.score + fit,
        }
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

/// The scores of leaving a hypothesis's last segment on a given slice, which are the same for every
/// new reading (departure scores apply only to readings in another key).
struct Exit {
    close: f32,
    departures: Vec<f32>,
    /// The rival hooks' groups for the hypothesis's last segment, in rule order.
    rivals: Vec<Option<u64>>,
}

fn exit(slices: &[Slice], index: usize, arena: &[Node], b: u32, rules: &RuleSet) -> Exit {
    let node = &arena[b as usize];
    let history = History { arena, slices, last: b };
    let context = DepartureContext { previous: &node.segment, slice: &slices[index], history };
    let start = node.segment.start as usize;
    let rival = RivalContext {
        segment: &node.segment,
        slices: &slices[start..=node.slice as usize],
        previous_slice: start.checked_sub(1).map(|i| &slices[i]),
    };
    Exit {
        close: close_score(slices, node, Some(&slices[index]), rules),
        departures: rules.score_departures(&context),
        rivals: rules.rival_groups(&rival),
    }
}

/// Finds the hypotheses that a new reading opens after: the one that leads into it best and, for
/// each rival hook that puts that hypothesis's last segment in a group, the best one whose last
/// segment has the same group and another reading. Returns each one's index and the score of the
/// path up to the new segment (closing and transition scores included), best first.
fn predecessors(
    reading: &Reading,
    slices: &[Slice],
    index: usize,
    arena: &[Node],
    beam: &[u32],
    exits: &[Exit],
    rules: &RuleSet,
) -> Vec<(u32, f32)> {
    let evaluate = |previous: Option<&Segment>, last: u32, departures: &[f32]| {
        let history = History { arena, slices, last };
        let context = TransitionContext { previous, next: reading, slice: &slices[index], history };
        rules.admits(&context).then(|| rules.score_transition(&context, departures))
    };
    if beam.is_empty() {
        return evaluate(None, NONE, &[]).map(|score| (NONE, score)).into_iter().collect();
    }
    let scored: Vec<(usize, f32)> = (0..beam.len())
        .filter_map(|position| {
            let (b, exit) = (beam[position], &exits[position]);
            let node = &arena[b as usize];
            let transition = evaluate(Some(&node.segment), b, &exit.departures)?;
            Some((position, node.score + exit.close + transition))
        })
        .collect();
    let best_of = |candidates: &mut dyn Iterator<Item = &(usize, f32)>| {
        candidates.copied().max_by(|a, b| a.1.total_cmp(&b.1))
    };
    let Some(best) = best_of(&mut scored.iter()) else {
        return vec![];
    };
    let mut found = vec![best];
    let best_reading = arena[beam[best.0] as usize].segment.reading;
    for (rule, group) in exits[best.0].rivals.iter().enumerate() {
        let Some(group) = *group else {
            continue;
        };
        let is_rival = |&&(position, _): &&(usize, f32)| {
            exits[position].rivals[rule] == Some(group)
                && arena[beam[position] as usize].segment.reading != best_reading
        };
        let rival = best_of(&mut scored.iter().filter(is_rival));
        if let Some(rival) = rival.filter(|r| !found.iter().any(|f| f.0 == r.0)) {
            found.push(rival);
        }
    }
    found.into_iter().map(|(position, score)| (beam[position], score)).collect()
}

fn open_segment(reading: Reading, index: usize, previous: u32, slice: &Slice, rules: &RuleSet) -> Segment {
    let empty = Segment {
        reading,
        start: index as u32,
        previous,
        duration: 0.0,
        members_heard: 0,
        bass: Bass::default(),
    };
    extend(empty, slice, rules)
}

fn extend(mut segment: Segment, slice: &Slice, rules: &RuleSet) -> Segment {
    let lowest = slice.bass().map(|n| n.midi);
    let opens_segment = segment.duration == 0.0;
    segment.duration += slice.duration;
    for note in &slice.notes {
        let Some(member) = segment.reading.chord.member_index(note.tpc) else {
            continue;
        };
        segment.members_heard |= 1 << member;
        let (midi, tpc, metric_level) = (note.midi, note.tpc, slice.metric_level);
        let is_slice_bass = lowest == Some(midi);
        let heard = BassNote { midi, tpc, member, is_slice_bass, metric_level, opens_segment };
        segment.bass = rules.hear_bass(segment.bass, &heard);
    }
    segment
}

fn close_score(slices: &[Slice], node: &Node, next_slice: Option<&Slice>, rules: &RuleSet) -> f32 {
    let span = &slices[node.segment.start as usize..=node.slice as usize];
    rules.score_close(&CloseContext { segment: &node.segment, slices: span, next_slice })
}

/// Keeps the best hypotheses, skipping any whose recent segments the rules count as the same as
/// those of a better one (see `Config::distinct_depth`). The sort is stable, so ties are broken by
/// insertion order. Also returns the dropped hypotheses when `Config::record_pruning` is set.
fn prune(
    mut pending: Vec<Node>,
    arena: &[Node],
    rules: &RuleSet,
    config: &Config,
) -> (Vec<Node>, Vec<Pruned>) {
    pending.sort_by(|a, b| b.score.total_cmp(&a.score));
    let first = arena.len() as u32;
    let merges = config.distinct_depth > 0 && !rules.distinguish_rules.is_empty();
    let mut seen: HashMap<Vec<u64>, u32> = HashMap::new();
    let mut kept = Vec::with_capacity(config.beam_width);
    let mut dropped = Vec::new();
    for (rank, node) in pending.into_iter().enumerate() {
        let cause = if kept.len() == config.beam_width {
            if !config.record_pruning {
                break;
            }
            PruneCause::Width
        } else if !merges {
            kept.push(node);
            continue;
        } else {
            match seen.entry(signature(&node.segment, arena, rules, config.distinct_depth)) {
                Entry::Vacant(entry) => {
                    entry.insert(first + kept.len() as u32);
                    kept.push(node);
                    continue;
                }
                Entry::Occupied(entry) => PruneCause::Merged { keeper: *entry.get() },
            }
        };
        if config.record_pruning {
            dropped.push(Pruned { node, rank: rank + 1, cause });
        }
    }
    (kept, dropped)
}

/// Returns the distinguish hooks' values for a segment and for up to `depth - 1` segments before it,
/// most recent first.
fn signature(segment: &Segment, arena: &[Node], rules: &RuleSet, depth: usize) -> Vec<u64> {
    let mut values = Vec::with_capacity(depth * rules.distinguish_rules.len());
    let mut current = segment;
    for count in 1.. {
        rules.distinguish(current, &mut values);
        if count == depth || current.previous == NONE {
            break;
        }
        current = &arena[current.previous as usize].segment;
    }
    values
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
    use crate::engine::rule::Rule;
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

    /// A rule set whose only rule counts segments with equal readings as the same.
    fn distinguishing_rules() -> RuleSet {
        let rules = crate::rules::function_distinct_readings::RULES.to_vec();
        RuleSet::new(rules, &Default::default()).unwrap()
    }

    fn kept_starts(arena: &[Node], config: &Config) -> Vec<u32> {
        prune(pending(), arena, &distinguishing_rules(), config).0.iter().map(|n| n.segment.start).collect()
    }

    #[test]
    fn prune_keeps_only_the_best_of_hypotheses_with_the_same_recent_readings() {
        assert_eq!(kept_starts(&arena("C", "C"), &merging()), [2]);
    }

    #[test]
    fn prune_keeps_hypotheses_whose_earlier_readings_differ() {
        assert_eq!(kept_starts(&arena("C", "F"), &merging()), [2, 1]);
    }

    /// The search with merging turned on, at the depth used before round 8.
    fn merging() -> Config {
        Config { distinct_depth: 4, ..Config::default() }
    }

    #[test]
    fn prune_keeps_every_boundary_variant_when_distinct_depth_is_zero() {
        let config = Config { distinct_depth: 0, ..Config::default() };
        assert_eq!(kept_starts(&arena("C", "C"), &config), [2, 1]);
    }

    #[test]
    fn prune_keeps_every_boundary_variant_without_distinguish_rules() {
        let rules = RuleSet::new(vec![], &Default::default()).unwrap();
        let kept = prune(pending(), &arena("C", "C"), &rules, &Config::default()).0;
        assert_eq!(kept.iter().map(|n| n.segment.start).collect::<Vec<_>>(), [2, 1]);
    }

    #[test]
    fn prune_looks_back_no_further_than_distinct_depth() {
        let config = Config { distinct_depth: 1, ..Config::default() };
        assert_eq!(kept_starts(&arena("C", "F"), &config), [2]);
    }

    fn dropped_causes(arena: &[Node], config: &Config) -> Vec<PruneCause> {
        let config = Config { record_pruning: true, ..config.clone() };
        prune(pending(), arena, &distinguishing_rules(), &config).1.iter().map(|p| p.cause).collect()
    }

    #[test]
    fn prune_records_the_hypothesis_kept_in_place_of_a_merged_one() {
        let arena = arena("C", "C");
        assert_eq!(dropped_causes(&arena, &merging()), [PruneCause::Merged { keeper: 2 }]);
    }

    #[test]
    fn prune_records_hypotheses_cut_by_beam_width() {
        let config = Config { beam_width: 1, ..Config::default() };
        assert_eq!(dropped_causes(&arena("C", "F"), &config), [PruneCause::Width]);
    }

    #[test]
    fn prune_keeps_at_most_beam_width_hypotheses() {
        let config = Config { beam_width: 1, ..Config::default() };
        assert_eq!(kept_starts(&arena("C", "F"), &config), [2]);
    }

    fn same_group(_: &RivalContext, _: &[f64]) -> Option<u64> {
        Some(0)
    }

    /// Returns the parents that a new reading on slice 3 opens after, from a beam of two hypotheses
    /// whose last segments end on slice 2 with the given roots and scores.
    fn parents(rival_hook: bool, roots: [&str; 2], scores: [f32; 2]) -> Vec<u32> {
        let rule = Rule {
            id: "test.rival",
            layer: crate::engine::rule::Layer::Chord,
            summary: "",
            citations: &[],
            default_weight: 1.0,
            params: &[],
            hook: crate::engine::rule::Hook::Rival(same_group),
        };
        let rules = RuleSet::new(if rival_hook { vec![rule] } else { vec![] }, &Default::default()).unwrap();
        let arena = vec![node(roots[0], 0, NONE, scores[0]), node(roots[1], 0, NONE, scores[1])];
        let slice = Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![],
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        };
        let slices = vec![slice; 4];
        let beam = [0, 1];
        let exits: Vec<Exit> = beam.iter().map(|&b| exit(&slices, 3, &arena, b, &rules)).collect();
        let found = predecessors(&reading("C"), &slices, 3, &arena, &beam, &exits, &rules);
        found.iter().map(|&(parent, _)| parent).collect()
    }

    #[test]
    fn a_new_segment_opens_after_the_best_hypothesis_alone() {
        assert_eq!(parents(false, ["G", "F"], [0.0, -1.0]), [0]);
    }

    #[test]
    fn a_new_segment_also_opens_after_the_best_rival_reading() {
        assert_eq!(parents(true, ["G", "F"], [0.0, -1.0]), [0, 1]);
    }

    #[test]
    fn a_rival_needs_another_reading() {
        assert_eq!(parents(true, ["G", "G"], [0.0, -1.0]), [0]);
    }
}
