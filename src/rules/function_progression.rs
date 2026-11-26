//! Rules about how chords function in succession.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::model::{Category, Reading};
use crate::engine::rule::{Citation, Hook, Layer, Rule, TransitionContext};
use crate::pitch::{Tpc, pitch_class};
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[
    Rule {
        id: "function.diatonic_preference",
        layer: Layer::Function,
        summary: "Prefer diatonic readings. Altered chords (applied chords, mixture, the Neapolitan and \
                  augmented sixths) are read only when the notes call for them. Mixture chords whose only \
                  borrowed tone is the lowered sixth degree, the most common kind, cost half as much.",
        citations: &[
            Citation { source: KOSTKA_PAYNE_ALMEN_2023, locator: "p. 255 (chromaticism and altered chords)" },
            Citation {
                source: KOSTKA_PAYNE_ALMEN_2023,
                locator: "p. 361 (ii°, iiø7, iv and vii°7 are the most frequent borrowed chords in major)",
            },
        ],
        default_weight: 1.0,
        hook: Hook::Transition(diatonic_preference),
    },
    Rule {
        id: "function.applied_resolution",
        layer: Layer::Function,
        summary: "An applied chord should be followed by the chord it tonicizes.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 256–260, 273–276 (secondary functions in context)",
        }],
        default_weight: 1.0,
        hook: Hook::Transition(applied_resolution),
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
        hook: Hook::Transition(dominant_resolution),
    },
    Rule {
        id: "function.augmented_sixth_bass",
        layer: Layer::Function,
        summary: "Prefer augmented sixth chords with the lowered sixth degree in the bass.",
        citations: &[Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 383–386, 391 (augmented sixths and other bass positions)",
        }],
        default_weight: 1.0,
        hook: Hook::Transition(augmented_sixth_bass),
    },
];

fn diatonic_preference(context: &TransitionContext) -> f32 {
    let next = context.next;
    match next.category {
        Category::Diatonic => 0.0,
        Category::Mixture if borrows_only_lowered_sixth(next) => -0.5,
        _ => -1.0,
    }
}

/// Reports whether the only chord tone foreign to the key is the lowered sixth degree,
/// as in ii°, iiø7, iv and vii°7 borrowed into major.
fn borrows_only_lowered_sixth(reading: &Reading) -> bool {
    let is_altered = |&tpc: &Tpc| reading.key.degree_of(tpc).alteration != 0;
    reading.chord.members().filter(is_altered).all(|tpc| reading.key.degree_of(tpc).number == 6)
}

fn applied_resolution(context: &TransitionContext) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    match previous.reading.tonicized {
        Some(target) if target.tonic == context.next.chord.root => 1.0,
        Some(_) => -1.0,
        None => 0.0,
    }
}

fn dominant_resolution(context: &TransitionContext) -> f32 {
    let Some(previous) = context.previous else {
        return 0.0;
    };
    let reading = previous.reading;
    let next = context.next;
    let resolves = next.key == reading.key && next.chord.root == reading.key.tonic;
    let is_tonic_six_four = context.slice.bass().is_some_and(|n| next.chord.member_index(n.tpc) == Some(2));
    let rewarded = reading.tonicized.is_none() && has_dominant_function(reading) && resolves;
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

fn augmented_sixth_bass(context: &TransitionContext) -> f32 {
    let chord = context.next.chord;
    let lowered_sixth = match chord.quality {
        Quality::ItalianSixth | Quality::GermanSixth => chord.root - 10,
        Quality::FrenchSixth => chord.root - 6,
        _ => return 0.0,
    };
    let bass = context.slice.bass().map(|n| pitch_class(n.tpc));
    if bass == Some(pitch_class(lowered_sixth)) { 0.0 } else { -1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    fn reading(key: &str, root: &str, quality: Quality, category: Category) -> Reading {
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        Reading { key: Key::parse(key, "major").unwrap(), chord, tonicized: None, category }
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
