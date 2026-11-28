//! Rules about a cadential six-four as evidence that the music has moved to a new key.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::key_hooks::{KeyHook, KeyTransitionContext};
use crate::engine::model::KeySegment;
use crate::engine::rule::{Citation, Hook, Layer, Param, Rule};
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "key.six_four_tonicization",
    layer: Layer::Key,
    summary: "Prefer not to hear a cadential six-four only as a tonicization. A major or minor triad \
              in second inversion on a strong beat, followed on a weaker beat by a root-position \
              major triad or dominant seventh over the same bass, is a cadential six-four: the pair \
              stands for the dominant of the key whose tonic is the six-four's root. Reading the \
              second chord as an applied dominant of that root, as in V64–V7/V, costs, since a \
              cadential six-four in a key is evidence that the music has modulated to it.",
    citations: &[
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "pp. 135, 144 (the cadential I64 falls on a stronger beat than the root-position V it delays)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 299 (a cadential I64–V in the new key as evidence of a modulation)",
        },
    ],
    default_weight: 0.5,
    params: &[STRONG_BEAT],
    hook: Hook::Key(KeyHook::Transition(six_four_tonicization)),
}];

/// The weakest metric level on which a cadential six-four may begin (3 = mid-measure in duple meters).
const STRONG_BEAT: Param = Param {
    index: 0,
    name: "strong_beat",
    default: 3.0,
    unit: "metric level",
    provenance: "measured: b52a2e8, round 6 (any stronger beat / level 3 / level 4, each at weight 0.5: tune RN 57.99 / 57.98 / 57.97); measured: sweeps/round7-constants.json, round 7 (2 / 4: tune RN +0.02 / -0.00; kept)",
};

fn six_four_tonicization(context: &KeyTransitionContext, params: &[f64]) -> f32 {
    let Some(previous) = context.previous else { return 0.0 };
    let opening = context.history.slices.get(previous.chord.start as usize).map_or(0, |s| s.metric_level);
    let is_on_stronger_beat =
        f64::from(opening) >= STRONG_BEAT.read(params) && opening > context.slice.metric_level;
    let tonicizes_six_four =
        context.next.function.tonicized.is_some_and(|t| t.tonic == previous.reading().chord.root);
    let is_cadential = is_on_stronger_beat && is_six_four(previous) && is_dominant_on_bass(context, previous);
    if is_cadential && tonicizes_six_four { -1.0 } else { 0.0 }
}

/// Reports whether the segment is a major or minor triad with its fifth in the bass.
fn is_six_four(segment: &KeySegment) -> bool {
    let chord = segment.reading().chord;
    matches!(chord.quality, Quality::Major | Quality::Minor)
        && chord.member_index(segment.chord.bass_tpc()) == Some(2)
}

/// Reports whether the new chord is a major triad or dominant seventh in root position on the bass
/// of the previous segment.
fn is_dominant_on_bass(context: &KeyTransitionContext, previous: &KeySegment) -> bool {
    let bass = previous.chord.bass_tpc();
    let keeps_bass = context.slice.bass().is_some_and(|note| note.tpc == bass);
    let chord = context.next.chord;
    keeps_bass && chord.root == bass && matches!(chord.quality, Quality::Major | Quality::DominantSeventh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::key_hooks::History;
    use crate::engine::model::{Category, ChordSegment, Function, KeySegment, Reading};
    use crate::engine::test_layers::key_segment;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::{Slice, SliceNote};
    use crate::theory::chord::Chord;
    use crate::theory::key::{Key, Mode};

    fn tpc(name: &str) -> i32 {
        tpc_from_name(name).unwrap()
    }

    fn slice(metric_level: u8, bass: &str) -> Slice {
        let note = SliceNote { tpc: tpc(bass), midi: 50, attacked: true, onset_level: metric_level };
        Slice {
            start: 0.0,
            duration: 1.0,
            notes: vec![note],
            measure: 0,
            metric_level,
            phrase: 0,
            key_signature: None,
        }
    }

    /// The beats and basses of a G major triad and the D7 after it.
    struct Setting<'a> {
        six_four_bass: &'a str,
        six_four_level: u8,
        dominant_bass: &'a str,
        dominant_level: u8,
    }

    const CADENTIAL: Setting =
        Setting { six_four_bass: "D", six_four_level: 4, dominant_bass: "D", dominant_level: 2 };

    /// Scores a G major triad read in C moving to D7 read in C, applied to the given degree.
    fn score(setting: &Setting, tonicized: Option<&str>) -> f32 {
        let c_major = Key::new(tpc("C"), Mode::Major);
        let chord = Chord::new(tpc("G"), Quality::Major);
        let six_four = Reading { chord, function: Function::diatonic(c_major) };
        let bass = Bass { note: Some((50, tpc(setting.six_four_bass))), ..Bass::default() };
        let segment = key_segment(six_four);
        let previous = KeySegment { chord: ChordSegment { bass, ..segment.chord }, ..segment };
        let tonicized = tonicized.map(|t| Key::new(tpc(t), Mode::Major));
        let category = if tonicized.is_some() { Category::Applied } else { Category::Diatonic };
        let chord = Chord::new(tpc("D"), Quality::DominantSeventh);
        let next = Reading { chord, function: Function { key: c_major, tonicized, category } };
        let slices = [
            slice(setting.six_four_level, setting.six_four_bass),
            slice(setting.dominant_level, setting.dominant_bass),
        ];
        let history = History::empty(&slices);
        let context =
            KeyTransitionContext { previous: Some(&previous), next: &next, slice: &slices[1], history };
        six_four_tonicization(&context, &RULES[0].default_params())
    }

    #[test]
    fn a_cadential_six_four_read_as_v64_before_v7_of_v_costs() {
        assert_eq!(score(&CADENTIAL, Some("G")), -1.0);
    }

    #[test]
    fn a_six_four_on_a_weaker_beat_than_the_dominant_is_free() {
        assert_eq!(score(&Setting { six_four_level: 1, ..CADENTIAL }, Some("G")), 0.0);
    }

    #[test]
    fn a_six_four_off_the_strong_beats_is_free() {
        assert_eq!(score(&Setting { six_four_level: 2, dominant_level: 1, ..CADENTIAL }, Some("G")), 0.0);
    }

    #[test]
    fn a_root_position_triad_before_v7_of_v_is_free() {
        assert_eq!(score(&Setting { six_four_bass: "G", ..CADENTIAL }, Some("G")), 0.0);
    }

    #[test]
    fn an_inverted_dominant_after_the_six_four_is_free() {
        assert_eq!(score(&Setting { dominant_bass: "F#", ..CADENTIAL }, Some("G")), 0.0);
    }

    #[test]
    fn the_dominant_read_without_tonicization_is_free() {
        assert_eq!(score(&CADENTIAL, None), 0.0);
    }
}
