//! Cuts a score into "salami slices": maximal spans during which no note starts or stops.

use super::meter::metric_level;
use super::model::{Measure, Offset, Score};
use crate::pitch::Tpc;
use serde::Serialize;

const EPSILON: f64 = 1e-6;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SliceNote {
    pub tpc: Tpc,
    pub midi: i32,
    /// True when the note starts at the beginning of the slice.
    pub attacked: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Slice {
    pub start: Offset,
    pub duration: Offset,
    /// Sounding notes, lowest first.
    pub notes: Vec<SliceNote>,
    pub measure: usize,
    pub metric_level: u8,
}

impl Slice {
    pub fn bass(&self) -> Option<&SliceNote> {
        self.notes.first()
    }
}

/// Slices the score at every note onset, note release and barline.
pub fn slice_score(score: &Score) -> Vec<Slice> {
    let times = breakpoints(score);
    let mut order: Vec<usize> = (0..score.notes.len()).collect();
    order.sort_by(|&a, &b| score.notes[a].onset.total_cmp(&score.notes[b].onset));
    let mut next = 0;
    let mut active: Vec<usize> = Vec::new();
    let mut slices = Vec::with_capacity(times.len());
    for window in times.windows(2) {
        let (start, end) = (window[0], window[1]);
        while next < order.len() && score.notes[order[next]].onset < end - EPSILON {
            active.push(order[next]);
            next += 1;
        }
        active.retain(|&i| score.notes[i].onset + score.notes[i].duration > start + EPSILON);
        slices.push(make_slice(score, &active, start, end));
    }
    slices
}

fn breakpoints(score: &Score) -> Vec<Offset> {
    let note_times = score.notes.iter().flat_map(|n| [n.onset, n.onset + n.duration]);
    let barlines = score.measures.iter().map(|m| m.start).chain([score.end()]);
    let mut times: Vec<Offset> = note_times.chain(barlines).filter(|&t| t <= score.end() + EPSILON).collect();
    times.sort_by(f64::total_cmp);
    times.dedup_by(|a, b| (*a - *b).abs() < EPSILON);
    times
}

fn make_slice(score: &Score, active: &[usize], start: Offset, end: Offset) -> Slice {
    let mut notes: Vec<SliceNote> = active
        .iter()
        .map(|&i| &score.notes[i])
        .map(|n| SliceNote {
            tpc: n.tpc,
            midi: n.midi,
            attacked: (n.onset - start).abs() < EPSILON && !n.tied_from_previous,
        })
        .collect();
    notes.sort_by_key(|n| n.midi);
    let measure = measure_at(&score.measures, start);
    let level = score.measures.get(measure).map_or(0, |m| metric_level(m, start - m.start));
    Slice { start, duration: end - start, notes, measure, metric_level: level }
}

/// Returns the index of the measure containing a time.
pub fn measure_at(measures: &[Measure], time: Offset) -> usize {
    measures.partition_point(|m| m.start <= time + EPSILON).saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::model::{Measure, Note};

    fn note(onset: f64, duration: f64, midi: i32) -> Note {
        Note { onset, duration, tpc: 0, midi, part: 0, tied_from_previous: false }
    }

    fn score(notes: Vec<Note>) -> Score {
        let measure = Measure { number: "1".into(), start: 0.0, duration: 4.0, beats: 4, beat_type: 4 };
        Score { notes, measures: vec![measure] }
    }

    #[test]
    fn overlapping_notes_produce_three_slices() {
        let slices = slice_score(&score(vec![note(0.0, 2.0, 60), note(1.0, 3.0, 64)]));
        assert_eq!(slices.len(), 3);
    }

    #[test]
    fn held_notes_are_not_reattacked() {
        let slices = slice_score(&score(vec![note(0.0, 4.0, 60), note(2.0, 2.0, 64)]));
        assert!(!slices[1].notes[0].attacked);
    }

    #[test]
    fn slice_notes_are_ordered_from_the_bass() {
        let slices = slice_score(&score(vec![note(0.0, 4.0, 67), note(0.0, 4.0, 48)]));
        assert_eq!(slices[0].bass().map(|n| n.midi), Some(48));
    }
}
