//! Rules about how chords function in succession.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::key_hooks::{KeyHook, KeyTransitionContext};
use crate::engine::model::{Category, KeySegment, Reading};
use crate::engine::rule::{Citation, Hook, Layer, Param, Rule};
use crate::pitch::Tpc;
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[
    Rule {
        id: "function.diatonic_preference",
        layer: Layer::Function,
        summary: "Prefer diatonic readings. Altered chords (applied chords, mixture, the Neapolitan and \
                  augmented sixths) are read only when the notes call for them. Mixture chords whose only \
                  borrowed tone is the lowered sixth degree, the most common kind, cost half as much. \
                  An applied chord that continues a tonicization costs three quarters as much: one \
                  that follows a chord applied to the same degree, or follows that degree's own chord \
                  right after a chord applied to it, as in V/V–V–V/V. A longer tonicization is heard \
                  as one excursion to the tonicized degree, not as a new choice at every chord.",
        citations: &[
            Citation { source: KOSTKA_PAYNE_ALMEN_2023, locator: "p. 255 (chromaticism and altered chords)" },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 278 (a bracket under a passage labels a longer tonicization)",
            },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "pp. 298–299 (the line between modulation and tonicization is not clearly defined)",
            },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 361 (ii°, iiø7, iv and vii°7 are the most frequent borrowed chords in major)",
            },
        ],
        default_weight: 1.0,
        params: &[LOWERED_SIXTH_MIXTURE, CONTINUED_TONICIZATION],
        hook: Hook::Key(KeyHook::Transition(diatonic_preference)),
    },
    Rule {
        id: "function.applied_resolution",
        layer: Layer::Function,
        summary: "An applied chord should be followed by the chord it tonicizes.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 256–260, 273–276 (secondary functions in context)",
        }],
        default_weight: 0.75,
        params: &[UNRESOLVED_COST],
        hook: Hook::Key(KeyHook::Transition(applied_resolution)),
    },
    Rule {
        id: "function.dominant_resolution",
        layer: Layer::Function,
        summary: "Prefer readings in which a dominant-function chord (V, V7, or the leading-tone triad \
                  or seventh chord) is followed by I (or i) of the same key. A tonic six-four does not \
                  count, since the cadential I64 delays V rather than resolving it.",
        citations: &[
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "pp. 95–98 (harmonic progression; the I and V chords)",
            },
            Citation { source: KOSTKA_PAYNE_ALMEN_2023, locator: "p. 102 (vii° substitutes for V)" },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "pp. 240, 242 (viiø7 and vii°7 have a dominant function and resolve to tonic)",
            },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 135 (the cadential I64 does not represent a tonic function)",
            },
        ],
        default_weight: 0.75,
        params: &[],
        hook: Hook::Key(KeyHook::Transition(dominant_resolution)),
    },
];

/// The cost of a mixture chord whose only borrowed tone is the lowered sixth degree, against 1 for
/// other altered chords.
const LOWERED_SIXTH_MIXTURE: Param = Param {
    index: 0,
    name: "lowered_sixth_mixture",
    default: 0.5,
    unit: "ratio",
    provenance: "chosen: b557b0a, round 1 (Kostka, Payne and Almen 2023, p. 361, name these the most frequent borrowed chords but give no rate; only 0.5 tried: RN +0.1); measured: sweeps/round7-constants.json, round 7 (0.25 / 0.375 / 0.75 / 1: tune RN -0.02 / -0.03 / -0.07 / -0.10; kept)",
};
/// The cost of an applied chord that continues a tonicization, against 1 for one that begins it.
const CONTINUED_TONICIZATION: Param = Param {
    index: 1,
    name: "continued_tonicization",
    default: 0.75,
    unit: "ratio",
    provenance: "measured: de9d21b, round 5 (0 / 0.25 / 0.5 / 0.625 / 0.75 / 0.875: tune RN -0.26 / -0.09 / +0.03 / +0.04 / +0.06 / +0.04); measured: sweeps/round7-constants.json, round 7 (0.5 / 0.625 / 0.875 / 1: tune RN -0.02 / -0.00 / +0.01 / -0.04; kept)",
};
/// The cost of an applied chord not followed by the chord it tonicizes, against a credit of 1 for
/// one that is.
const UNRESOLVED_COST: Param = Param {
    index: 0,
    name: "unresolved_cost",
    default: 1.0,
    unit: "ratio to the credit",
    provenance: "chosen: 4e0fbe2, round 1 (not swept); measured: sweeps/round7-constants.json, round 7 (0.5 / 0.75 / 1.5 / 2: tune RN -0.00 / +0.02 / -0.08 / -0.21; kept)",
};

fn diatonic_preference(context: &KeyTransitionContext, params: &[f64]) -> f32 {
    let next = context.next;
    match next.function.category {
        Category::Diatonic => 0.0,
        Category::Mixture if borrows_only_lowered_sixth(next) => -LOWERED_SIXTH_MIXTURE.read(params) as f32,
        Category::Applied if continues_tonicization(context) => -CONTINUED_TONICIZATION.read(params) as f32,
        _ => -1.0,
    }
}

/// Reports whether an applied chord continues a tonicization already under way: the chord just
/// before it, in the same key, is applied to the same degree, or is that degree's own chord
/// following a chord applied to it.
fn continues_tonicization(context: &KeyTransitionContext) -> bool {
    let next = context.next;
    let Some(target) = next.function.tonicized else { return false };
    let mut recent = context.history.segments().take_while(|s| s.key == next.key);
    let tonicizes = |segment: &KeySegment| segment.function.tonicized == Some(target);
    match recent.next() {
        Some(previous) if tonicizes(&previous) => true,
        Some(previous)
            if previous.reading().chord.root == target.tonic && previous.function.tonicized.is_none() =>
        {
            recent.next().is_some_and(|s| tonicizes(&s))
        }
        _ => false,
    }
}

/// Reports whether the only chord tone foreign to the key is the lowered sixth degree,
/// as in ii°, iiø7, iv and vii°7 borrowed into major.
fn borrows_only_lowered_sixth(reading: &Reading) -> bool {
    let is_altered = |&tpc: &Tpc| reading.key.degree_of(tpc).alteration != 0;
    reading.chord.members().filter(is_altered).all(|tpc| reading.key.degree_of(tpc).number == 6)
}

fn applied_resolution(context: &KeyTransitionContext, params: &[f64]) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    match previous.function.tonicized {
        Some(target) if target.tonic == context.next.chord.root => 1.0,
        Some(_) => -UNRESOLVED_COST.read(params) as f32,
        None => 0.0,
    }
}

fn dominant_resolution(context: &KeyTransitionContext, _: &[f64]) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    let reading = previous.reading();
    let next = context.next;
    let resolves = next.key == reading.key && next.chord.root == reading.key.tonic;
    let is_tonic_six_four = context.slice.bass().is_some_and(|n| next.chord.member_index(n.tpc) == Some(2));
    let rewarded = reading.function.tonicized.is_none() && has_dominant_function(reading) && resolves;
    if rewarded && !is_tonic_six_four { 1.0 } else { 0.0 }
}

/// Reports whether a chord is V, V7, or a leading-tone triad or seventh chord of its key.
fn has_dominant_function(reading: Reading) -> bool {
    let root = reading.chord.root - reading.key.tonic;
    match reading.chord.quality {
        Quality::Major | Quality::DominantSeventh => root == 1,
        Quality::Diminished | Quality::DiminishedSeventh | Quality::HalfDiminishedSeventh => root == 5,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::Function;
    use crate::engine::test_layers::{history_after, key_segment, path_of};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    fn reading(key: &str, root: &str, quality: Quality, category: Category) -> Reading {
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        Reading {
            key: Key::parse(key, "major").unwrap(),
            chord,
            function: Function { tonicized: None, category },
        }
    }

    /// Returns a segment in C major given as (root, quality, tonicized key).
    fn segment(&(root, quality, target): &(&str, Quality, Option<&str>)) -> KeySegment {
        let mut reading = reading("C", root, quality, Category::Diatonic);
        reading.function.tonicized = target.map(|t| Key::parse(t, "major").unwrap());
        key_segment(reading)
    }

    /// Scores reading D major as V/V in C major after a path of the given chords.
    fn secondary_dominant_after(chords: &[(&str, Quality, Option<&str>)]) -> f32 {
        let path = path_of(&chords.iter().map(segment).collect::<Vec<_>>());
        let mut next = reading("C", "D", Quality::Major, Category::Applied);
        next.function.tonicized = Key::parse("G", "major");
        let slice = Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![],
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        };
        let history = history_after(&path, &[]);
        let previous = path.segment(history.last);
        diatonic_preference(
            &KeyTransitionContext { previous: Some(&previous), next: &next, slice: &slice, history },
            &RULES[0].default_params(),
        )
    }

    #[test]
    fn an_applied_chord_beginning_a_tonicization_costs_in_full() {
        assert_eq!(secondary_dominant_after(&[("C", Quality::Major, None)]), -1.0);
    }

    #[test]
    fn an_applied_chord_after_its_target_and_a_chord_applied_to_it_costs_less() {
        let chords = [("D", Quality::DominantSeventh, Some("G")), ("G", Quality::Major, None)];
        assert_eq!(secondary_dominant_after(&chords), -0.75);
    }

    #[test]
    fn an_applied_chord_after_another_applied_to_the_same_degree_costs_less() {
        assert_eq!(secondary_dominant_after(&[("D", Quality::DominantSeventh, Some("G"))]), -0.75);
    }

    #[test]
    fn an_applied_chord_after_its_target_alone_costs_in_full() {
        let chords = [("C", Quality::Major, None), ("G", Quality::Major, None)];
        assert_eq!(secondary_dominant_after(&chords), -1.0);
    }

    #[test]
    fn leading_tone_seventh_has_dominant_function() {
        let vii7 = reading("C", "B", Quality::HalfDiminishedSeventh, Category::Diatonic);
        assert!(has_dominant_function(vii7));
    }

    #[test]
    fn subdominant_has_no_dominant_function() {
        let iv = reading("C", "F", Quality::Major, Category::Diatonic);
        assert!(!has_dominant_function(iv));
    }

    #[test]
    fn borrowed_diminished_seventh_borrows_only_the_lowered_sixth() {
        let vii7 = reading("C", "B", Quality::DiminishedSeventh, Category::Mixture);
        assert!(borrows_only_lowered_sixth(&vii7));
    }

    #[test]
    fn borrowed_submediant_also_borrows_the_lowered_third() {
        let flat_vi = reading("C", "Ab", Quality::Major, Category::Mixture);
        assert!(!borrows_only_lowered_sixth(&flat_vi));
    }
}
