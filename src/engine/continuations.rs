//! Extends the hypotheses in the beam, and the hearings they interpret, by one more slice.

use super::chord_hooks::ChordSliceContext;
use super::key_hooks::KeySliceContext;
use super::model::{Hearing, HearingId, Hypothesis, Reading};
use super::moment::Moment;
use super::pending::{Pending, PendingHearingId};
use super::ruleset::RuleSet;
use super::segment_extension::extend_segment;
use crate::theory::chord::Chord;
use std::collections::HashMap;

/// Extends every hypothesis in the beam by the moment's slice, and with it the hearing it interprets.
/// Each hearing is extended once, however many hypotheses interpret it.
pub fn continuations(moment: &Moment, beam: &[u32], rules: &RuleSet) -> Pending {
    let mut extension = Extension {
        moment,
        rules,
        pending: Pending { hearings: Vec::new(), hypotheses: Vec::with_capacity(beam.len()) },
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
    /// The extension of each hearing extended so far.
    extended: HashMap<HearingId, PendingHearingId>,
    chord_fits: HashMap<Chord, f32>,
    key_fits: HashMap<Reading, f32>,
}

impl Extension<'_> {
    fn extend(&mut self, parent: u32) {
        let hypothesis = self.moment.layers.hypotheses[parent as usize];
        let chord = self.moment.layers.hearing(hypothesis.hearing).segment.chord;
        let chord_fit = self.chord_fit(chord);
        let hearing = self.extend_hearing(hypothesis.hearing, chord_fit);
        let key_fit = self.key_fit(Reading { chord, function: hypothesis.function });
        let (previous, function) = (hypothesis.previous, hypothesis.function);
        let score = hypothesis.score + chord_fit + key_fit;
        self.pending.hypotheses.push(Hypothesis { parent, previous, hearing, function, score });
    }

    /// Returns the extension of a hearing by the slice, given its chord's fit to the slice.
    fn extend_hearing(&mut self, id: HearingId, chord_fit: f32) -> PendingHearingId {
        if let Some(&extended) = self.extended.get(&id) {
            return extended;
        }
        let hearing = self.moment.layers.hearing(id);
        let segment = extend_segment(hearing.segment, self.moment.slice(), self.rules);
        let slice = self.moment.index as u32;
        let next = Hearing { previous: hearing.previous, slice, segment, score: hearing.score + chord_fit };
        let extended = self.pending.add_hearing(next);
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
