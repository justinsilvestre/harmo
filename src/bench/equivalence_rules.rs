//! The tests that decide whether two Roman numeral labels are notational equivalents.
//!
//! Each test compares the engine's label with one expert label. None of them credits a genuinely
//! different reading, such as a cadential six-four read as V, or a relative or parallel key.

use crate::pitch::pitch_class;
use crate::theory::chord::Quality;
use crate::theory::label::Label;

/// Reports whether two labels name a triad and its own seventh chord on the same root, in the same
/// key, with the same tonicization and the same bass note, such as V and V7.
pub fn is_triad_and_its_seventh(a: &Label, b: &Label) -> bool {
    let is_pair = |triad: &Label, seventh: &Label| seventh.quality.triad() == Some(triad.quality);
    let same_chord_place = a.key == b.key && a.tonicized == b.tonicized && a.root == b.root;
    same_chord_place && (is_pair(a, b) || is_pair(b, a)) && pitch_class(a.bass()) == pitch_class(b.bass())
}

/// Reports whether an applied chord X/Y in one key names the same chord as the other label in the
/// key that Y stands for, such as V/V in C and V in G.
/// At least one of the labels must be applied, so two plain labels in different keys never match.
/// A label applied to its own tonic in the other mode, such as VI/i in a major key, is how some
/// analysts write mixture, and matching it would credit a reading in the parallel key, so it is
/// left out.
pub fn is_applied_in_other_key(a: &Label, b: &Label) -> bool {
    let is_applied = |l: &Label| l.tonicized.is_some_and(|t| t.tonic != l.key.tonic);
    let is_applied = (is_applied(a) || is_applied(b)) && !is_parallel_mixture(a) && !is_parallel_mixture(b);
    let same_chord = a.root == b.root && a.quality == b.quality && a.inversion == b.inversion;
    is_applied && same_chord && a.reference_key() == b.reference_key()
}

/// Reports whether one label is a dominant seventh on the fifth degree and the other a leading-tone
/// chord of the same key and tonicization, while the dominant's root is not sounding (`sounding`
/// holds one bit per pitch class). The two must share a bass, unless the dominant seventh is
/// written in root position: with its root silent, that figure names the implied root rather than
/// a sounding bass.
/// With the root absent, the two name the same sounding pitches.
pub fn is_rootless_dominant(a: &Label, b: &Label, sounding: u16) -> bool {
    let (dominant, leading) = match (is_dominant_seventh(a), is_dominant_seventh(b)) {
        (true, false) => (a, b),
        (false, true) => (b, a),
        _ => return false,
    };
    let same_place = a.key == b.key && a.tonicized == b.tonicized;
    let root_is_silent = sounding & (1 << pitch_class(dominant.root)) == 0;
    same_place
        && is_leading_tone_chord(leading)
        && (pitch_class(a.bass()) == pitch_class(b.bass()) || dominant.bass() == dominant.root)
        && root_is_silent
}

fn is_parallel_mixture(label: &Label) -> bool {
    label.tonicized.is_some_and(|t| t.tonic == label.key.tonic && t.mode != label.key.mode)
}

fn is_dominant_seventh(label: &Label) -> bool {
    label.quality == Quality::DominantSeventh && label.root == label.reference_key().tonic + 1
}

/// Reports whether the label is viio, viio7 or viiø7: a diminished chord on the leading tone, which
/// lies five fifths above the tonic in either mode.
fn is_leading_tone_chord(label: &Label) -> bool {
    let is_diminished = matches!(
        label.quality,
        Quality::Diminished | Quality::DiminishedSeventh | Quality::HalfDiminishedSeventh
    );
    is_diminished && label.root == label.reference_key().tonic + 5
}

/// Reports whether two labels agree on everything but the inversion.
pub fn is_same_but_inversion(a: &Label, b: &Label) -> bool {
    Label { inversion: 0, ..*a } == Label { inversion: 0, ..*b }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Tpc, tpc_from_name};
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    fn tpc(name: &str) -> Tpc {
        tpc_from_name(name).unwrap()
    }

    fn key(name: &str) -> Key {
        let mode = if name.chars().next().unwrap().is_uppercase() { "major" } else { "minor" };
        Key::parse(&name.to_uppercase(), mode).unwrap()
    }

    /// A label in `key`, tonicizing `tonicized` if given, of the chord on `root` with `bass`.
    fn label(key_name: &str, tonicized: Option<&str>, chord: (&str, Quality), bass: &str) -> Label {
        let chord = Chord::new(tpc(chord.0), chord.1);
        Label::new(key(key_name), tonicized.map(key), chord, tpc(bass))
    }

    fn pitch_classes(names: &[&str]) -> u16 {
        names.iter().fold(0, |mask, name| mask | 1 << pitch_class(tpc(name)))
    }

    const V: (&str, Quality) = ("G", Quality::Major);
    const V7: (&str, Quality) = ("G", Quality::DominantSeventh);
    const VIIO: (&str, Quality) = ("B", Quality::Diminished);

    #[test]
    fn a_triad_and_its_seventh_on_the_same_bass_pair() {
        assert!(is_triad_and_its_seventh(&label("C", None, V, "G"), &label("C", None, V7, "G")));
    }

    #[test]
    fn a_seventh_in_the_bass_does_not_pair_with_the_triad() {
        assert!(!is_triad_and_its_seventh(&label("C", None, V, "G"), &label("C", None, V7, "F")));
    }

    #[test]
    fn a_major_triad_does_not_pair_with_a_minor_seventh_chord() {
        let minor_seventh = ("G", Quality::MinorSeventh);
        assert!(!is_triad_and_its_seventh(&label("C", None, V, "G"), &label("C", None, minor_seventh, "G")));
    }

    #[test]
    fn a_triad_and_its_seventh_in_different_keys_do_not_pair() {
        assert!(!is_triad_and_its_seventh(&label("C", None, V, "G"), &label("G", None, V7, "G")));
    }

    #[test]
    fn an_applied_dominant_matches_the_dominant_of_its_target_key() {
        let d = ("D", Quality::Major);
        assert!(is_applied_in_other_key(&label("C", Some("G"), d, "D"), &label("G", None, d, "D")));
    }

    #[test]
    fn an_applied_leading_tone_chord_matches_its_minor_target_key() {
        let viio7 = ("C#", Quality::DiminishedSeventh);
        assert!(is_applied_in_other_key(&label("C", Some("d"), viio7, "C#"), &label("d", None, viio7, "C#")));
    }

    #[test]
    fn an_applied_chord_does_not_match_the_target_key_of_the_other_mode() {
        let d = ("D", Quality::Major);
        assert!(!is_applied_in_other_key(&label("C", Some("G"), d, "D"), &label("g", None, d, "D")));
    }

    #[test]
    fn a_key_area_label_matches_the_tonic_of_its_key() {
        let i64_of_v = ("G", Quality::Major);
        assert!(is_applied_in_other_key(
            &label("C", Some("G"), i64_of_v, "D"),
            &label("G", None, i64_of_v, "D")
        ));
    }

    #[test]
    fn the_same_applied_chord_from_two_keys_matches() {
        let d = ("D", Quality::Major);
        assert!(is_applied_in_other_key(&label("C", Some("G"), d, "D"), &label("D", Some("G"), d, "D")));
    }

    #[test]
    fn mixture_written_as_applied_to_the_tonic_does_not_match_the_parallel_key() {
        let vi = ("E-", Quality::Major);
        assert!(!is_applied_in_other_key(&label("G", Some("g"), vi, "E-"), &label("g", None, vi, "E-")));
    }

    #[test]
    fn two_plain_labels_in_different_keys_do_not_match() {
        assert!(!is_applied_in_other_key(&label("C", None, V, "G"), &label("G", None, V, "G")));
    }

    #[test]
    fn an_applied_chord_needs_the_same_inversion() {
        let d = ("D", Quality::Major);
        assert!(!is_applied_in_other_key(&label("C", Some("G"), d, "F#"), &label("G", None, d, "D")));
    }

    #[test]
    fn a_rootless_dominant_seventh_matches_the_leading_tone_triad() {
        let sounding = pitch_classes(&["D", "F", "B"]);
        assert!(is_rootless_dominant(&label("C", None, V7, "D"), &label("C", None, VIIO, "D"), sounding));
    }

    #[test]
    fn a_sounding_dominant_root_rules_out_the_rootless_reading() {
        let sounding = pitch_classes(&["D", "F", "B", "G"]);
        assert!(!is_rootless_dominant(&label("C", None, V7, "D"), &label("C", None, VIIO, "D"), sounding));
    }

    #[test]
    fn a_rootless_dominant_needs_the_same_bass() {
        let sounding = pitch_classes(&["D", "F", "B"]);
        assert!(!is_rootless_dominant(&label("C", None, V7, "B"), &label("C", None, VIIO, "D"), sounding));
    }

    #[test]
    fn a_root_position_dominant_with_its_root_silent_matches_any_leading_tone_inversion() {
        let sounding = pitch_classes(&["D", "F", "B"]);
        assert!(is_rootless_dominant(&label("C", None, V7, "G"), &label("C", None, VIIO, "D"), sounding));
    }

    #[test]
    fn a_rootless_dominant_matches_a_diminished_seventh_in_minor() {
        let (v7, viio7) = (V7, ("B", Quality::DiminishedSeventh));
        let sounding = pitch_classes(&["B", "D", "F", "A-"]);
        assert!(is_rootless_dominant(&label("c", None, v7, "B"), &label("c", None, viio7, "B"), sounding));
    }

    #[test]
    fn a_rootless_dominant_needs_the_same_tonicization() {
        let sounding = pitch_classes(&["D", "F", "B"]);
        assert!(!is_rootless_dominant(
            &label("C", None, V7, "D"),
            &label("F", Some("C"), VIIO, "D"),
            sounding
        ));
    }

    #[test]
    fn labels_that_differ_only_in_inversion_are_the_same_but_inversion() {
        assert!(is_same_but_inversion(&label("C", None, V, "G"), &label("C", None, V, "B")));
    }

    #[test]
    fn labels_on_different_roots_are_not_the_same_but_inversion() {
        assert!(!is_same_but_inversion(&label("C", None, V, "G"), &label("C", None, VIIO, "B")));
    }
}
