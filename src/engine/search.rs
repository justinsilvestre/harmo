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
use super::chord_hooks::{ChordCloseContext, ChordSliceContext, ChordTransitionContext};
use super::key_hooks::{
    History, KeyCloseContext, KeyDepartureContext, KeySliceContext, KeyTransitionContext, RivalContext,
};
use super::layers::Layers;
use super::model::{ChordSegment, Hearing, Hypothesis, KeySegment, NONE, Reading};
use super::pruning::{PruneCause, Pruned};
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
    /// Every hypothesis that survived pruning, and every hearing of the chords that one of them
    /// interprets.
    pub layers: Layers,
    /// The final hypotheses, best first, each with its score once its last segment is closed.
    /// Each is given by its index in the hypotheses.
    pub ranking: Vec<(u32, f32)>,
    /// The beam after each slice, best first. Empty unless `Config::record_snapshots` is set.
    pub snapshots: Vec<Vec<u32>>,
    /// The hypotheses dropped at each slice, best first. Empty unless `Config::record_pruning` is set.
    pub pruned: Vec<Vec<Pruned>>,
}

impl Analysis {
    /// Returns the segment that a hypothesis ends in, as the key layer reads it.
    pub fn segment(&self, hypothesis: u32) -> KeySegment {
        self.layers.segment(hypothesis)
    }
}

pub fn analyze(slices: &[Slice], rules: &RuleSet, config: &Config) -> Analysis {
    let mut layers = Layers::default();
    let mut beam: Vec<u32> = Vec::new();
    let mut snapshots = Vec::new();
    let mut pruned = Vec::new();
    for index in 0..slices.len() {
        let keys = rules.key.propose(&slices[index]);
        let pending = step(&keys, slices, index, &layers, &beam, rules, config);
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

/// What the search proposes at one slice, before pruning: hypotheses, each extending or opening
/// after one in the beam, and the new hearings of the chords that they interpret.
struct Pending {
    hearings: Vec<Hearing>,
    /// Each hypothesis's `hearing` is its index in `hearings`.
    hypotheses: Vec<Hypothesis>,
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
    layers: &Layers,
    beam: &[u32],
    rules: &RuleSet,
    config: &Config,
) -> Pending {
    let exits: Vec<Exit> = beam.iter().map(|&b| exit(slices, index, layers, b, rules)).collect();
    let (proposed, openings) = openings(keys, slices, index, rules, config);
    let transitions = chord_transitions(slices, index, &proposed, &exits, &layers.hearings, rules);
    let mut opened: Vec<(&Opening, Option<usize>, f32)> = Vec::new();
    for opening in &openings {
        let found = predecessors(opening, slices, index, layers, beam, &exits, &transitions, rules);
        opened.extend(found.into_iter().map(|(position, score)| (opening, position, score)));
    }
    let mut pending = continuations(slices, index, layers, beam, rules);
    let hearings = &mut pending.hearings;
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
                if parent_chord == NONE { 0.0 } else { layers.hearings[parent_chord as usize].score };
            let score = previous_score + close + transition + opening.chord_fit;
            let segment = open_segment(chord, index, &slices[index], rules);
            let (parent, previous, slice) = (parent_chord, parent_chord, index as u32);
            hearings.push(Hearing { parent, previous, slice, segment, score });
            hearings.len() as u32 - 1
        });
        pending.hypotheses.push(Hypothesis {
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
fn continuations(slices: &[Slice], index: usize, layers: &Layers, beam: &[u32], rules: &RuleSet) -> Pending {
    let (slice, next_slice) = (&slices[index], slices.get(index + 1));
    let mut chord_fits: HashMap<Chord, f32> = HashMap::new();
    let mut key_fits: HashMap<Reading, f32> = HashMap::new();
    let mut extended: HashMap<u32, u32> = HashMap::new();
    let mut hearings = Vec::new();
    let mut hypotheses = Vec::with_capacity(beam.len());
    for &b in beam {
        let node = &layers.hypotheses[b as usize];
        let hearing = layers.hearings[node.hearing as usize];
        let chord = hearing.segment.chord;
        let chord_fit = *chord_fits.entry(chord).or_insert_with(|| {
            let context = ChordSliceContext { slice, next_slice, slices, index, chord, is_opening: false };
            rules.chord.score_slice(&context)
        });
        let next_hearing = *extended.entry(node.hearing).or_insert_with(|| {
            let segment = extend(hearing.segment, slice, rules);
            hearings.push(Hearing {
                parent: node.hearing,
                slice: index as u32,
                segment,
                score: hearing.score + chord_fit,
                ..hearing
            });
            hearings.len() as u32 - 1
        });
        let reading = Reading { key: node.key, chord, function: node.function };
        let key_fit = *key_fits.entry(reading).or_insert_with(|| {
            let context =
                KeySliceContext { slice, next_slice, slices, index, reading: &reading, is_opening: false };
            rules.key.score_slice(&context)
        });
        hypotheses.push(Hypothesis {
            parent: b,
            slice: index as u32,
            hearing: next_hearing,
            score: node.score + chord_fit + key_fit,
            ..*node
        });
    }
    Pending { hearings, hypotheses }
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
    let chords = rules.chord.propose(slice);
    let mut readings: Vec<Reading> = Vec::new();
    for &chord in &chords {
        for &key in keys {
            for interpret in &rules.key.interpreters {
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
            rules.chord.score_slice(&context)
        })
        .collect();
    let score = |reading: Reading| {
        let context =
            KeySliceContext { slice, next_slice, slices, index, reading: &reading, is_opening: true };
        let chord_id = chords.binary_search(&reading.chord).expect("a reading's chord was proposed");
        let chord_fit = chord_fits[chord_id];
        Opening { reading, fit: chord_fit + rules.key.score_slice(&context), chord_fit, chord_id }
    };
    let mut scored: Vec<Opening> = readings.into_iter().map(score).collect();
    scored.sort_by(|a, b| b.fit.total_cmp(&a.fit));
    let rest = scored.split_off(config.candidate_limit.min(scored.len()));
    let mut rivals: Vec<Reading> = Vec::new();
    for opening in &scored {
        for (rival_of, params) in &rules.key.rival_candidate {
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
    segment: KeySegment,
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

fn exit(slices: &[Slice], index: usize, layers: &Layers, b: u32, rules: &RuleSet) -> Exit {
    let node = &layers.hypotheses[b as usize];
    let segment = layers.segment(b);
    let history = History { layers, slices, last: b };
    let context = KeyDepartureContext { previous: &segment, slice: &slices[index], history };
    let start = segment.chord.start as usize;
    let rival = RivalContext {
        segment: &segment,
        slices: &slices[start..=node.slice as usize],
        previous_slice: start.checked_sub(1).map(|i| &slices[i]),
    };
    let next_slice = Some(&slices[index]);
    let chord_close = chord_close_score(slices, layers.hearing_of(b), next_slice, rules);
    Exit {
        close: chord_close + key_close_score(slices, node, &segment, next_slice, rules),
        chord_close,
        departures: rules.key.score_departures(&context),
        rivals: rules.key.rival_groups(&rival),
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
            rules.chord.score_transition(&ChordTransitionContext {
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
    layers: &Layers,
    beam: &[u32],
    exits: &[Exit],
    transitions: &[Rc<[f32]>],
    rules: &RuleSet,
) -> Vec<(Option<usize>, f32)> {
    let reading = &opening.reading;
    let evaluate = |previous: Option<&KeySegment>, last: u32, (departures, row): (&[f32], usize)| {
        let history = History { layers, slices, last };
        let context = KeyTransitionContext { previous, next: reading, slice: &slices[index], history };
        let chord_transition = transitions[row][opening.chord_id];
        rules
            .key
            .admits(&context)
            .then(|| chord_transition + rules.key.score_transition(&context, departures))
    };
    if beam.is_empty() {
        return evaluate(None, NONE, (&[], 0)).map(|score| (None, score)).into_iter().collect();
    }
    let scored: Vec<(usize, f32)> = (0..beam.len())
        .filter_map(|position| {
            let (b, exit) = (beam[position], &exits[position]);
            let transition = evaluate(Some(&exit.segment), b, (&exit.departures, position))?;
            Some((position, layers.hypotheses[b as usize].score + exit.close + transition))
        })
        .collect();
    let best_of = |candidates: &mut dyn Iterator<Item = &(usize, f32)>| {
        candidates.copied().max_by(|a, b| a.1.total_cmp(&b.1))
    };
    let Some(best) = best_of(&mut scored.iter()) else {
        return vec![];
    };
    let mut found = vec![best];
    let best_reading = exits[best.0].segment.reading();
    for (rule, group) in exits[best.0].rivals.iter().enumerate() {
        let Some(group) = *group else {
            continue;
        };
        let is_rival = |&&(position, _): &&(usize, f32)| {
            exits[position].rivals[rule] == Some(group) && exits[position].segment.reading() != best_reading
        };
        let rival = best_of(&mut scored.iter().filter(is_rival));
        if let Some(rival) = rival.filter(|r| !found.iter().any(|f| f.0 == r.0)) {
            found.push(rival);
        }
    }
    found.into_iter().map(|(position, score)| (Some(position), score)).collect()
}

fn open_segment(chord: Chord, index: usize, slice: &Slice, rules: &RuleSet) -> ChordSegment {
    let empty =
        ChordSegment { chord, start: index as u32, duration: 0.0, members_heard: 0, bass: Bass::default() };
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
        segment.bass = rules.chord.hear_bass(segment.bass, &heard);
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
    rules.chord.score_close(&ChordCloseContext { segment: &hearing.segment, slices: span, next_slice })
}

fn key_close_score(
    slices: &[Slice],
    node: &Hypothesis,
    segment: &KeySegment,
    next_slice: Option<&Slice>,
    rules: &RuleSet,
) -> f32 {
    let span = &slices[segment.chord.start as usize..=node.slice as usize];
    rules.key.score_close(&KeyCloseContext { segment, slices: span, next_slice })
}

/// Keeps the best of the pending hypotheses, skipping any whose recent segments the rules count as
/// the same as those of a better one (see `Config::distinct_depth`). The sort is stable, so ties are
/// broken by the order in which the hypotheses were proposed. Also returns the dropped hypotheses
/// when `Config::record_pruning` is set.
fn prune(
    pending: &Pending,
    layers: &Layers,
    rules: &RuleSet,
    config: &Config,
) -> (Vec<Hypothesis>, Vec<Pruned>) {
    let mut ranked = pending.hypotheses.clone();
    ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
    let first = layers.hypotheses.len() as u32;
    let merges = config.distinct_depth > 0 && !rules.key.distinguish.is_empty();
    let mut seen: HashMap<Vec<u64>, u32> = HashMap::new();
    let mut kept = Vec::with_capacity(config.beam_width);
    let mut dropped = Vec::new();
    let mut kept_hearings: HashSet<u32> = HashSet::new();
    let segment = |hypothesis: &Hypothesis| {
        KeySegment::of(pending.hearings[hypothesis.hearing as usize].segment, hypothesis)
    };
    for (rank, hypothesis) in ranked.into_iter().enumerate() {
        let cause = if kept.len() == config.beam_width {
            if !config.record_pruning {
                break;
            }
            PruneCause::Width
        } else if is_hearing_beyond_limit(&kept_hearings, hypothesis.hearing, config) {
            PruneCause::HearingWidth
        } else if !merges {
            kept_hearings.insert(hypothesis.hearing);
            kept.push(hypothesis);
            continue;
        } else {
            match seen.entry(signature(segment(&hypothesis), layers, rules, config.distinct_depth)) {
                Entry::Vacant(entry) => {
                    entry.insert(first + kept.len() as u32);
                    kept_hearings.insert(hypothesis.hearing);
                    kept.push(hypothesis);
                    continue;
                }
                Entry::Occupied(entry) => PruneCause::Merged { keeper: *entry.get() },
            }
        };
        if config.record_pruning {
            let (parent, score) = (hypothesis.parent, hypothesis.score);
            dropped.push(Pruned { parent, score, segment: segment(&hypothesis), rank: rank + 1, cause });
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
fn signature(segment: KeySegment, layers: &Layers, rules: &RuleSet, depth: usize) -> Vec<u64> {
    let mut values = Vec::with_capacity(depth * rules.key.distinguish.len());
    let mut current = segment;
    for count in 1.. {
        rules.key.distinguish(&current, &mut values);
        if count == depth || current.previous == NONE {
            break;
        }
        current = layers.segment(current.previous);
    }
    values
}

/// Adds the kept hypotheses to the layers, with the pending hearings that they interpret, in the
/// order in which the hypotheses first interpret them. Returns the new beam.
fn commit(layers: &mut Layers, hearings: &[Hearing], kept: Vec<Hypothesis>) -> Vec<u32> {
    let first = layers.hypotheses.len() as u32;
    let mut committed = vec![NONE; hearings.len()];
    for hypothesis in kept {
        let pending = hypothesis.hearing as usize;
        if committed[pending] == NONE {
            committed[pending] = layers.hearings.len() as u32;
            layers.hearings.push(hearings[pending]);
        }
        layers.hypotheses.push(Hypothesis { hearing: committed[pending], ..hypothesis });
    }
    (first..layers.hypotheses.len() as u32).collect()
}

fn rank_final(slices: &[Slice], layers: &Layers, beam: &[u32], rules: &RuleSet) -> Vec<(u32, f32)> {
    let finish = |b: u32| {
        let node = &layers.hypotheses[b as usize];
        let segment = layers.segment(b);
        let close = chord_close_score(slices, layers.hearing_of(b), None, rules)
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
