//! Rules about key areas whose tonic chord never sounds.

use super::key_confirmation::is_tonic;
use super::sources::{BURSTEIN_STRAUS_2016, KOSTKA_PAYNE_ALMEN_2023};
use crate::engine::rule::{Citation, DepartureContext, Hook, Layer, Rule};

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
    hook: Hook::Departure(absent_tonic),
}];

fn absent_tonic(context: &DepartureContext, _: &[f64]) -> f32 {
    let key = context.previous.reading.key;
    let mut area = context.history.segments().take_while(|s| s.reading.key == key);
    if area.any(|s| is_tonic(&s)) { 0.0 } else { -1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::bass::Bass;
    use crate::engine::model::{Function, NONE, Reading, Segment};
    use crate::engine::rule::History;
    use crate::engine::rule::arenas_of;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::Slice;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

    /// Builds a path of one-slice segments, each given as (key, chord root), read in major.
    fn path(chords: &[(&str, &str)]) -> Vec<Segment> {
        let node = |(i, &(key, root)): (usize, &(&str, &str))| {
            let key = Key::new(tpc_from_name(key).unwrap(), Mode::Major);
            let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
            let reading = Reading { key, chord, function: Function::DIATONIC };
            let previous = if i == 0 { NONE } else { i as u32 - 1 };

            Segment {
                reading,
                start: i as u32,
                previous,
                duration: 1.0,
                members_heard: 0b111,
                bass: Bass::default(),
            }
        };
        chords.iter().enumerate().map(node).collect()
    }

    /// Scores leaving the key of the path's last segment.
    fn depart(chords: &[(&str, &str)]) -> f32 {
        let segments = path(chords);
        let layers = arenas_of(&segments);
        let last = layers.hypotheses.len() as u32 - 1;
        let (start, duration, notes, measure, metric_level, phrase) = (0.0, 1.0, vec![], 0, 0, 0);
        let slice = Slice { start, duration, notes, measure, metric_level, phrase, key_signature: None };
        let history = History { layers: &layers, slices: &[], last };
        absent_tonic(&DepartureContext { previous: &segments[last as usize], slice: &slice, history }, &[])
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
