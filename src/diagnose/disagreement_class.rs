//! Sorts the frames where two analysts disagree by what their labels differ on.

use super::chord_relation::{RootInterval, adds_seventh, root_interval};
use super::key_relation::is_tonicization_of;
use crate::theory::label::Label;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub enum DisagreementClass {
    /// A cadential six-four read as a tonic six-four by one analyst and as a dominant by the other.
    CadentialSixFour,
    TonicizationVsModulation,
    DifferentKeySameChord,
    /// The same key and chord, read as an applied chord by one analyst only.
    AppliedVsDiatonic,
    SeventhVsTriad,
    Inversion,
    RootAThirdApart,
    Other,
}

impl DisagreementClass {
    pub fn name(self) -> &'static str {
        match self {
            DisagreementClass::CadentialSixFour => "I64 vs V (cadential six-four)",
            DisagreementClass::TonicizationVsModulation => "tonicization vs modulation",
            DisagreementClass::DifferentKeySameChord => "different key, same chord",
            DisagreementClass::AppliedVsDiatonic => "applied vs diatonic",
            DisagreementClass::SeventhVsTriad => "seventh vs triad",
            DisagreementClass::Inversion => "same chord, different inversion",
            DisagreementClass::RootAThirdApart => "roots a third apart",
            DisagreementClass::Other => "other",
        }
    }
}

/// Classifies how two analysts' differing labels differ, testing the classes in declaration order.
pub fn classify_disagreement(a: &Label, b: &Label) -> DisagreementClass {
    let same_chord = a.root == b.root && a.quality == b.quality && a.inversion == b.inversion;
    if is_cadential_pair(a, b) || is_cadential_pair(b, a) {
        DisagreementClass::CadentialSixFour
    } else if is_tonicization_of(a, b) || is_tonicization_of(b, a) {
        DisagreementClass::TonicizationVsModulation
    } else if a.key != b.key && same_chord {
        DisagreementClass::DifferentKeySameChord
    } else if same_chord {
        DisagreementClass::AppliedVsDiatonic
    } else if a.root == b.root && (adds_seventh(a.quality, b.quality) || adds_seventh(b.quality, a.quality)) {
        DisagreementClass::SeventhVsTriad
    } else if a.root == b.root && a.quality == b.quality {
        DisagreementClass::Inversion
    } else if root_interval(a, b) == RootInterval::Third {
        DisagreementClass::RootAThirdApart
    } else {
        DisagreementClass::Other
    }
}

/// Reports whether `six_four` is a tonic triad over its fifth and `dominant` a dominant chord in the
/// same key, the two usual readings of a cadential six-four.
fn is_cadential_pair(six_four: &Label, dominant: &Label) -> bool {
    let tonic = six_four.key.tonic;
    let is_tonic_six_four =
        six_four.root == tonic && six_four.inversion == 2 && !six_four.quality.is_seventh();
    let is_dominant = dominant.root == tonic + 1 && dominant.secondary.is_none();
    six_four.key == dominant.key && six_four.secondary.is_none() && is_tonic_six_four && is_dominant
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn label(key: &str, root: &str, quality: Quality, bass: &str) -> Label {
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        Label::new(Key::parse(key, "major").unwrap(), None, chord, tpc_from_name(bass).unwrap())
    }

    fn classify(a: Label, b: Label) -> DisagreementClass {
        classify_disagreement(&a, &b)
    }

    #[test]
    fn tonic_six_four_against_dominant_is_cadential() {
        let class = classify(label("C", "C", Quality::Major, "G"), label("C", "G", Quality::Major, "G"));
        assert_eq!(class, DisagreementClass::CadentialSixFour);
    }

    #[test]
    fn applied_dominant_against_dominant_of_new_key_is_tonicization() {
        let c = Key::parse("C", "major").unwrap();
        let chord = Chord::new(tpc_from_name("D").unwrap(), Quality::Major);
        let applied = Label::new(c, Key::parse("G", "major"), chord, chord.root);
        let class = classify(applied, label("G", "D", Quality::Major, "D"));
        assert_eq!(class, DisagreementClass::TonicizationVsModulation);
    }

    #[test]
    fn same_chord_in_different_keys_is_a_key_difference() {
        let class = classify(label("C", "F", Quality::Major, "F"), label("F", "F", Quality::Major, "F"));
        assert_eq!(class, DisagreementClass::DifferentKeySameChord);
    }

    #[test]
    fn dominant_seventh_against_dominant_triad_is_seventh_vs_triad() {
        let class =
            classify(label("C", "G", Quality::DominantSeventh, "G"), label("C", "G", Quality::Major, "G"));
        assert_eq!(class, DisagreementClass::SeventhVsTriad);
    }

    #[test]
    fn same_chord_over_another_bass_is_an_inversion_difference() {
        let class = classify(label("C", "F", Quality::Major, "A"), label("C", "F", Quality::Major, "F"));
        assert_eq!(class, DisagreementClass::Inversion);
    }

    #[test]
    fn dominant_against_leading_tone_chord_is_roots_a_third_apart() {
        let class = classify(label("C", "G", Quality::Major, "G"), label("C", "B", Quality::Diminished, "B"));
        assert_eq!(class, DisagreementClass::RootAThirdApart);
    }

    #[test]
    fn subdominant_against_dominant_is_other() {
        let class = classify(label("C", "F", Quality::Major, "F"), label("C", "G", Quality::Major, "G"));
        assert_eq!(class, DisagreementClass::Other);
    }
}
