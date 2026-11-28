//! Rules about which diatonic seventh chords the style favors.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::rule::{Citation, Hook, Layer, Rule, TransitionContext};
use crate::theory::chord::Quality;
use crate::theory::key::Mode;

pub const RULES: &[Rule] = &[Rule {
    id: "function.rare_seventh",
    layer: Layer::Function,
    summary: "Avoid reading the rarest diatonic seventh chords: those on the tonic, mediant and \
              submediant (IM7, iii7 and vi7 in major; i7, III7 and VI7 in minor).",
    citations: &[Citation {
        source: KOSTKA_PAYNE_ALMEN_2023,
        locator: "p. 237 (seventh chords ranked by frequency: V7, ii7, viiø7, IVM7, vi7, IM7, iii7)",
    }],
    default_weight: 1.5,
    params: &[],
    hook: Hook::Transition(rare_seventh),
}];

/// Roots of the rare seventh chords, in fifths above the tonic: tonic, mediant and submediant.
const MAJOR_RARE_ROOTS: [i32; 3] = [0, 4, 3];
const MINOR_RARE_ROOTS: [i32; 3] = [0, -3, -4];

fn rare_seventh(context: &TransitionContext, _: &[f64]) -> f32 {
    let reading = context.next;
    let is_nondominant_seventh = matches!(
        reading.chord.quality,
        Quality::MajorSeventh | Quality::MinorSeventh | Quality::MinorMajorSeventh
    );
    let rare_roots = match reading.key.mode {
        Mode::Major => MAJOR_RARE_ROOTS,
        Mode::Minor => MINOR_RARE_ROOTS,
    };
    let is_rare_root = rare_roots.contains(&(reading.chord.root - reading.key.tonic));
    let is_rare = is_nondominant_seventh && is_rare_root && reading.function.tonicized.is_none();
    if is_rare { -1.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Function, Reading};
    use crate::engine::rule::History;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::Chord;
    use crate::theory::key::Key;

    fn score(root: &str, quality: Quality) -> f32 {
        let key = Key::parse("C", "major").unwrap();
        let chord = Chord::new(tpc_from_name(root).unwrap(), quality);
        let next = Reading { key, chord, function: Function::DIATONIC };
        let slice = Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![],
            measure: 0,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        };
        let history = History::empty(&[]);
        rare_seventh(&TransitionContext { previous: None, next: &next, slice: &slice, history }, &[])
    }

    #[test]
    fn tonic_major_seventh_costs() {
        assert_eq!(score("C", Quality::MajorSeventh), -1.0);
    }

    #[test]
    fn supertonic_seventh_is_free() {
        assert_eq!(score("D", Quality::MinorSeventh), 0.0);
    }

    #[test]
    fn tonic_triad_is_free() {
        assert_eq!(score("C", Quality::Major), 0.0);
    }
}
