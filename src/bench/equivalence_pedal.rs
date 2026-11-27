//! Finds pedals: a bass note held or restruck while the harmony above it changes.
//!
//! Kostka, Payne and Almén (2023, ch. 13, "The Pedal Point") describe a pedal point as a tone that
//! begins as a chord tone, becomes a nonchord tone as the harmonies around it change, and ends as a
//! chord tone again, and advise leaving the inversions of the chords above it unmarked.

use super::equivalence_rules::is_same_but_inversion;
use super::frames::label_runs;
use super::metrics::is_rn_match;
use crate::pitch::pitch_class;
use crate::theory::label::Label;

/// Marks a frame that lies over no pedal.
pub const NO_PEDAL: u32 = u32::MAX;

/// A bass pitch class held or restruck through two or more of the expert's labels.
#[derive(Clone, Copy, Debug)]
pub struct Pedal {
    pub bass: u8,
    /// The expert's labels at the pedal's start and end, where the pedal note is their bass.
    pub chords: [Option<Label>; 2],
}

impl Pedal {
    /// Reports whether, over this pedal, the label counts as the expert's: it is one of the pedal's
    /// own chords, or it differs from the expert's label only in inversion where the expert names the
    /// chord above the pedal as if the pedal were absent.
    pub fn credits(&self, label: &Label, truth: &Label) -> bool {
        let is_above_pedal = pitch_class(truth.bass()) != self.bass;
        let is_pedal_chord = self.chords.iter().flatten().any(|c| is_rn_match(label, c));
        is_pedal_chord || is_above_pedal && is_same_but_inversion(label, truth)
    }
}

/// Finds the pedals: spans over which the lowest pitch class stays the same through two or more of
/// the expert's labels, where at least one of them leaves it out, so that the bass becomes a nonchord
/// tone, or where one chord over the bass returns after six-four chords over it, as in I–IV64–I.
/// Returns each frame's index into the pedals, and the pedals.
pub fn find_pedals(labels: &[Option<Label>], lowest: &[Option<u8>]) -> (Vec<u32>, Vec<Pedal>) {
    let mut pedal_of = vec![NO_PEDAL; labels.len()];
    let mut pedals = Vec::new();
    let mut start = 0;
    for frame in 1..=labels.len() {
        if frame < labels.len() && lowest[frame] == lowest[start] {
            continue;
        }
        if let Some(pedal) = lowest[start].and_then(|pc| pedal(&labels[start..frame], pc)) {
            pedal_of[start..frame].fill(pedals.len() as u32);
            pedals.push(pedal);
        }
        start = frame;
    }
    (pedal_of, pedals)
}

/// Returns the pedal over a span with a single lowest pitch class, if the span is one.
fn pedal(labels: &[Option<Label>], bass: u8) -> Option<Pedal> {
    let chords: Vec<Label> = label_runs(labels).iter().filter_map(|&(start, _)| labels[start]).collect();
    let has_bass = |label: &Label| label.chord().members().any(|m| pitch_class(m) == bass);
    let is_pedal = chords.len() >= 2 && !chords.iter().all(has_bass) || is_six_four_split(&chords, bass);
    let over_bass = |label: Option<&Label>| label.copied().filter(|l| pitch_class(l.bass()) == bass);
    is_pedal.then(|| Pedal { bass, chords: [over_bass(chords.first()), over_bass(chords.last())] })
}

/// Reports whether the labels over a held bass are a root-position chord, then one or more six-four
/// chords over its root, then the first chord again, as in I–(IV64)–I. Kostka, Payne and Almén
/// (2023, ch. 9, pp. 139–140) call such a chord a pedal six-four, which elaborates the chord around
/// it and is written in parentheses. Their other pedal six-four, V–(I64)–V, is left out, because a
/// tonic six-four over the dominant may also be a cadential six-four, and reading that as V is a
/// different reading rather than a matter of notation.
fn is_six_four_split(chords: &[Label], bass: u8) -> bool {
    let [first, inner @ .., last] = chords else { return false };
    let is_pedal_six_four = |label: &Label| {
        let is_tonic = label.root == label.reference_key().tonic;
        label.inversion == 2 && pitch_class(label.bass()) == bass && !is_tonic
    };
    let is_outer = first == last && pitch_class(first.bass()) == bass && first.inversion == 0;
    !inner.is_empty() && is_outer && inner.iter().all(is_pedal_six_four)
}
