//! Rules about key areas whose tonic chord never sounds.

use super::key_confirmation::is_tonic;
use super::sources::{BURSTEIN_STRAUS_2016, KOSTKA_PAYNE_ALMEN_2023};
use crate::engine::key_hooks::{KeyDepartureContext, KeyHook};
use crate::engine::rule::{Citation, Hook, Layer, Rule};

pub const RULES: &[Rule] = &[Rule {
    id: "key.absent_tonic",
    layer: Layer::Key,
    summary: "Prefer not to leave a key area in which the key's tonic chord never sounded, in any \
              inversion. A key is established by its tonic; an area that holds only other chords \
              of the key, such as a prolonged dominant or leading-tone chord, is better heard as \
              applied chords within the surrounding key, whose goal chord may never arrive.",
    citations: &[
        Citation {
            source: BURSTEIN_STRAUS_2016,
            locator: "p. 152 (the I chord has a Tonic function and establishes tonal repose)",
        },
        Citation {
            source: BURSTEIN_STRAUS_2016,
            locator: "p. 374 (a tonicization treats a harmony other than I briefly as a tonic; a modulation is long and substantial and confirmed with a cadence)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 286 (secondary dominants may resolve deceptively, so the chord they tonicize need not follow)",
        },
    ],
    default_weight: 1.0,
    params: &[],
    hook: Hook::Key(KeyHook::Departure(absent_tonic)),
}];

fn absent_tonic(context: &KeyDepartureContext, _: &[f64]) -> f32 {
    let key = context.previous.function.key;
    let mut area = context.history.segments().take_while(|s| s.function.key == key);
    if area.any(|s| is_tonic(&s)) { 0.0 } else { -1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Function, KeySegment, Reading};
    use crate::engine::test_layers::{history_after, key_segment, path_of};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    /// Returns a segment of a major triad, read diatonically in a major key.
    fn segment(&(key, root): &(&str, &str)) -> KeySegment {
        let key = Key::new(tpc_from_name(key).unwrap(), Mode::Major);
        let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
        key_segment(Reading { chord, function: Function::diatonic(key) })
    }

    /// Scores leaving the key of the last of a path of segments, each given as (key, chord root).
    fn depart(chords: &[(&str, &str)]) -> f32 {
        let path = path_of(&chords.iter().map(segment).collect::<Vec<_>>());
        let (start, duration, notes, measure, metric_level, phrase) = (0.0, 1.0, vec![], 0, 0, 0);
        let slice = Slice { start, duration, notes, measure, metric_level, phrase, key_signature: None };
        let history = history_after(&path, &[]);
        let previous = path.segment(history.last);
        absent_tonic(&KeyDepartureContext { previous: &previous, slice: &slice, history }, &[])
    }

    #[test]
    fn leaving_an_area_without_its_tonic_costs() {
        assert_eq!(depart(&[("C", "C"), ("G", "D"), ("G", "D")]), -1.0);
    }

    #[test]
    fn leaving_an_area_that_sounded_its_tonic_is_free() {
        assert_eq!(depart(&[("C", "C"), ("G", "D"), ("G", "G"), ("G", "C")]), 0.0);
    }

    #[test]
    fn a_tonic_chord_before_the_area_does_not_count() {
        assert_eq!(depart(&[("G", "G"), ("C", "C"), ("G", "D")]), -1.0);
    }
}
