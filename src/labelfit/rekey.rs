//! Reads a Roman numeral label's figure in another key, as if the analysis had named that key instead.

use crate::pitch::Tpc;
use crate::theory::chord::Quality;
use crate::theory::key::{Degree, Key, Mode};
use crate::theory::label::Label;

/// Returns how far, in fifths, the chord's root moves when the label's figure is read in `target`.
///
/// - Augmented sixth chords keep their place relative to the tonic, since their figures name no degree.
/// - Applied chords move with the key they tonicize, whose degree is read in the target key.
/// - Other chords take the degree of their figure in the target key. In minor keys a figure on the
///   sixth or seventh degree is natural, except that diminished chords use the raised degree,
///   as in RomanText ("viio7" is built on the leading tone, "VII" on the subtonic).
pub fn root_motion(label: &Label, target: Key) -> Tpc {
    let transposition = target.tonic - label.key.tonic;
    if target.mode == label.key.mode || is_augmented_sixth(label.quality) {
        return transposition;
    }
    match (label.tonicized, label.secondary) {
        (Some(tonicized), Some(secondary)) => {
            let written = written_degree(label.key.mode, secondary);
            target.tpc_of(degree_in(target.mode, written, Quality::Major)) - tonicized.tonic
        }
        _ => {
            let written = written_degree(label.key.mode, label.degree);
            target.tpc_of(degree_in(target.mode, written, label.quality)) - label.root
        }
    }
}

fn is_augmented_sixth(quality: Quality) -> bool {
    matches!(quality, Quality::ItalianSixth | Quality::FrenchSixth | Quality::GermanSixth)
}

/// Returns the degree as a figure writes it, which in minor leaves the raised sixth and seventh unmarked.
fn written_degree(mode: Mode, degree: Degree) -> Degree {
    let is_raised_in_minor = mode == Mode::Minor && degree.alteration == 1 && degree.number >= 6;
    if is_raised_in_minor { Degree { alteration: 0, ..degree } } else { degree }
}

/// Returns the degree that a written figure stands for in a key of the given mode.
fn degree_in(mode: Mode, written: Degree, quality: Quality) -> Degree {
    let is_diminished =
        matches!(quality, Quality::Diminished | Quality::DiminishedSeventh | Quality::HalfDiminishedSeventh);
    let is_raised = mode == Mode::Minor && is_diminished && written.alteration == 0 && written.number >= 6;
    if is_raised { Degree { alteration: 1, ..written } } else { written }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Chord;

    fn tpc(name: &str) -> Tpc {
        tpc_from_name(name).unwrap()
    }

    fn key(name: &str, mode: &str) -> Key {
        Key::parse(name, mode).unwrap()
    }

    fn label(key: Key, tonicized: Option<Key>, root: &str, quality: Quality) -> Label {
        Label::new(key, tonicized, Chord::new(tpc(root), quality), tpc(root))
    }

    #[test]
    fn transposition_moves_every_root_alike() {
        let submediant = label(key("C", "minor"), None, "A-", Quality::Major);
        assert_eq!(root_motion(&submediant, key("G", "minor")), 1);
    }

    #[test]
    fn submediant_of_minor_read_in_major_is_raised() {
        let submediant = label(key("A", "minor"), None, "F", Quality::Major);
        assert_eq!(root_motion(&submediant, key("A", "major")), tpc("F#") - tpc("F"));
    }

    #[test]
    fn leading_tone_chord_of_major_read_in_minor_keeps_the_leading_tone() {
        let leading = label(key("C", "major"), None, "B", Quality::DiminishedSeventh);
        assert_eq!(root_motion(&leading, key("C", "minor")), 0);
    }

    #[test]
    fn subtonic_of_minor_read_in_major_is_the_leading_tone() {
        let subtonic = label(key("C", "minor"), None, "B-", Quality::Major);
        assert_eq!(root_motion(&subtonic, key("C", "major")), tpc("B") - tpc("B-"));
    }

    #[test]
    fn applied_chord_follows_the_degree_it_tonicizes() {
        let applied = label(key("C", "minor"), Some(key("E-", "major")), "B-", Quality::Major);
        assert_eq!(root_motion(&applied, key("C", "major")), tpc("E") - tpc("E-"));
    }
}
