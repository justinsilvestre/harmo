//! How two chords with different roots or qualities relate.

use crate::pitch::pitch_class;
use crate::theory::chord::Quality;
use crate::theory::label::Label;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub enum RootInterval {
    Third,
    Fifth,
    Step,
    /// A tritone, or two spellings of the same pitch.
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub enum QualityChange {
    /// The engine adds a seventh to the expert's triad.
    ExtraSeventh,
    /// The engine reads a triad where the expert hears a seventh chord built on it.
    MissingSeventh,
    Other,
}

impl RootInterval {
    pub fn name(self) -> &'static str {
        match self {
            RootInterval::Third => "third",
            RootInterval::Fifth => "fifth",
            RootInterval::Step => "step",
            RootInterval::Other => "other",
        }
    }
}

impl QualityChange {
    pub fn name(self) -> &'static str {
        match self {
            QualityChange::ExtraSeventh => "extra seventh",
            QualityChange::MissingSeventh => "missing seventh",
            QualityChange::Other => "other",
        }
    }
}

/// Classifies the interval between two roots by pitch class, in either direction.
pub fn root_interval(a: &Label, b: &Label) -> RootInterval {
    match pitch_class(b.root - a.root) {
        3 | 4 | 8 | 9 => RootInterval::Third,
        5 | 7 => RootInterval::Fifth,
        1 | 2 | 10 | 11 => RootInterval::Step,
        _ => RootInterval::Other,
    }
}

/// Classifies how the engine's chord quality differs from the expert's.
pub fn quality_change(expert: Quality, engine: Quality) -> QualityChange {
    if adds_seventh(expert, engine) {
        QualityChange::ExtraSeventh
    } else if adds_seventh(engine, expert) {
        QualityChange::MissingSeventh
    } else {
        QualityChange::Other
    }
}

/// Reports whether `seventh` is a seventh chord built on the triad `triad`, as V7 is on V.
pub fn adds_seventh(triad: Quality, seventh: Quality) -> bool {
    !triad.is_seventh() && seventh.is_seventh() && seventh.template()[..3] == *triad.template()
}

/// Formats a label as its numeral in root position, such as "V7/V" or "viio".
pub fn root_position_figure(label: &Label) -> String {
    Label { inversion: 0, ..*label }.figure()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    fn label(root: &str, quality: Quality, bass: &str) -> Label {
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        Label::new(Key::parse("C", "major").unwrap(), None, chord, tpc_from_name(bass).unwrap())
    }

    fn triad(root: &str) -> Label {
        label(root, Quality::Major, root)
    }

    #[test]
    fn g_and_b_are_a_third_apart() {
        assert_eq!(root_interval(&triad("G"), &triad("B")), RootInterval::Third);
    }

    #[test]
    fn c_and_f_are_a_fifth_apart() {
        assert_eq!(root_interval(&triad("C"), &triad("F")), RootInterval::Fifth);
    }

    #[test]
    fn b_and_c_are_a_step_apart() {
        assert_eq!(root_interval(&triad("B"), &triad("C")), RootInterval::Step);
    }

    #[test]
    fn c_and_f_sharp_are_another_interval() {
        assert_eq!(root_interval(&triad("C"), &triad("F#")), RootInterval::Other);
    }

    #[test]
    fn dominant_seventh_for_major_triad_is_an_extra_seventh() {
        assert_eq!(quality_change(Quality::Major, Quality::DominantSeventh), QualityChange::ExtraSeventh);
    }

    #[test]
    fn diminished_triad_for_half_diminished_seventh_is_a_missing_seventh() {
        let change = quality_change(Quality::HalfDiminishedSeventh, Quality::Diminished);
        assert_eq!(change, QualityChange::MissingSeventh);
    }

    #[test]
    fn minor_seventh_for_major_triad_is_another_change() {
        assert_eq!(quality_change(Quality::Major, Quality::MinorSeventh), QualityChange::Other);
    }

    #[test]
    fn root_position_figure_drops_the_inversion() {
        assert_eq!(root_position_figure(&label("G", Quality::DominantSeventh, "B")), "V7");
    }
}
