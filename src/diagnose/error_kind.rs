//! The kinds of error a wrong frame can show, and how a wrong label is assigned to one.

use super::chord_relation::{QualityChange, RootInterval, quality_change, root_interval};
use super::key_relation::{KeyRelation, relate_keys};
use super::producibility::Gap;
use crate::theory::label::Label;

/// Why the engine's top reading differs from the expert's label at a frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorKind {
    /// The piece's analysts label the frame differently, so it is judged separately.
    Disagreement,
    NoReading,
    /// The engine has the expert's label, but its chord change falls up to a quarter note away.
    Timing,
    /// The engine has no way to produce the expert's label.
    Unproducible(Gap),
    Key {
        relation: KeyRelation,
        is_sustained: bool,
    },
    Root(RootInterval),
    /// The same key and chord, read as an applied chord by one side and not by the other.
    Function,
    Quality(QualityChange),
    Inversion,
}

impl ErrorKind {
    /// Returns the broad group the kind belongs to.
    pub fn group(self) -> &'static str {
        match self {
            ErrorKind::Disagreement => "disagreement",
            ErrorKind::NoReading => "no reading",
            ErrorKind::Timing => "timing",
            ErrorKind::Unproducible(_) => "unproducible",
            ErrorKind::Key { .. } => "key",
            ErrorKind::Root(_) => "root",
            ErrorKind::Function => "function",
            ErrorKind::Quality(_) => "quality",
            ErrorKind::Inversion => "inversion",
        }
    }

    /// Returns a name such as "key: relative, sustained" that identifies the kind.
    pub fn name(self) -> String {
        let detail = match self {
            ErrorKind::Unproducible(gap) => gap.name().to_string(),
            ErrorKind::Key { relation, is_sustained } => {
                format!("{}, {}", relation.name(), if is_sustained { "sustained" } else { "brief" })
            }
            ErrorKind::Root(interval) => interval.name().to_string(),
            ErrorKind::Quality(change) => change.name().to_string(),
            _ => return self.group().to_string(),
        };
        format!("{}: {detail}", self.group())
    }
}

/// Classifies how a wrong engine label differs from a producible expert label, checking the key
/// first, then the root, the function, the quality and the inversion.
/// A key error is sustained when the two keys stay apart for `is_sustained_key` frames.
pub fn classify_difference(expert: &Label, engine: &Label, is_sustained_key: bool) -> ErrorKind {
    if expert.key != engine.key {
        ErrorKind::Key { relation: relate_keys(expert, engine), is_sustained: is_sustained_key }
    } else if expert.root != engine.root {
        ErrorKind::Root(root_interval(expert, engine))
    } else if expert.secondary != engine.secondary {
        ErrorKind::Function
    } else if expert.quality != engine.quality {
        ErrorKind::Quality(quality_change(expert.quality, engine.quality))
    } else {
        ErrorKind::Inversion
    }
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

    fn dominant(bass: &str) -> Label {
        label("C", "G", Quality::Major, bass)
    }

    #[test]
    fn different_key_is_a_key_error() {
        let kind = classify_difference(&dominant("G"), &label("G", "G", Quality::Major, "G"), true);
        assert_eq!(kind.name(), "key: dominant, sustained");
    }

    #[test]
    fn leading_tone_chord_for_dominant_is_a_root_a_third_away() {
        let kind = classify_difference(&dominant("G"), &label("C", "B", Quality::Diminished, "B"), false);
        assert_eq!(kind, ErrorKind::Root(RootInterval::Third));
    }

    #[test]
    fn added_seventh_is_a_quality_error() {
        let kind =
            classify_difference(&dominant("G"), &label("C", "G", Quality::DominantSeventh, "G"), false);
        assert_eq!(kind, ErrorKind::Quality(QualityChange::ExtraSeventh));
    }

    #[test]
    fn different_bass_alone_is_an_inversion_error() {
        assert_eq!(classify_difference(&dominant("G"), &dominant("B"), false), ErrorKind::Inversion);
    }

    #[test]
    fn applied_reading_of_a_diatonic_chord_is_a_function_error() {
        let c = Key::parse("C", "major").unwrap();
        let chord = Chord::new(tpc_from_name("E").unwrap(), Quality::Major);
        let applied = Label::new(c, Key::parse("A", "minor"), chord, chord.root);
        let mixture = Label::new(c, None, chord, chord.root);
        assert_eq!(classify_difference(&applied, &mixture, false), ErrorKind::Function);
    }
}
