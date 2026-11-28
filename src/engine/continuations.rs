//! Extends the hypotheses in the beam, and the chord hypotheses they read, by one more slice.

use super::chord_hooks::ChordSliceContext;
use super::hypothesis::Hypothesis;
use super::key_contexts::KeySliceContext;
use super::model::{ChordHypothesisId, KeyHypothesisId, Reading};
use super::moment::Moment;
use super::pending::{Pending, PendingChordHypothesisId, Proposed};
use super::ruleset::RuleSet;
use super::segment_extension::extend_segment;
use crate::theory::chord::Chord;
use std::collections::HashMap;

/// Extends every hypothesis in the beam by the moment's slice, and with it the chord hypothesis it reads.
/// Each chord hypothesis is extended once, however many hypotheses read it.
pub fn continuations(moment: &Moment, beam: &[KeyHypothesisId], rules: &RuleSet) -> Pending {
    let mut extension = Extension {
        moment,
        rules,
        pending: Pending { chord: Proposed::with_capacity(0), key: Proposed::with_capacity(beam.len()) },
        extended: HashMap::new(),
        chord_fits: HashMap::new(),
        key_fits: HashMap::new(),
    };
    for &hypothesis in beam {
        extension.extend(hypothesis);
    }
    extension.pending
}

/// The continuations proposed so far, with the slice rules' scores already computed.
struct Extension<'a> {
    moment: &'a Moment<'a>,
    rules: &'a RuleSet,
    pending: Pending,
    /// The extension of each chord hypothesis extended so far.
    extended: HashMap<ChordHypothesisId, PendingChordHypothesisId>,
    chord_fits: HashMap<Chord, f32>,
    key_fits: HashMap<Reading, f32>,
}

impl Extension<'_> {
    fn extend(&mut self, parent: KeyHypothesisId) {
        let hypothesis = *self.moment.layers.key.get(parent);
        let chord = self.moment.layers.chord.get(hypothesis.reads).unit.chord;
        let chord_fit = self.chord_fit(chord);
        let reads = self.extend_chord_hypothesis(hypothesis.reads, chord_fit);
        let key_fit = self.key_fit(Reading { chord, function: hypothesis.unit });
        let (previous, unit) = (hypothesis.previous, hypothesis.unit);
        let score = hypothesis.score + key_fit;
        self.pending.key.add(Hypothesis { parent, previous, reads, unit, score });
    }

    /// Returns the extension of a chord hypothesis by the slice, given its chord's fit to the slice.
    fn extend_chord_hypothesis(&mut self, id: ChordHypothesisId, chord_fit: f32) -> PendingChordHypothesisId {
        if let Some(&extended) = self.extended.get(&id) {
            return extended;
        }
        let chord_hypothesis = self.moment.layers.chord.get(id);
        let next = Hypothesis {
            parent: id,
            previous: chord_hypothesis.previous,
            reads: self.moment.index as u32,
            unit: extend_segment(chord_hypothesis.unit, self.moment.slice(), self.rules),
            score: chord_hypothesis.score + chord_fit,
        };
        let extended = self.pending.chord.add(next);
        self.extended.insert(id, extended);
        extended
    }

    fn chord_fit(&mut self, chord: Chord) -> f32 {
        let (moment, rules) = (self.moment, self.rules);
        *self.chord_fits.entry(chord).or_insert_with(|| {
            rules.chord.score_slice(&ChordSliceContext::at(moment.slices, moment.index, chord, false))
        })
    }

    fn key_fit(&mut self, reading: Reading) -> f32 {
        let (moment, rules) = (self.moment, self.rules);
        *self.key_fits.entry(reading).or_insert_with(|| {
            rules.key.score_slice(&KeySliceContext::at(moment.slices, moment.index, &reading, false))
        })
    }
}
