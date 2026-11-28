//! The engine core: a left-to-right beam search in which every slice either extends the current
//! chord segment or opens a new one with a freshly proposed reading.
//!
//! The search keeps two layers. The chord layer holds hearings of the chords (`Hearing`), scored
//! by the chord rules alone; the key layer holds hypotheses (`Hypothesis`) that each read one hearing's
//! latest segment in a key, scored by every rule. Several hypotheses can share one hearing.
//!
//! The beam at each slice is the set of interpretations a listener might hold at that moment.
//! Hypotheses that open the same reading at the same slice are merged, keeping the best history.
//! Hypotheses whose recent segments the rules count as the same interpretation compete for one place
//! in the beam, so that the beam holds more distinct interpretations.

use super::bass::{Bass, BassNote};
use super::model::{ChordSegment, Hearing, Hypothesis, NONE, Reading, Segment, segment_of};
use super::pruning::{PruneCause, Pruned};
use super::rule::{
    ChordCloseContext, ChordSliceContext, ChordTransitionContext, CloseContext, DepartureContext, History,
    RivalContext, SliceContext, TransitionContext,
};
use super::ruleset::RuleSet;
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;
use crate::theory::key::Key;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Config {
    /// Number of hypotheses kept after each slice.
    pub beam_width: usize,
    /// Number of hearings of the chords kept after each slice, each ranked by the best hypothesis
    /// that interprets it. Zero keeps every hearing that some kept hypothesis interprets.
    pub hearing_width: usize,
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
            hearing_width: 40,
            candidate_limit: 48,
            distinct_depth: 0,
            record_snapshots: false,
            record_pruning: false,
        }
    }
}

pub struct Analysis {
    /// Every hypothesis that survived pruning; each points to its parent by index.
    pub hypotheses: Vec<Hypothesis>,
    /// Every hearing of the chords that some hypothesis in `hypotheses` interprets.
    pub hearings: Vec<Hearing>,
    /// The final hypotheses, best first, each with its score once its last segment is closed.
    /// Each is given by its index in `hypotheses`.
    pub ranking: Vec<(u32, f32)>,
    /// The beam after each slice, best first. Empty unless `Config::record_snapshots` is set.
    pub snapshots: Vec<Vec<u32>>,
    /// The hypotheses dropped at each slice, best first. Empty unless `Config::record_pruning` is set.
    pub pruned: Vec<Vec<Pruned>>,
}

impl Analysis {
    /// Returns the segment that the hypothesis `node` ends in, as both layers read it.
    pub fn segment(&self, node: u32) -> Segment {
        segment_of(&self.hypotheses, &self.hearings, node)
    }
}

pub fn analyze(slices: &[Slice], rules: &RuleSet, config: &Config) -> Analysis {
    let mut hypotheses: Vec<Hypothesis> = Vec::new();
    let mut hearings: Vec<Hearing> = Vec::new();
    let mut beam: Vec<u32> = Vec::new();
    let mut snapshots = Vec::new();
    let mut pruned = Vec::new();
    for index in 0..slices.len() {
        let keys = rules.propose_keys(&slices[index]);
        let first_chord = hearings.len();
        let pending = step(&keys, slices, index, &hypotheses, &mut hearings, &beam, rules, config);
        let (mut kept, dropped) = prune(pending, &hypotheses, &hearings, rules, config);
        keep_interpreted_hearings(&mut hearings, first_chord, &mut kept);
        beam = commit(&mut hypotheses, kept);
        if config.record_snapshots {
            snapshots.push(beam.clone());
        }
        if config.record_pruning {
            pruned.push(dropped);
        }
    }
    let ranking = rank_final(slices, &hypotheses, &hearings, &beam, rules);
    Analysis { hypotheses, hearings, ranking, snapshots, pruned }
}

/// A new reading for a segment opening on a slice, with its fit to that slice.
struct Opening {
    reading: Reading,
    /// The slice rules' score of the reading on the opening slice, both layers included.
    fit: f32,
    /// The chord slice rules' share of `fit`.
    chord_fit: f32,
    /// The index of the reading's chord among the chords proposed for the slice.
    chord_id: usize,
}

#[allow(clippy::too_many_arguments)]
fn step(
    keys: &[Key],
    slices: &[Slice],
    index: usize,
    hypotheses: &[Hypothesis],
    hearings: &mut Vec<Hearing>,
    beam: &[u32],
    rules: &RuleSet,
    config: &Config,
) -> Vec<Hypothesis> {
    let exits: Vec<Exit> =
        beam.iter().map(|&b| exit(slices, index, hypotheses, hearings, b, rules)).collect();
    let (proposed, openings) = openings(keys, slices, index, rules, config);
    let transitions = chord_transitions(slices, index, &proposed, &exits, hearings, rules);
    let mut opened: Vec<(&Opening, Option<usize>, f32)> = Vec::new();
    for opening in &openings {
        let found =
            predecessors(opening, slices, index, (hypotheses, hearings), beam, &exits, &transitions, rules);
        opened.extend(found.into_iter().map(|(position, score)| (opening, position, score)));
    }
    let mut pending = continuations(slices, index, hypotheses, hearings, beam, rules);
    let mut heard: HashMap<(u32, Chord), u32> = HashMap::new();
    for (opening, position, previous_score) in opened {
        let (parent, parent_chord, close) = match position {
            Some(position) => (beam[position], exits[position].hearing, exits[position].chord_close),
            None => (NONE, NONE, 0.0),
        };
        let transition = transitions[position.unwrap_or(0)][opening.chord_id];
        let chord = opening.reading.chord;
        let hearing = *heard.entry((parent_chord, chord)).or_insert_with(|| {
            let previous_score =
                if parent_chord == NONE { 0.0 } else { hearings[parent_chord as usize].score };
            let score = previous_score + close + transition + opening.chord_fit;
            let segment = open_segment(chord, index, parent_chord, &slices[index], rules);
            hearings.push(Hearing { parent: parent_chord, slice: index as u32, segment, score });
            hearings.len() as u32 - 1
        });
        pending.push(Hypothesis {
            parent,
            slice: index as u32,
            hearing,
            key: opening.reading.key,
            function: opening.reading.function,
            previous: parent,
            score: previous_score + opening.fit,
        });
    }
    pending
}

/// Extends every hypothesis in the beam by the slice, and with it the hearing it interprets.
fn continuations(
    slices: &[Slice],
    index: usize,
    hypotheses: &[Hypothesis],
    hearings: &mut Vec<Hearing>,
    beam: &[u32],
    rules: &RuleSet,
) -> Vec<Hypothesis> {
    let (slice, next_slice) = (&slices[index], slices.get(index + 1));
    let mut chord_fits: HashMap<Chord, f32> = HashMap::new();
    let mut key_fits: HashMap<Reading, f32> = HashMap::new();
    let mut extended: HashMap<u32, u32> = HashMap::new();
    let mut nodes = Vec::with_capacity(beam.len());
    for &b in beam {
        let node = &hypotheses[b as usize];
        let hearing = hearings[node.hearing as usize];
        let chord = hearing.segment.chord;
        let chord_fit = *chord_fits.entry(chord).or_insert_with(|| {
            let context = ChordSliceContext { slice, next_slice, slices, index, chord, is_opening: false };
            rules.score_chord_slice(&context)
        });
        let next_hearing = *extended.entry(node.hearing).or_insert_with(|| {
            let segment = extend(hearing.segment, slice, rules);
            hearings.push(Hearing {
                parent: node.hearing,
                slice: index as u32,
                segment,
                score: hearing.score + chord_fit,
            });
            hearings.len() as u32 - 1
        });
        let reading = Reading { key: node.key, chord, function: node.function };
        let key_fit = *key_fits.entry(reading).or_insert_with(|| {
            let context =
                SliceContext { slice, next_slice, slices, index, reading: &reading, is_opening: false };
            rules.score_slice(&context)
        });
        nodes.push(Hypothesis {
            parent: b,
            slice: index as u32,
            hearing: next_hearing,
            score: node.score + chord_fit + key_fit,
            ..*node
        });
    }
    nodes
}

/// Proposes new readings for a segment opening on this slice, best-fitting first.
/// Also returns the chords proposed, in the order that `Opening::chord_id` indexes.
fn openings(
    keys: &[Key],
    slices: &[Slice],
    index: usize,
    rules: &RuleSet,
    config: &Config,
) -> (Vec<Chord>, Vec<Opening>) {
    let (slice, next_slice) = (&slices[index], slices.get(index + 1));
    let mut chords: Vec<_> = rules.proposers.iter().flat_map(|propose| propose(slice)).collect();
    chords.sort();
    chords.dedup();
    let mut readings: Vec<Reading> = Vec::new();
    for &chord in &chords {
        for &key in keys {
            for interpret in &rules.interpreters {
                readings.extend(interpret(key, chord).into_iter().map(|function| Reading {
                    key,
                    chord,
                    function,
                }));
            }
        }
    }
    readings.sort();
    readings.dedup();
    let chord_fits: Vec<f32> = chords
        .iter()
        .map(|&chord| {
            let context = ChordSliceContext { slice, next_slice, slices, index, chord, is_opening: true };
            rules.score_chord_slice(&context)
        })
        .collect();
    let score = |reading: Reading| {
        let context = SliceContext { slice, next_slice, slices, index, reading: &reading, is_opening: true };
        let chord_id = chords.binary_search(&reading.chord).expect("a reading's chord was proposed");
        let chord_fit = chord_fits[chord_id];
        Opening { reading, fit: chord_fit + rules.score_slice(&context), chord_fit, chord_id }
    };
    let mut scored: Vec<Opening> = readings.into_iter().map(score).collect();
    scored.sort_by(|a, b| b.fit.total_cmp(&a.fit));
    let rest = scored.split_off(config.candidate_limit.min(scored.len()));
    let mut rivals: Vec<Reading> = Vec::new();
    for opening in &scored {
        for (rival_of, params) in &rules.rival_candidate_rules {
            rivals.extend(rival_of(&opening.reading, slice, params));
        }
    }
    let is_rival = |r: &Reading| {
        rivals
            .iter()
            .any(|v| v.key == r.key && v.chord == r.chord && v.function.tonicized == r.function.tonicized)
    };
    scored.extend(rest.into_iter().filter(|opening| is_rival(&opening.reading)));
    (chords, scored)
}

/// The scores of leaving a hypothesis's last segment on a given slice, which are the same for every
/// new reading (departure scores apply only to readings in another key).
struct Exit {
    /// The hypothesis's last segment, as both layers read it.
    segment: Segment,
    /// The hearing of the chords that the hypothesis interprets.
    hearing: u32,
    /// The close rules' score of that segment, both layers included.
    close: f32,
    /// The chord close rules' share of `close`.
    chord_close: f32,
    departures: Vec<f32>,
    /// The rival hooks' groups for the hypothesis's last segment, in rule order.
    rivals: Vec<Option<u64>>,
}

fn exit(
    slices: &[Slice],
    index: usize,
    hypotheses: &[Hypothesis],
    hearings: &[Hearing],
    b: u32,
    rules: &RuleSet,
) -> Exit {
    let node = &hypotheses[b as usize];
    let segment = segment_of(hypotheses, hearings, b);
    let history = History { hypotheses, hearings, slices, last: b };
    let context = DepartureContext { previous: &segment, slice: &slices[index], history };
    let start = segment.start as usize;
    let rival = RivalContext {
        segment: &segment,
        slices: &slices[start..=node.slice as usize],
        previous_slice: start.checked_sub(1).map(|i| &slices[i]),
    };
    let next_slice = Some(&slices[index]);
    let chord_close = chord_close_score(slices, &hearings[node.hearing as usize], next_slice, rules);
    Exit {
        close: chord_close + key_close_score(slices, node, &segment, next_slice, rules),
        chord_close,
        departures: rules.score_departures(&context),
        rivals: rules.rival_groups(&rival),
        segment,
        hearing: node.hearing,
    }
}

/// Scores, in the chord layer, opening each of `proposed` after each exit's hearing, indexed by
/// exit and then by chord. With no exits, at the start of the piece, there is one row for opening
/// after nothing.
fn chord_transitions(
    slices: &[Slice],
    index: usize,
    proposed: &[Chord],
    exits: &[Exit],
    hearings: &[Hearing],
    rules: &RuleSet,
) -> Vec<Rc<[f32]>> {
    let row = |hearing: u32| -> Rc<[f32]> {
        let previous = (hearing != NONE).then(|| &hearings[hearing as usize].segment);
        let score = |&next: &Chord| {
            rules.score_chord_transition(&ChordTransitionContext {
                previous,
                next,
                slice: &slices[index],
                slices,
            })
        };
        proposed.iter().map(score).collect()
    };
    if exits.is_empty() {
        return vec![row(NONE)];
    }
    let mut rows: HashMap<u32, Rc<[f32]>> = HashMap::new();
    exits.iter().map(|exit| rows.entry(exit.hearing).or_insert_with(|| row(exit.hearing)).clone()).collect()
}

/// Finds the hypotheses that a new reading opens after: the one that leads into it best and, for
/// each rival hook that puts that hypothesis's last segment in a group, the best one whose last
/// segment has the same group and another reading. Returns each one's position in the beam (`None`
/// at the start of the piece) and the score of the path up to the new segment (closing and
/// transition scores included), best first.
#[allow(clippy::too_many_arguments)]
fn predecessors(
    opening: &Opening,
    slices: &[Slice],
    index: usize,
    (hypotheses, hearings): (&[Hypothesis], &[Hearing]),
    beam: &[u32],
    exits: &[Exit],
    transitions: &[Rc<[f32]>],
    rules: &RuleSet,
) -> Vec<(Option<usize>, f32)> {
    let reading = &opening.reading;
    let evaluate = |previous: Option<&Segment>, last: u32, (departures, row): (&[f32], usize)| {
        let history = History { hypotheses, hearings, slices, last };
        let context = TransitionContext { previous, next: reading, slice: &slices[index], history };
        let chord_transition = transitions[row][opening.chord_id];
        rules.admits(&context).then(|| chord_transition + rules.score_transition(&context, departures))
    };
    if beam.is_empty() {
        return evaluate(None, NONE, (&[], 0)).map(|score| (None, score)).into_iter().collect();
    }
    let scored: Vec<(usize, f32)> = (0..beam.len())
        .filter_map(|position| {
            let (b, exit) = (beam[position], &exits[position]);
            let transition = evaluate(Some(&exit.segment), b, (&exit.departures, position))?;
            Some((position, hypotheses[b as usize].score + exit.close + transition))
        })
        .collect();
    let best_of = |candidates: &mut dyn Iterator<Item = &(usize, f32)>| {
        candidates.copied().max_by(|a, b| a.1.total_cmp(&b.1))
    };
    let Some(best) = best_of(&mut scored.iter()) else {
        return vec![];
    };
    let mut found = vec![best];
    let best_reading = exits[best.0].segment.reading;
    for (rule, group) in exits[best.0].rivals.iter().enumerate() {
        let Some(group) = *group else {
            continue;
        };
        let is_rival = |&&(position, _): &&(usize, f32)| {
            exits[position].rivals[rule] == Some(group) && exits[position].segment.reading != best_reading
        };
        let rival = best_of(&mut scored.iter().filter(is_rival));
        if let Some(rival) = rival.filter(|r| !found.iter().any(|f| f.0 == r.0)) {
            found.push(rival);
        }
    }
    found.into_iter().map(|(position, score)| (Some(position), score)).collect()
}

fn open_segment(chord: Chord, index: usize, previous: u32, slice: &Slice, rules: &RuleSet) -> ChordSegment {
    let empty = ChordSegment {
        chord,
        start: index as u32,
        previous,
        duration: 0.0,
        members_heard: 0,
        bass: Bass::default(),
    };
    extend(empty, slice, rules)
}

fn extend(mut segment: ChordSegment, slice: &Slice, rules: &RuleSet) -> ChordSegment {
    let lowest = slice.bass().map(|n| n.midi);
    let opens_segment = segment.duration == 0.0;
    segment.duration += slice.duration;
    for note in &slice.notes {
        let Some(member) = segment.chord.member_index(note.tpc) else {
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

fn chord_close_score(
    slices: &[Slice],
    hearing: &Hearing,
    next_slice: Option<&Slice>,
    rules: &RuleSet,
) -> f32 {
    let span = &slices[hearing.segment.start as usize..=hearing.slice as usize];
    rules.score_chord_close(&ChordCloseContext { segment: &hearing.segment, slices: span, next_slice })
}

fn key_close_score(
    slices: &[Slice],
    node: &Hypothesis,
    segment: &Segment,
    next_slice: Option<&Slice>,
    rules: &RuleSet,
) -> f32 {
    let span = &slices[segment.start as usize..=node.slice as usize];
    rules.score_close(&CloseContext { segment, slices: span, next_slice })
}

/// Keeps the best hypotheses, skipping any whose recent segments the rules count as the same as
/// those of a better one (see `Config::distinct_depth`). The sort is stable, so ties are broken by
/// insertion order. Also returns the dropped hypotheses when `Config::record_pruning` is set.
fn prune(
    mut pending: Vec<Hypothesis>,
    hypotheses: &[Hypothesis],
    hearings: &[Hearing],
    rules: &RuleSet,
    config: &Config,
) -> (Vec<Hypothesis>, Vec<Pruned>) {
    pending.sort_by(|a, b| b.score.total_cmp(&a.score));
    let first = hypotheses.len() as u32;
    let merges = config.distinct_depth > 0 && !rules.distinguish_rules.is_empty();
    let mut seen: HashMap<Vec<u64>, u32> = HashMap::new();
    let mut kept = Vec::with_capacity(config.beam_width);
    let mut dropped = Vec::new();
    let mut kept_hearings: HashSet<u32> = HashSet::new();
    let joined = |node: &Hypothesis| Segment::join(&hearings[node.hearing as usize].segment, node);
    for (rank, node) in pending.into_iter().enumerate() {
        let cause = if kept.len() == config.beam_width {
            if !config.record_pruning {
                break;
            }
            PruneCause::Width
        } else if is_hearing_beyond_limit(&kept_hearings, node.hearing, config) {
            PruneCause::HearingWidth
        } else if !merges {
            kept_hearings.insert(node.hearing);
            kept.push(node);
            continue;
        } else {
            match seen.entry(signature(joined(&node), (hypotheses, hearings), rules, config.distinct_depth)) {
                Entry::Vacant(entry) => {
                    entry.insert(first + kept.len() as u32);
                    kept_hearings.insert(node.hearing);
                    kept.push(node);
                    continue;
                }
                Entry::Occupied(entry) => PruneCause::Merged { keeper: *entry.get() },
            }
        };
        if config.record_pruning {
            dropped.push(Pruned { node, segment: joined(&node), rank: rank + 1, cause });
        }
    }
    (kept, dropped)
}

/// Reports whether a hypothesis interpreting `hearing` would take the beam past its limit of hearings,
/// given the hearings that the hypotheses already kept interpret.
fn is_hearing_beyond_limit(hearings: &HashSet<u32>, hearing: u32, config: &Config) -> bool {
    config.hearing_width > 0 && hearings.len() == config.hearing_width && !hearings.contains(&hearing)
}

/// Returns the distinguish hooks' values for a segment and for up to `depth - 1` segments before it,
/// most recent first.
fn signature(
    segment: Segment,
    (hypotheses, hearings): (&[Hypothesis], &[Hearing]),
    rules: &RuleSet,
    depth: usize,
) -> Vec<u64> {
    let mut values = Vec::with_capacity(depth * rules.distinguish_rules.len());
    let mut current = segment;
    for count in 1.. {
        rules.distinguish(&current, &mut values);
        if count == depth || current.previous == NONE {
            break;
        }
        current = segment_of(hypotheses, hearings, current.previous);
    }
    values
}

/// Drops the hearings added on this slice that no kept hypothesis interprets, and renumbers the
/// rest in the order the kept hypotheses first interpret them.
fn keep_interpreted_hearings(hearings: &mut Vec<Hearing>, first: usize, kept: &mut [Hypothesis]) {
    let added = hearings.split_off(first);
    let mut renumbered: HashMap<u32, u32> = HashMap::new();
    for node in kept.iter_mut() {
        node.hearing = *renumbered.entry(node.hearing).or_insert_with(|| {
            hearings.push(added[node.hearing as usize - first]);
            hearings.len() as u32 - 1
        });
    }
}

fn commit(hypotheses: &mut Vec<Hypothesis>, nodes: Vec<Hypothesis>) -> Vec<u32> {
    let first = hypotheses.len() as u32;
    hypotheses.extend(nodes);
    (first..hypotheses.len() as u32).collect()
}

fn rank_final(
    slices: &[Slice],
    hypotheses: &[Hypothesis],
    hearings: &[Hearing],
    beam: &[u32],
    rules: &RuleSet,
) -> Vec<(u32, f32)> {
    let finish = |b: u32| {
        let node = &hypotheses[b as usize];
        let segment = segment_of(hypotheses, hearings, b);
        let close = chord_close_score(slices, &hearings[node.hearing as usize], None, rules)
            + key_close_score(slices, node, &segment, None, rules);
        (b, node.score + close)
    };
    let mut ranking: Vec<(u32, f32)> = beam.iter().map(|&b| finish(b)).collect();
    ranking.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranking
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
