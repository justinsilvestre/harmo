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
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

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

/// The point that the search has reached: the slice it hears now, within the score, and what both
/// layers have kept from the slices before it.
#[derive(Clone, Copy)]
struct Moment<'a> {
    slices: &'a [Slice],
    index: usize,
    layers: &'a Layers,
}

impl<'a> Moment<'a> {
    fn slice(&self) -> &'a Slice {
        &self.slices[self.index]
    }

    fn next_slice(&self) -> Option<&'a Slice> {
        self.slices.get(self.index + 1)
    }

    /// Returns the history of the segments up to and including the last segment of a hypothesis.
    fn history(&self, last: u32) -> History<'a> {
        History { layers: self.layers, slices: self.slices, last }
    }
}

/// What the search proposes at one slice, before pruning: hypotheses, each extending or opening
/// after one in the beam, and the new hearings of the chords that they interpret.
struct Pending {
    hearings: Vec<Hearing>,
    /// Each hypothesis's `hearing` is its index in `hearings`.
    hypotheses: Vec<Hypothesis>,
}

/// A score made of a part from each layer's rules.
#[derive(Clone, Copy, Default)]
struct LayerScores {
    chord: f32,
    key: f32,
}

impl LayerScores {
    fn total(self) -> f32 {
        self.chord + self.key
    }
}

/// A new reading for a segment opening on a slice, with its fit to that slice.
struct Opening {
    reading: Reading,
    /// The slice rules' scores of the reading on the opening slice.
    fit: LayerScores,
    /// The index of the reading's chord among the chords proposed for the slice.
    chord_id: usize,
}

fn step(moment: &Moment, beam: &[u32], rules: &RuleSet, config: &Config) -> Pending {
    let (proposed, openings) = openings(moment, rules, config);
    let mut transitions: HashMap<u32, Vec<f32>> = HashMap::new();
    let mut exit_from = |b: u32| {
        let hearing = moment.layers.hypotheses[b as usize].hearing;
        let row = transitions
            .entry(hearing)
            .or_insert_with(|| chord_transitions(moment, &proposed, hearing, rules));
        exit(moment, b, row.clone(), rules)
    };
    let exits: Vec<Exit> = beam.iter().map(|&b| exit_from(b)).collect();
    let mut opened: Vec<(&Opening, Predecessor)> = Vec::new();
    for opening in &openings {
        opened.extend(predecessors(opening, moment, &exits, rules).into_iter().map(|p| (opening, p)));
    }
    let mut pending = continuations(moment, beam, rules);
    let hearings = &mut pending.hearings;
    let mut heard: HashMap<(u32, Chord), u32> = HashMap::new();
    for (opening, predecessor) in opened {
        let exit = predecessor.exit.map(|position| &exits[position]);
        let parent = exit.map_or(NONE, |exit| exit.hypothesis);
        let parent_hearing = exit.map_or(NONE, |exit| exit.hearing);
        let chord = opening.reading.chord;
        let hearing = *heard.entry((parent_hearing, chord)).or_insert_with(|| {
            let (previous_score, close) = match exit {
                Some(exit) => (moment.layers.hearings[parent_hearing as usize].score, exit.close.chord),
                None => (0.0, 0.0),
            };
            let score = previous_score + close + predecessor.chord_transition + opening.fit.chord;
            let segment = open_segment(chord, moment, rules);
            hearings.push(Hearing { previous: parent_hearing, slice: moment.index as u32, segment, score });
            hearings.len() as u32 - 1
        });
        pending.hypotheses.push(Hypothesis {
            parent,
            previous: parent,
            hearing,
            function: opening.reading.function,
            score: predecessor.score + opening.fit.total(),
        });
    }
    pending
}

/// Extends every hypothesis in the beam by the slice, and with it the hearing it interprets.
fn continuations(moment: &Moment, beam: &[u32], rules: &RuleSet) -> Pending {
    let (slices, index, layers) = (moment.slices, moment.index, moment.layers);
    let (slice, next_slice) = (moment.slice(), moment.next_slice());
    let mut chord_fits: HashMap<Chord, f32> = HashMap::new();
    let mut key_fits: HashMap<Reading, f32> = HashMap::new();
    let mut extended: HashMap<u32, u32> = HashMap::new();
    let mut hearings = Vec::new();
    let mut hypotheses = Vec::with_capacity(beam.len());
    for &b in beam {
        let hypothesis = &layers.hypotheses[b as usize];
        let hearing = layers.hearings[hypothesis.hearing as usize];
        let chord = hearing.segment.chord;
        let chord_fit = *chord_fits.entry(chord).or_insert_with(|| {
            let context = ChordSliceContext { slice, next_slice, slices, index, chord, is_opening: false };
            rules.chord.score_slice(&context)
        });
        let next_hearing = *extended.entry(hypothesis.hearing).or_insert_with(|| {
            hearings.push(Hearing {
                slice: index as u32,
                segment: extend(hearing.segment, slice, rules),
                score: hearing.score + chord_fit,
                ..hearing
            });
            hearings.len() as u32 - 1
        });
        let reading = Reading { chord, function: hypothesis.function };
        let key_fit = *key_fits.entry(reading).or_insert_with(|| {
            let context =
                KeySliceContext { slice, next_slice, slices, index, reading: &reading, is_opening: false };
            rules.key.score_slice(&context)
        });
        hypotheses.push(Hypothesis {
            parent: b,
            hearing: next_hearing,
            score: hypothesis.score + chord_fit + key_fit,
            ..*hypothesis
        });
    }
    Pending { hearings, hypotheses }
}

/// Proposes new readings for a segment opening on this slice, best-fitting first.
/// Also returns the chords proposed, in the order that `Opening::chord_id` indexes.
fn openings(moment: &Moment, rules: &RuleSet, config: &Config) -> (Vec<Chord>, Vec<Opening>) {
    let (slices, index) = (moment.slices, moment.index);
    let (slice, next_slice) = (moment.slice(), moment.next_slice());
    let chords = rules.chord.propose(slice);
    let keys = rules.key.propose(slice);
    let mut readings: Vec<Reading> = Vec::new();
    for &chord in &chords {
        for &key in &keys {
            for interpret in &rules.key.interpreters {
                readings
                    .extend(interpret(key, chord).into_iter().map(|function| Reading { chord, function }));
            }
        }
    }
    // Listing readings by key first decides the order of readings that fit the slice equally well.
    readings.sort_by_key(|r| (r.function.key, r.chord, r.function.tonicized, r.function.category));
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
        let fit = LayerScores { chord: chord_fits[chord_id], key: rules.key.score_slice(&context) };
        Opening { reading, fit, chord_id }
    };
    let mut scored: Vec<Opening> = readings.into_iter().map(score).collect();
    scored.sort_by(|a, b| b.fit.total().total_cmp(&a.fit.total()));
    let rest = scored.split_off(config.candidate_limit.min(scored.len()));
    let mut rivals: Vec<Reading> = Vec::new();
    for opening in &scored {
        for (rival_of, params) in &rules.key.rival_candidate {
            rivals.extend(rival_of(&opening.reading, slice, params));
        }
    }
    let is_rival = |r: &Reading| {
        rivals.iter().any(|v| {
            v.function.key == r.function.key
                && v.chord == r.chord
                && v.function.tonicized == r.function.tonicized
        })
    };
    scored.extend(rest.into_iter().filter(|opening| is_rival(&opening.reading)));
    (chords, scored)
}

/// The scores of leaving a hypothesis in the beam on a given slice, which are the same for every
/// new reading, except that departure scores apply only to readings in another key and chord
/// transition scores depend on the new reading's chord.
struct Exit {
    hypothesis: u32,
    /// The hypothesis's score before it is left.
    score: f32,
    /// The hypothesis's last segment.
    segment: KeySegment,
    /// The hearing of the chords that the hypothesis interprets.
    hearing: u32,
    /// The close rules' scores of the last segment.
    close: LayerScores,
    /// The chord transition rules' score of opening each proposed chord after the hearing, in the
    /// order of the proposed chords.
    chord_transitions: Vec<f32>,
    departures: Vec<f32>,
    /// The rival hooks' groups for the hypothesis's last segment, in rule order.
    rivals: Vec<Option<u64>>,
}

fn exit(moment: &Moment, b: u32, chord_transitions: Vec<f32>, rules: &RuleSet) -> Exit {
    let hypothesis = &moment.layers.hypotheses[b as usize];
    let segment = moment.layers.segment(b);
    let context =
        KeyDepartureContext { previous: &segment, slice: moment.slice(), history: moment.history(b) };
    let start = segment.heard.start as usize;
    let rival = RivalContext {
        segment: &segment,
        slices: &moment.slices[start..moment.index],
        previous_slice: start.checked_sub(1).map(|i| &moment.slices[i]),
    };
    Exit {
        hypothesis: b,
        score: hypothesis.score,
        close: close_scores(moment.slices, &segment, moment.index, rules),
        chord_transitions,
        departures: rules.key.score_departures(&context),
        rivals: rules.key.rival_groups(&rival),
        segment,
        hearing: hypothesis.hearing,
    }
}

/// Scores, in the chord layer, opening each of `proposed` after a hearing, or after nothing at
/// the start of the piece if `hearing` is `NONE`.
fn chord_transitions(moment: &Moment, proposed: &[Chord], hearing: u32, rules: &RuleSet) -> Vec<f32> {
    let previous = (hearing != NONE).then(|| &moment.layers.hearings[hearing as usize].segment);
    let (slice, slices) = (moment.slice(), moment.slices);
    let score = |&next: &Chord| {
        rules.chord.score_transition(&ChordTransitionContext { previous, next, slice, slices })
    };
    proposed.iter().map(score).collect()
}

/// A hypothesis in the beam that a new reading opens after, or nothing at the start of the piece.
#[derive(Clone, Copy)]
struct Predecessor {
    /// The hypothesis's position in the beam, or `None` at the start of the piece.
    exit: Option<usize>,
    /// The chord transition rules' score of opening the new reading's chord after it.
    chord_transition: f32,
    /// The score of the path up to the new segment, closing and transition scores included.
    score: f32,
}

/// Finds the hypotheses that a new reading opens after: the one that leads into it best and, for
/// each rival hook that puts that hypothesis's last segment in a group, the best one whose last
/// segment has the same group and whose hearing hears another chord there. Returns them best first.
fn predecessors(opening: &Opening, moment: &Moment, exits: &[Exit], rules: &RuleSet) -> Vec<Predecessor> {
    let reading = &opening.reading;
    let evaluate = |previous: Option<&KeySegment>, last: u32, departures: &[f32], chord_transition: f32| {
        let (slice, history) = (moment.slice(), moment.history(last));
        let context = KeyTransitionContext { previous, next: reading, slice, history };
        rules
            .key
            .admits(&context)
            .then(|| chord_transition + rules.key.score_transition(&context, departures))
    };
    if exits.is_empty() {
        let chord_transition = chord_transitions(moment, &[reading.chord], NONE, rules)[0];
        let transition = evaluate(None, NONE, &[], chord_transition);
        return transition
            .map(|score| Predecessor { exit: None, chord_transition, score })
            .into_iter()
            .collect();
    }
    let scored: Vec<Predecessor> = exits
        .iter()
        .enumerate()
        .filter_map(|(position, exit)| {
            let chord_transition = exit.chord_transitions[opening.chord_id];
            let transition =
                evaluate(Some(&exit.segment), exit.hypothesis, &exit.departures, chord_transition)?;
            let score = exit.score + exit.close.total() + transition;
            Some(Predecessor { exit: Some(position), chord_transition, score })
        })
        .collect();
    let best_of = |indices: &mut dyn Iterator<Item = usize>| {
        indices.max_by(|&a, &b| scored[a].score.total_cmp(&scored[b].score))
    };
    let exit_of = |i: usize| &exits[scored[i].exit.expect("every exit has a position")];
    let Some(best) = best_of(&mut (0..scored.len())) else {
        return vec![];
    };
    let mut found = vec![best];
    let best_chord = exit_of(best).segment.heard.chord;
    for (rule, group) in exit_of(best).rivals.iter().enumerate() {
        let Some(group) = *group else {
            continue;
        };
        let is_rival = |&i: &usize| {
            exit_of(i).rivals[rule] == Some(group) && exit_of(i).segment.heard.chord != best_chord
        };
        if let Some(rival) = best_of(&mut (0..scored.len()).filter(is_rival)).filter(|r| !found.contains(r)) {
            found.push(rival);
        }
    }
    found.into_iter().map(|i| scored[i]).collect()
}

fn open_segment(chord: Chord, moment: &Moment, rules: &RuleSet) -> ChordSegment {
    let start = moment.index as u32;
    let empty = ChordSegment { chord, start, duration: 0.0, members_heard: 0, bass: Bass::default() };
    extend(empty, moment.slice(), rules)
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

/// Returns the close rules' scores of a segment whose last slice is the one before `end`.
fn close_scores(slices: &[Slice], segment: &KeySegment, end: usize, rules: &RuleSet) -> LayerScores {
    let (span, next_slice) = (&slices[segment.heard.start as usize..end], slices.get(end));
    let chord =
        rules.chord.score_close(&ChordCloseContext { segment: &segment.heard, slices: span, next_slice });
    LayerScores { chord, key: rules.key.score_close(&KeyCloseContext { segment, slices: span, next_slice }) }
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
    let segment = |hypothesis: &Hypothesis| KeySegment {
        heard: pending.hearings[hypothesis.hearing as usize].segment,
        function: hypothesis.function,
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
            match seen.entry(signature(
                &segment(&hypothesis),
                hypothesis.previous,
                layers,
                rules,
                config.distinct_depth,
            )) {
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
            let (parent, previous, score) = (hypothesis.parent, hypothesis.previous, hypothesis.score);
            let segment = segment(&hypothesis);
            dropped.push(Pruned { parent, previous, score, segment, rank: rank + 1, cause });
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
/// most recent first. `previous` is the last hypothesis of the segment before it.
fn signature(
    segment: &KeySegment,
    previous: u32,
    layers: &Layers,
    rules: &RuleSet,
    depth: usize,
) -> Vec<u64> {
    let mut values = Vec::with_capacity(depth * rules.key.distinguish.len());
    rules.key.distinguish(segment, &mut values);
    for earlier in layers.path_back(previous).take(depth - 1) {
        rules.key.distinguish(&layers.segment(earlier), &mut values);
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
        let close = close_scores(slices, &layers.segment(b), slices.len(), rules);
        (b, layers.hypotheses[b as usize].score + close.total())
    };
    let mut ranking: Vec<(u32, f32)> = beam.iter().map(|&b| finish(b)).collect();
    ranking.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranking
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
