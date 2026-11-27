//! How long each pitch class sounds over any span of a score, summed over voices.

use crate::pitch::pitch_class;
use crate::score::model::Offset;
use crate::score::slicing::Slice;

/// A set of pitch classes, one bit per class (C = bit 0).
pub type PitchClassSet = u16;

/// Cumulative sounding time of each pitch class, in quarter notes, so that any span can be read in constant time.
pub struct Sounding {
    starts: Vec<Offset>,
    /// The sounding time of each pitch class before each slice.
    before: Vec<[f64; 12]>,
    /// How many voices sound each pitch class during each slice.
    voices: Vec<[f64; 12]>,
}

/// The sounding time of a span's chord tones and of all its notes, in quarter notes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fit {
    pub chord_tones: f64,
    pub total: f64,
}

impl Fit {
    /// Returns the share of the sounding time that belongs to chord tones, or `None` when nothing sounds.
    pub fn share(self) -> Option<f64> {
        (self.total > 1e-9).then(|| self.chord_tones / self.total)
    }
}

impl std::ops::Add for Fit {
    type Output = Fit;

    fn add(self, other: Fit) -> Fit {
        Fit { chord_tones: self.chord_tones + other.chord_tones, total: self.total + other.total }
    }
}

impl Sounding {
    pub fn new(slices: &[Slice]) -> Sounding {
        let mut before = Vec::with_capacity(slices.len());
        let mut voices = Vec::with_capacity(slices.len());
        let mut running = [0.0; 12];
        for slice in slices {
            let mut counts = [0.0; 12];
            for note in &slice.notes {
                counts[pitch_class(note.tpc) as usize] += 1.0;
            }
            before.push(running);
            for (total, count) in running.iter_mut().zip(counts) {
                *total += count * slice.duration;
            }
            voices.push(counts);
        }
        Sounding { starts: slices.iter().map(|s| s.start).collect(), before, voices }
    }

    /// Returns how much of the span from `start` to `end` the chord's pitch classes fill.
    pub fn fit(&self, start: Offset, end: Offset, chord: PitchClassSet) -> Fit {
        let mut fit = Fit::default();
        for (pc, time) in self.durations(start, end).into_iter().enumerate() {
            fit.total += time;
            if chord & (1 << pc) != 0 {
                fit.chord_tones += time;
            }
        }
        fit
    }

    /// Returns how long each pitch class sounds from `start` to `end`, summed over voices.
    pub fn durations(&self, start: Offset, end: Offset) -> [f64; 12] {
        let (from, to) = (self.cumulative(start), self.cumulative(end));
        std::array::from_fn(|pc| (to[pc] - from[pc]).max(0.0))
    }

    fn cumulative(&self, time: Offset) -> [f64; 12] {
        let index = self.starts.partition_point(|&s| s <= time).saturating_sub(1);
        let Some(before) = self.before.get(index) else {
            return [0.0; 12];
        };
        let elapsed = (time - self.starts[index]).max(0.0);
        let mut result = *before;
        for (total, count) in result.iter_mut().zip(self.voices[index]) {
            *total += count * elapsed;
        }
        result
    }
}

/// Returns the set of pitch classes of spelled pitches.
pub fn pitch_class_set(tpcs: impl IntoIterator<Item = crate::pitch::Tpc>) -> PitchClassSet {
    tpcs.into_iter().fold(0, |set, tpc| set | 1 << pitch_class(tpc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::model::{Measure, Note, Score};
    use crate::score::slicing::slice_score;

    fn sounding(notes: &[(f64, f64, i32)]) -> Sounding {
        let note = |&(onset, duration, tpc): &(f64, f64, i32)| Note {
            onset,
            duration,
            tpc,
            midi: 60,
            part: 0,
            tied_from_previous: false,
            has_fermata: false,
        };
        let measure = Measure { number: "1".into(), start: 0.0, duration: 4.0, beats: 4, beat_type: 4 };
        let score =
            Score { notes: notes.iter().map(note).collect(), measures: vec![measure], ..Score::default() };
        Sounding::new(&slice_score(&score))
    }

    #[test]
    fn chord_tones_share_is_weighted_by_duration() {
        let sounding = sounding(&[(0.0, 3.0, 0), (3.0, 1.0, 2)]);
        assert_eq!(sounding.fit(0.0, 4.0, pitch_class_set([0])).share(), Some(0.75));
    }

    #[test]
    fn partial_span_counts_only_its_part_of_a_note() {
        let sounding = sounding(&[(0.0, 4.0, 0), (0.0, 4.0, 1)]);
        assert_eq!(sounding.fit(1.0, 2.0, pitch_class_set([0])), Fit { chord_tones: 1.0, total: 2.0 });
    }

    #[test]
    fn silent_span_has_no_share() {
        let sounding = sounding(&[(0.0, 1.0, 0)]);
        assert_eq!(sounding.fit(2.0, 3.0, pitch_class_set([0])).share(), None);
    }
}
