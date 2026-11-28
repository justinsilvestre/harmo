//! The new readings that the search proposes for a segment opening on a slice.

use super::chord_hooks::ChordSliceContext;
use super::key_hooks::KeySliceContext;
use super::model::{ChordSegment, Reading};
use super::moment::Moment;
use super::ruleset::RuleSet;
use super::segment_extension::open_segment;
use crate::score::slicing::Slice;
use crate::theory::chord::Chord;

/// A score made of a part from each layer's rules.
#[derive(Clone, Copy, Default)]
pub struct LayerScores {
    pub chord: f32,
    pub key: f32,
}

impl LayerScores {
    pub fn total(self) -> f32 {
        self.chord + self.key
    }
}

/// A chord proposed for a segment opening on a slice.
pub struct ProposedChord {
    pub chord: Chord,
    /// The chord slice rules' score of the chord on the opening slice.
    pub fit: f32,
    /// The chord segment that the chord opens on the slice.
    pub segment: ChordSegment,
}

/// A new reading for a segment opening on a slice, with its fit to that slice.
pub struct Opening {
    pub reading: Reading,
    /// The slice rules' scores of the reading on the opening slice.
    pub fit: LayerScores,
    /// The index of the reading's chord in `Proposals::chords`.
    pub chord_index: usize,
}

pub struct Proposals {
    /// The chords proposed, in order and without repeats.
    pub chords: Vec<ProposedChord>,
    /// The new readings considered, best-fitting first, followed by the rival candidates of those.
    pub openings: Vec<Opening>,
}

/// Proposes new readings for a segment opening on the moment's slice, and keeps the
/// `candidate_limit` that fit it best, with the rival candidates that rules name for them.
pub fn propose(moment: &Moment, rules: &RuleSet, candidate_limit: usize) -> Proposals {
    let chords: Vec<ProposedChord> = rules
        .chord
        .propose(moment.slice())
        .into_iter()
        .map(|chord| propose_chord(chord, moment, rules))
        .collect();
    let fit = |reading: Reading| fit_opening(reading, &chords, moment, rules);
    let mut openings: Vec<Opening> = readings(&chords, moment.slice(), rules).into_iter().map(fit).collect();
    openings.sort_by(|a, b| b.fit.total().total_cmp(&a.fit.total()));
    let rest = openings.split_off(candidate_limit.min(openings.len()));
    let rivals = rival_candidates(&openings, moment.slice(), rules);
    openings.extend(rest.into_iter().filter(|opening| is_named(&rivals, &opening.reading)));
    Proposals { chords, openings }
}

fn propose_chord(chord: Chord, moment: &Moment, rules: &RuleSet) -> ProposedChord {
    let fit = rules.chord.score_slice(&ChordSliceContext::at(moment.slices, moment.index, chord, true));
    ProposedChord { chord, fit, segment: open_segment(chord, moment, rules) }
}

/// Returns every reading of a proposed chord in a proposed key that the interpreters allow, without
/// repeats.
fn readings(chords: &[ProposedChord], slice: &Slice, rules: &RuleSet) -> Vec<Reading> {
    let keys = rules.key.propose(slice);
    let mut readings: Vec<Reading> = Vec::new();
    for &ProposedChord { chord, .. } in chords {
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
    readings
}

fn fit_opening(reading: Reading, chords: &[ProposedChord], moment: &Moment, rules: &RuleSet) -> Opening {
    let chord_index =
        chords.binary_search_by_key(&reading.chord, |c| c.chord).expect("a reading's chord was proposed");
    let context = KeySliceContext::at(moment.slices, moment.index, &reading, true);
    let fit = LayerScores { chord: chords[chord_index].fit, key: rules.key.score_slice(&context) };
    Opening { reading, fit, chord_index }
}

/// Returns the rival readings that the rival-candidate rules name for the openings.
fn rival_candidates(openings: &[Opening], slice: &Slice, rules: &RuleSet) -> Vec<Reading> {
    let rivals_of = |opening: &Opening| {
        let rules = rules.key.rival_candidate.iter();
        rules.filter_map(|(rival_of, params)| rival_of(&opening.reading, slice, params)).collect::<Vec<_>>()
    };
    openings.iter().flat_map(rivals_of).collect()
}

/// Reports whether a reading is one of the named readings, whatever its category.
fn is_named(named: &[Reading], reading: &Reading) -> bool {
    named.iter().any(|r| {
        r.function.key == reading.function.key
            && r.chord == reading.chord
            && r.function.tonicized == reading.function.tonicized
    })
}
