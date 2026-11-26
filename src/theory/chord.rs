//! Chord qualities as spelled templates, and chord-member lookups.

use crate::pitch::{Tpc, pitch_class};
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub enum Quality {
    Major,
    Minor,
    Diminished,
    Augmented,
    DominantSeventh,
    MinorSeventh,
    MajorSeventh,
    DiminishedSeventh,
    HalfDiminishedSeventh,
    MinorMajorSeventh,
    ItalianSixth,
    FrenchSixth,
    GermanSixth,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub struct Chord {
    pub root: Tpc,
    pub quality: Quality,
}

/// All qualities with a template, in the order used when classifying pitch sets.
pub const QUALITIES: [Quality; 13] = [
    Quality::Major,
    Quality::Minor,
    Quality::Diminished,
    Quality::Augmented,
    Quality::DominantSeventh,
    Quality::MinorSeventh,
    Quality::MajorSeventh,
    Quality::DiminishedSeventh,
    Quality::HalfDiminishedSeventh,
    Quality::MinorMajorSeventh,
    Quality::ItalianSixth,
    Quality::FrenchSixth,
    Quality::GermanSixth,
];

/// Intervals of the ninth, in fifths above the root, which are ignored when classifying.
const NINTHS: [i32; 3] = [2, -5, 9];

impl Quality {
    /// Chord members in fifths above the root, ordered root, third, fifth, seventh.
    /// Augmented sixth chords follow music21's rooting: the Italian and German chords are
    /// rooted on the raised fourth degree and the French chord on the second degree.
    pub fn template(self) -> &'static [i32] {
        match self {
            Quality::Major => &[0, 4, 1],
            Quality::Minor => &[0, -3, 1],
            Quality::Diminished => &[0, -3, -6],
            Quality::Augmented => &[0, 4, 8],
            Quality::DominantSeventh => &[0, 4, 1, -2],
            Quality::MinorSeventh => &[0, -3, 1, -2],
            Quality::MajorSeventh => &[0, 4, 1, 5],
            Quality::DiminishedSeventh => &[0, -3, -6, -9],
            Quality::HalfDiminishedSeventh => &[0, -3, -6, -2],
            Quality::MinorMajorSeventh => &[0, -3, 1, 5],
            Quality::ItalianSixth => &[0, -10, -6],
            Quality::FrenchSixth => &[0, 4, -6, -2],
            Quality::GermanSixth => &[0, -10, -6, -9],
            Quality::Other => &[0],
        }
    }

    /// Returns a short label in the style of AugmentedNet's quality vocabulary.
    pub fn label(self) -> &'static str {
        match self {
            Quality::Major => "M",
            Quality::Minor => "m",
            Quality::Diminished => "d",
            Quality::Augmented => "a",
            Quality::DominantSeventh => "Mm7",
            Quality::MinorSeventh => "mm7",
            Quality::MajorSeventh => "MM7",
            Quality::DiminishedSeventh => "dd7",
            Quality::HalfDiminishedSeventh => "dm7",
            Quality::MinorMajorSeventh => "mM7",
            Quality::ItalianSixth => "It+6",
            Quality::FrenchSixth => "Fr+6",
            Quality::GermanSixth => "Ger+6",
            Quality::Other => "other",
        }
    }

    pub fn is_seventh(self) -> bool {
        self.template().len() == 4
    }
}

impl Chord {
    pub fn new(root: Tpc, quality: Quality) -> Chord {
        Chord { root, quality }
    }

    /// Returns the chord's spelled members, ordered root, third, fifth, seventh.
    pub fn members(self) -> impl Iterator<Item = Tpc> {
        self.quality.template().iter().map(move |interval| self.root + interval)
    }

    /// Returns the index of the member spelled exactly like the pitch.
    pub fn member_index(self, tpc: Tpc) -> Option<usize> {
        self.members().position(|member| member == tpc)
    }

    /// Returns the index of a member with the same pitch class, ignoring spelling.
    pub fn member_index_enharmonic(self, tpc: Tpc) -> Option<usize> {
        let pc = pitch_class(tpc);
        self.members().position(|member| pitch_class(member) == pc)
    }

    /// Returns the inversion (0 = root position, 1 = first inversion, ...) for a bass pitch.
    pub fn inversion(self, bass: Tpc) -> u8 {
        self.member_index(bass).or_else(|| self.member_index_enharmonic(bass)).unwrap_or(0) as u8
    }
}

/// Classifies a spelled pitch set with a known root, as given in a Roman numeral analysis.
/// Ninths are ignored, and a missing fifth is tolerated.
/// Sets that match no template, even enharmonically, are classified as `Quality::Other`.
pub fn classify(root: Tpc, pitches: &[Tpc]) -> Quality {
    let intervals = sorted_unique(pitches.iter().map(|p| p - root).filter(|i| !NINTHS.contains(i)));
    let exact = |template: &[i32]| sorted_unique(template.iter().copied()) == intervals;
    let enharmonic = |template: &[i32]| same_pitch_classes(template, &intervals);
    let without_fifth =
        |template: &[i32]| template.len() == 4 && exact(&[template[0], template[1], template[3]]);
    let tests: [&TemplateTest; 3] = [&exact, &without_fifth, &enharmonic];
    tests
        .iter()
        .find_map(|test| QUALITIES.iter().find(|q| test(q.template())))
        .copied()
        .unwrap_or(Quality::Other)
}

type TemplateTest<'a> = dyn Fn(&[i32]) -> bool + 'a;

fn sorted_unique(values: impl Iterator<Item = i32>) -> Vec<i32> {
    let mut sorted: Vec<i32> = values.collect();
    sorted.sort();
    sorted.dedup();
    sorted
}

fn same_pitch_classes(template: &[i32], intervals: &[i32]) -> bool {
    let to_pcs = |set: &[i32]| {
        let mut pcs: Vec<u8> = set.iter().map(|&i| pitch_class(i)).collect();
        pcs.sort();
        pcs.dedup();
        pcs
    };
    to_pcs(template) == to_pcs(intervals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;

    fn tpcs(names: &[&str]) -> Vec<Tpc> {
        names.iter().map(|n| tpc_from_name(n).unwrap()).collect()
    }

    #[test]
    fn german_sixth_is_recognized_from_music21_rooting() {
        let root = tpc_from_name("F#").unwrap();
        assert_eq!(classify(root, &tpcs(&["A-", "C", "E-", "F#"])), Quality::GermanSixth);
    }

    #[test]
    fn dominant_ninth_is_classified_as_dominant_seventh() {
        let root = tpc_from_name("G").unwrap();
        assert_eq!(classify(root, &tpcs(&["G", "B", "D", "F", "A"])), Quality::DominantSeventh);
    }

    #[test]
    fn seventh_without_fifth_keeps_its_quality() {
        let root = tpc_from_name("G").unwrap();
        assert_eq!(classify(root, &tpcs(&["G", "B", "F"])), Quality::DominantSeventh);
    }

    #[test]
    fn unrecognized_set_is_other() {
        let root = tpc_from_name("C").unwrap();
        assert_eq!(classify(root, &tpcs(&["C", "D", "F#"])), Quality::Other);
    }

    #[test]
    fn first_inversion_is_detected_from_bass() {
        let chord = Chord::new(tpc_from_name("C").unwrap(), Quality::Major);
        assert_eq!(chord.inversion(tpc_from_name("E").unwrap()), 1);
    }

    #[test]
    fn enharmonic_bass_still_gives_inversion() {
        let chord = Chord::new(tpc_from_name("D").unwrap(), Quality::FrenchSixth);
        assert_eq!(chord.inversion(tpc_from_name("G#").unwrap()), 2);
    }
}
