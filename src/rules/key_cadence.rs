//! Rules about cadences as evidence for a key.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::model::Segment;
use crate::engine::rule::{Citation, Hook, Layer, Param, Rule, TransitionContext};
use crate::theory::chord::Quality;

pub const RULES: &[Rule] = &[Rule {
    id: "key.cadential_confirmation",
    layer: Layer::Key,
    summary: "Prefer keys confirmed by a perfect authentic cadence: a root-position V or V7 followed by \
              a root-position I on a strong beat. A cadence in a new key is what confirms a modulation, \
              while the same chords heard as applied chords of the old key earn nothing here.",
    citations: &[Citation {
        source: KOSTKA_PAYNE_ALMEN_2023,
        locator: "pp. 298–299 (evidence that convinces the listener of a modulation)",
    }],
    default_weight: 0.375,
    params: &[STRONG_BEAT],
    hook: Hook::Transition(cadential_confirmation),
}];

/// The weakest metric level on which a cadential tonic may arrive (3 = mid-measure in duple meters).
const STRONG_BEAT: Param = Param {
    index: 0,
    name: "strong_beat",
    default: 3.0,
    unit: "metric level",
    provenance: "chosen: 15273b4, round 2 (Kostka, Payne and Almen 2023, p. 149: the last chord of a cadence usually falls on a stronger beat than the one before it, but no level is given); measured: sweeps/round7-constants.json, round 7 (2 / 4: tune RN -0.05 / -0.08; kept)",
};

fn cadential_confirmation(context: &TransitionContext, params: &[f64]) -> f32 {
    let Some(previous) = context.previous else { return 0.0 };
    let tonic = context.next;
    let key = tonic.key;
    let is_tonic = tonic.tonicized.is_none() && tonic.chord.root == key.tonic && previous.reading.key == key;
    let bass_is_tonic = context.slice.bass().is_some_and(|note| note.tpc == key.tonic);
    let is_strong = f64::from(context.slice.metric_level) >= STRONG_BEAT.read(params);
    if is_tonic && bass_is_tonic && is_strong && is_root_position_dominant(previous) { 1.0 } else { 0.0 }
}

fn is_root_position_dominant(segment: &Segment) -> bool {
    let reading = segment.reading;
    let is_dominant = reading.tonicized.is_none()
        && reading.chord.root == reading.key.tonic + 1
        && matches!(reading.chord.quality, Quality::Major | Quality::DominantSeventh);
    is_dominant && segment.bass_tpc() == reading.chord.root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Category, NONE, Reading};
    use crate::engine::rule::History;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::{Slice, SliceNote};
    use crate::theory::chord::Chord;
    use crate::theory::key::{Key, Mode};

    fn tpc(name: &str) -> i32 {
        tpc_from_name(name).unwrap()
    }

    fn reading(root: &str, quality: Quality) -> Reading {
        let key = Key::new(tpc("C"), Mode::Major);
        Reading { key, chord: Chord::new(tpc(root), quality), tonicized: None, category: Category::Diatonic }
    }

    /// Scores G7, with the given bass, moving to C major on a slice with the given bass and metric level.
    fn cadence(dominant_bass: &str, tonic_bass: &str, metric_level: u8) -> f32 {
        let bass = Bass { lowest: Some((43, tpc(dominant_bass))), ..Bass::default() };
        let dominant = reading("G", Quality::DominantSeventh);
        let previous = Segment {
            reading: dominant,
            start: 0,
            previous: NONE,
            duration: 1.0,
            members_heard: 0b1111,
            bass,
        };
        let note = SliceNote { tpc: tpc(tonic_bass), midi: 48, attacked: true, onset_level: metric_level };
        let slice = Slice {
            start: 1.0,
            duration: 1.0,
            notes: vec![note],
            measure: 0,
            metric_level,
            phrase: 0,
            key_signature: None,
        };
        let next = reading("C", Quality::Major);
        let history = History { arena: &[], slices: &[], last: NONE };
        let params = RULES[0].default_params();
        cadential_confirmation(
            &TransitionContext { previous: Some(&previous), next: &next, slice: &slice, history },
            &params,
        )
    }

    #[test]
    fn root_position_cadence_on_a_downbeat_confirms_the_key() {
        assert_eq!(cadence("G", "C", 4), 1.0);
    }

    #[test]
    fn inverted_dominant_does_not_confirm_the_key() {
        assert_eq!(cadence("B", "C", 4), 0.0);
    }

    #[test]
    fn inverted_tonic_does_not_confirm_the_key() {
        assert_eq!(cadence("G", "E", 4), 0.0);
    }

    #[test]
    fn tonic_on_a_weak_beat_does_not_confirm_the_key() {
        assert_eq!(cadence("G", "C", 2), 0.0);
    }
}
