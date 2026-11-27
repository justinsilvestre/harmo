//! The equivalents RN measure: a full Roman numeral match that also accepts the engine's label when
//! it differs from the expert's only by a notational convention.
//!
//! Each equivalence is applied against the expert's label as given, without chaining one onto
//! another, except that the timing tolerance also applies the other equivalences to nearby frames.

use super::equivalence_pedal::{Pedal, find_pedals};
use super::equivalence_rules::{
    is_applied_in_other_key, is_rootless_dominant, is_same_but_inversion, is_triad_and_its_seventh,
};
use super::frames::label_runs;
use super::metrics::is_rn_match;
use crate::pitch::pitch_class;
use crate::score::slicing::Slice;
use crate::theory::label::Label;
use serde::Serialize;

/// How a frame's label came to match, the equivalences in the order they are tried.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Credit {
    Exact,
    /// The expert labels a triad and then its seventh chord, or the reverse, and the engine gives
    /// either label anywhere in the pair. Whether a seventh that enters partway through a chord
    /// makes a new chord is a matter of analytical judgment (Kostka, Payne and Almén 2023, ch. 15).
    Seventh,
    /// The expert relabels one chord with another inversion as its bass moves, such as I6 then I,
    /// and the engine gives either label anywhere over the pair. The two labels name the same chord.
    Inversion,
    /// An applied chord X/Y against X in the key that Y stands for (Kostka, Payne and Almén 2023,
    /// ch. 16, on tonicization against modulation).
    Applied,
    /// A dominant seventh without its root against the leading-tone chord (Burstein and Straus
    /// 2016, ch. 13; Kostka, Payne and Almén 2023, ch. 7).
    Rootless,
    /// Over a held bass, the chord above it regardless of inversion, or the chord the bass belongs to
    /// (Kostka, Payne and Almén 2023, ch. 13).
    Pedal,
    /// The expert's label, or an equivalent of it, a little earlier or later. No source gives the
    /// tolerance; it is a measurement convention.
    Timing,
}

/// The equivalences that credit a frame the exact measure does not, in the order they are tried.
pub const EQUIVALENCES: [Credit; 6] =
    [Credit::Seventh, Credit::Inversion, Credit::Applied, Credit::Rootless, Credit::Pedal, Credit::Timing];

impl Credit {
    pub fn name(self) -> &'static str {
        match self {
            Credit::Exact => "exact",
            Credit::Seventh => "seventh",
            Credit::Inversion => "inversion",
            Credit::Applied => "applied",
            Credit::Rootless => "rootless",
            Credit::Pedal => "pedal",
            Credit::Timing => "timing",
        }
    }
}

/// What sounds at each frame of a piece.
pub struct FrameSounds {
    /// The pitch classes sounding, one bit each.
    pub sounding: Vec<u16>,
    /// The pitch class of the lowest note sounding.
    pub lowest: Vec<Option<u8>>,
}

impl FrameSounds {
    /// Reads the sounds at each frame from the slice sounding there.
    pub fn new(slices: &[Slice], frame_slices: &[usize]) -> FrameSounds {
        let sounding_in =
            |slice: &Slice| slice.notes.iter().fold(0, |mask, n| mask | 1 << pitch_class(n.tpc));
        let at = |&slice: &usize| slices.get(slice);
        FrameSounds {
            sounding: frame_slices.iter().map(|s| at(s).map_or(0, sounding_in)).collect(),
            lowest: frame_slices
                .iter()
                .map(|s| at(s).and_then(Slice::bass).map(|n| pitch_class(n.tpc)))
                .collect(),
        }
    }
}

/// What one expert analysis implies about equivalent labels, worked out once per piece.
pub struct ExpertEquivalents {
    /// The run of equal labels that each frame lies in.
    run_of: Vec<u32>,
    /// For each run, the labels of the neighboring runs that pair with it as a triad and its seventh.
    partners: Vec<[Option<Label>; 2]>,
    /// For each run, the labels of the neighboring runs that name the same chord in another inversion.
    inversions: Vec<[Option<Label>; 2]>,
    /// For each frame, the index into `pedals` of the pedal it lies over, if any.
    pedal_of: Vec<u32>,
    pedals: Vec<Pedal>,
}

impl ExpertEquivalents {
    pub fn new(labels: &[Option<Label>], sounds: &FrameSounds) -> ExpertEquivalents {
        let runs = label_runs(labels);
        let mut run_of = vec![0; labels.len()];
        for (index, &(start, end)) in runs.iter().enumerate() {
            run_of[start..end].fill(index as u32);
        }
        let (pedal_of, pedals) = find_pedals(labels, &sounds.lowest);
        ExpertEquivalents {
            run_of,
            partners: neighbor_partners(labels, &runs, is_triad_and_its_seventh),
            inversions: neighbor_partners(labels, &runs, is_same_but_inversion),
            pedal_of,
            pedals,
        }
    }
}

/// One expert analysis of a piece with what is needed to judge equivalent labels against it.
pub struct ExpertView<'a> {
    pub labels: &'a [Option<Label>],
    pub equivalents: &'a ExpertEquivalents,
    pub sounds: &'a FrameSounds,
    /// How many frames away the timing tolerance looks.
    pub reach: usize,
}

impl ExpertView<'_> {
    /// Returns how the label matches the expert's at the frame, or `None` if it does not.
    pub fn credit(&self, label: &Label, frame: usize) -> Option<Credit> {
        let truth = self.labels.get(frame).copied().flatten()?;
        if is_rn_match(label, &truth) {
            return Some(Credit::Exact);
        }
        if let Some(credit) = self.equivalent_at(label, frame, frame) {
            return Some(credit);
        }
        let nearby = frame.saturating_sub(self.reach)..(frame + self.reach + 1).min(self.labels.len());
        let matches = |&near: &usize| {
            self.labels[near].is_some_and(|t| is_rn_match(label, &t))
                || self.equivalent_at(label, near, frame).is_some()
        };
        nearby.filter(|&near| near != frame).any(|near| matches(&near)).then_some(Credit::Timing)
    }

    /// Returns the equivalence, other than timing, between the label and the expert's label at
    /// frame `expert`, judging what sounds from frame `heard`.
    fn equivalent_at(&self, label: &Label, expert: usize, heard: usize) -> Option<Credit> {
        let truth = self.labels[expert]?;
        let equivalents = self.equivalents;
        let run = equivalents.run_of[expert] as usize;
        let is_partner =
            |partners: &[Option<Label>; 2]| partners.iter().flatten().any(|p| is_rn_match(label, p));
        if is_partner(&equivalents.partners[run]) {
            return Some(Credit::Seventh);
        }
        if is_partner(&equivalents.inversions[run]) {
            return Some(Credit::Inversion);
        }
        if is_applied_in_other_key(label, &truth) {
            return Some(Credit::Applied);
        }
        if is_rootless_dominant(label, &truth, self.sounds.sounding[heard]) {
            return Some(Credit::Rootless);
        }
        let pedal = equivalents.pedals.get(equivalents.pedal_of[expert] as usize);
        pedal.is_some_and(|p| p.credits(label, &truth)).then_some(Credit::Pedal)
    }
}

/// Returns, for each run of equal labels, the neighboring runs' labels that pair with it by `pairs`.
fn neighbor_partners(
    labels: &[Option<Label>],
    runs: &[(usize, usize)],
    pairs: fn(&Label, &Label) -> bool,
) -> Vec<[Option<Label>; 2]> {
    let label_of = |run: Option<&(usize, usize)>| run.and_then(|&(start, _)| labels[start]);
    let partner = |own: Option<Label>, other: Option<Label>| {
        other.filter(|o| own.is_some_and(|own| own != *o && pairs(&own, o)))
    };
    let pair = |index: usize| {
        let own = label_of(runs.get(index));
        let before = index.checked_sub(1).and_then(|i| runs.get(i));
        [partner(own, label_of(before)), partner(own, label_of(runs.get(index + 1)))]
    };
    (0..runs.len()).map(pair).collect()
}

#[cfg(test)]
#[path = "equivalence_tests.rs"]
mod tests;
