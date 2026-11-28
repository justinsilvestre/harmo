//! Cuts a score into "salami slices": maximal spans during which no note starts or stops.

use super::meter::metric_level;
use super::model::{KeySignature, Measure, Note, Offset, Score};
use crate::pitch::Tpc;
use serde::Serialize;

const EPSILON: f64 = 1e-6;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SliceNote {
    pub tpc: Tpc,
    pub midi: i32,
    /// True when the note starts at the beginning of the slice.
    pub attacked: bool,
    /// The metric level at the note's onset, which may precede the slice.
    pub onset_level: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct Slice {
    pub start: Offset,
    pub duration: Offset,
    /// Sounding notes, lowest first.
    pub notes: Vec<SliceNote>,
    pub measure: usize,
    pub metric_level: u8,
    /// The number of fermatas that end at or before the slice's start,
    /// which numbers the phrases of music that marks its phrase endings with fermatas,
    /// such as chorales.
    pub phrase: usize,
    /// The key signature in force at the slice's start.
    pub key_signature: Option<KeySignature>,
}

impl Slice {
    pub fn bass(&self) -> Option<&SliceNote> {
        self.notes.first()
    }

    /// Returns whether every sounding note has the same spelled pitch class,
    /// as in a single line or a passage in unison or octaves.
    pub fn is_single_line(&self) -> bool {
        self.notes.first().is_some_and(|first| self.notes.iter().all(|n| n.tpc == first.tpc))
    }

    /// Returns whether the slice sounds at least `notes` notes and every one of them starts with it.
    pub fn is_struck_together(&self, notes: usize) -> bool {
        self.notes.len() >= notes && self.notes.iter().all(|note| note.attacked)
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
    let notes = annotate_notes(score);
    for window in times.windows(2) {
        let (start, end) = (window[0], window[1]);
        while next < order.len() && score.notes[order[next]].onset < end - EPSILON {
            active.push(order[next]);
            next += 1;
        }
        active.retain(|&i| score.notes[i].onset + score.notes[i].duration > start + EPSILON);
        slices.push(make_slice(score, &notes, &active, window));
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

/// Returns each note as it sounds in the slice it starts, with `attacked` false for tied notes.
fn annotate_notes(score: &Score) -> Vec<SliceNote> {
    let annotate = |n: &Note| SliceNote {
        tpc: n.tpc,
        midi: n.midi,
        attacked: !n.tied_from_previous,
        onset_level: level_at(&score.measures, n.onset),
    };
    score.notes.iter().map(annotate).collect()
}

fn make_slice(score: &Score, annotated: &[SliceNote], active: &[usize], window: &[Offset]) -> Slice {
    let (start, end) = (window[0], window[1]);
    let starts_here = |i: usize| (score.notes[i].onset - start).abs() < EPSILON;
    let mut notes: Vec<SliceNote> = active
        .iter()
        .map(|&i| SliceNote { attacked: annotated[i].attacked && starts_here(i), ..annotated[i] })
        .collect();
    notes.sort_by_key(|n| n.midi);
    let measure = measure_at(&score.measures, start);
    let metric_level = level_at(&score.measures, start);
    let phrase = score.fermata_ends.partition_point(|&t| t <= start + EPSILON);
    let signatures_begun = score.key_signatures.partition_point(|k| k.start <= start + EPSILON);
    let key_signature = signatures_begun.checked_sub(1).map(|i| score.key_signatures[i]);
    Slice { start, duration: end - start, notes, measure, metric_level, phrase, key_signature }
}

fn level_at(measures: &[Measure], time: Offset) -> u8 {
    let measure = measure_at(measures, time);
    measures.get(measure).map_or(0, |m| metric_level(m, time - m.start))
}

/// Returns the index of the measure containing a time.
pub fn measure_at(measures: &[Measure], time: Offset) -> usize {
    measures.partition_point(|m| m.start <= time + EPSILON).saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::model::Measure;

    fn slice_of(names: &[&str]) -> Slice {
        let note = |name: &&str| SliceNote {
            tpc: crate::pitch::tpc_from_name(name).unwrap(),
            midi: 60,
            attacked: true,
            onset_level: 0,
        };
        let notes = names.iter().map(note).collect();
        Slice {
            start: 0.0,
            duration: 1.0,
            notes,
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        }
    }

    #[test]
    fn notes_of_one_spelling_in_octaves_are_a_single_line() {
        assert!(slice_of(&["C", "C"]).is_single_line());
    }

    #[test]
    fn enharmonic_notes_are_not_a_single_line() {
        assert!(!slice_of(&["C#", "Db"]).is_single_line());
    }

    #[test]
    fn a_rest_is_not_a_single_line() {
        assert!(!slice_of(&[]).is_single_line());
    }

    fn note(onset: f64, duration: f64, midi: i32) -> Note {
        Note { onset, duration, tpc: 0, midi, part: 0, tied_from_previous: false, has_fermata: false }
    }

    fn score(notes: Vec<Note>) -> Score {
        let measure = Measure { number: "1".into(), start: 0.0, duration: 4.0, beats: 4, beat_type: 4 };
        Score { notes, measures: vec![measure], fermata_ends: vec![], key_signatures: vec![] }
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
    fn held_notes_keep_their_onset_level() {
        let slices = slice_score(&score(vec![note(1.0, 3.0, 60), note(2.0, 2.0, 64)]));
        assert_eq!(slices[1].notes[0].onset_level, 2);
    }

    #[test]
    fn slices_after_a_fermata_begin_a_new_phrase() {
        let score = Score { fermata_ends: vec![2.0], ..score(vec![note(0.0, 2.0, 60), note(2.0, 2.0, 62)]) };
        assert_eq!(slice_score(&score)[1].phrase, 1);
    }

    #[test]
    fn slices_carry_the_key_signature_in_force() {
        let signature = KeySignature { start: 2.0, fifths: 1 };
        let score =
            Score { key_signatures: vec![signature], ..score(vec![note(0.0, 2.0, 60), note(2.0, 2.0, 62)]) };
        assert_eq!(slice_score(&score)[1].key_signature, Some(signature));
    }

    #[test]
    fn slices_before_the_first_key_signature_have_none() {
        let signature = KeySignature { start: 2.0, fifths: 1 };
        let score =
            Score { key_signatures: vec![signature], ..score(vec![note(0.0, 2.0, 60), note(2.0, 2.0, 62)]) };
        assert_eq!(slice_score(&score)[0].key_signature, None);
    }

    #[test]
    fn slice_notes_are_ordered_from_the_bass() {
        let slices = slice_score(&score(vec![note(0.0, 4.0, 67), note(0.0, 4.0, 48)]));
        assert_eq!(slices[0].bass().map(|n| n.midi), Some(48));
    }
}
