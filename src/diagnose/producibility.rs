//! Decides whether the engine could produce a label at all, by asking the rule set's own
//! chord proposers and vocabulary.

use crate::engine::ruleset::RuleSet;
use crate::score::slicing::Slice;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::{Key, all_keys};
use crate::theory::label::Label;
use serde::Serialize;

/// Why the engine cannot produce a label.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub enum Gap {
    /// The expert's pitch set matches no chord template, for example a chord with an added sixth.
    UnknownQuality,
    /// The key is not among those the engine can represent.
    KeyOutOfRange,
    /// The vocabulary has no reading of the chord in the key.
    ChordNotInKey,
    /// The label applies the chord to another degree, but the engine reads it in the key itself.
    AppliedReadInKey,
    /// The label gives the chord a function other than dominant or leading tone in the tonicized
    /// key, such as ii/V or IV/IV.
    AppliedNonDominant,
    /// The label applies a dominant or leading-tone chord to a degree the engine does not tonicize.
    AppliedTargetMissing,
    /// The vocabulary allows the reading, but no rule proposes the chord, or no rule proposes the
    /// key, on any of the label's slices.
    NotProposed,
}

impl Gap {
    pub fn name(self) -> &'static str {
        match self {
            Gap::UnknownQuality => "quality outside the chord templates",
            Gap::KeyOutOfRange => "key outside the engine's keys",
            Gap::ChordNotInKey => "chord not in the key's vocabulary",
            Gap::AppliedReadInKey => "applied chord read in the key",
            Gap::AppliedNonDominant => "non-dominant applied function",
            Gap::AppliedTargetMissing => "target cannot be tonicized",
            Gap::NotProposed => "chord not proposed from the notes",
        }
    }
}

/// Returns why the engine cannot produce `label` on any of `slices`, or `None` if it can.
pub fn find_gap(rules: &RuleSet, label: &Label, slices: &[Slice]) -> Option<Gap> {
    if label.quality == Quality::Other {
        return Some(Gap::UnknownQuality);
    }
    if !all_keys().contains(&label.key) {
        return Some(Gap::KeyOutOfRange);
    }
    let chord = Chord::new(label.root, label.quality);
    vocabulary_gap(rules, label, chord).or_else(|| proposal_gap(rules, label, chord, slices))
}

fn vocabulary_gap(rules: &RuleSet, label: &Label, chord: Chord) -> Option<Gap> {
    let readings: Vec<_> =
        rules.interpreters.iter().flat_map(|interpret| interpret(label.key, chord)).collect();
    if readings.iter().any(|(tonicized, _)| has_same_numeral(label, *tonicized, chord)) {
        return None;
    }
    let is_dominant_function = matches!(label.degree.number, 5 | 7);
    Some(match label.tonicized {
        None => Gap::ChordNotInKey,
        Some(_) if readings.iter().any(|(tonicized, _)| tonicized.is_none()) => Gap::AppliedReadInKey,
        Some(_) if !is_dominant_function => Gap::AppliedNonDominant,
        Some(_) => Gap::AppliedTargetMissing,
    })
}

/// Reports whether reading `chord` as tonicizing `tonicized` gives the label's degree and secondary
/// degree, which is all the benchmark compares. The mode of a tonicized key matters only where it
/// changes a degree, as it does for leading-tone chords but not for dominants.
fn has_same_numeral(label: &Label, tonicized: Option<Key>, chord: Chord) -> bool {
    let reading = Label::new(label.key, tonicized, chord, chord.root);
    reading.degree == label.degree && reading.secondary == label.secondary
}

fn proposal_gap(rules: &RuleSet, label: &Label, chord: Chord, slices: &[Slice]) -> Option<Gap> {
    let is_proposed = |slice: &Slice| {
        rules.propose_keys(slice).contains(&label.key)
            && rules.proposers.iter().any(|propose| propose(slice).contains(&chord))
    };
    (!slices.iter().any(is_proposed)).then_some(Gap::NotProposed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ruleset::Weights;
    use crate::pitch::{Tpc, tpc_from_name};
    use crate::rules::default_rules;
    use crate::score::slicing::SliceNote;

    fn rules() -> RuleSet {
        RuleSet::new(default_rules(), &Weights::new()).unwrap()
    }

    fn slice(names: &[&str]) -> Slice {
        let note = |(i, name): (usize, &&str)| SliceNote {
            tpc: tpc_from_name(name).unwrap(),
            midi: 60 + i as i32,
            attacked: true,
            onset_level: 0,
        };
        let notes = names.iter().enumerate().map(note).collect();
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

    fn label(target: Option<&str>, root: &str, quality: Quality) -> Label {
        label_with_mode(target, "major", root, quality)
    }

    fn label_with_mode(target: Option<&str>, mode: &str, root: &str, quality: Quality) -> Label {
        let key = Key::parse("C", "major").unwrap();
        let tonicized = target.map(|t| Key::parse(t, mode).unwrap());
        let root: Tpc = tpc_from_name(root).unwrap();
        Label::new(key, tonicized, Chord::new(root, quality), root)
    }

    #[test]
    fn dominant_seventh_of_the_dominant_can_be_produced() {
        let gap =
            find_gap(&rules(), &label(Some("G"), "D", Quality::DominantSeventh), &[slice(&["D", "F#", "C"])]);
        assert_eq!(gap, None);
    }

    #[test]
    fn dominant_of_a_minor_target_matches_the_major_target_reading() {
        let label = label_with_mode(Some("F"), "minor", "C", Quality::DominantSeventh);
        assert_eq!(find_gap(&rules(), &label, &[slice(&["C", "E", "Bb"])]), None);
    }

    #[test]
    fn leading_tone_chord_of_a_minor_target_needs_that_target() {
        let label = label_with_mode(Some("F"), "minor", "E", Quality::DiminishedSeventh);
        let gap = find_gap(&rules(), &label, &[slice(&["E", "G", "Bb", "Db"])]);
        assert_eq!(gap, Some(Gap::AppliedTargetMissing));
    }

    #[test]
    fn unknown_quality_cannot_be_produced() {
        assert_eq!(find_gap(&rules(), &label(None, "C", Quality::Other), &[]), Some(Gap::UnknownQuality));
    }

    #[test]
    fn major_chord_on_raised_four_is_not_in_the_key() {
        let gap = find_gap(&rules(), &label(None, "F#", Quality::Major), &[slice(&["F#", "A#"])]);
        assert_eq!(gap, Some(Gap::ChordNotInKey));
    }

    #[test]
    fn mediant_of_the_dominant_is_a_non_dominant_applied_function() {
        let gap = find_gap(&rules(), &label(Some("G"), "B", Quality::Minor), &[slice(&["B", "D", "F#"])]);
        assert_eq!(gap, Some(Gap::AppliedNonDominant));
    }

    #[test]
    fn chord_missing_from_the_notes_is_not_proposed() {
        let gap = find_gap(&rules(), &label(None, "G", Quality::Major), &[slice(&["C", "E"])]);
        assert_eq!(gap, Some(Gap::NotProposed));
    }
}
