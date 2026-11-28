//! Scores how well the music heard up to a slice fits the key of a reading.

use super::key_modal_minor::modal_minor_tonics;
use super::key_profile::{natural_minor_logs, profile_logs};
use super::sources::{
    BURSTEIN_STRAUS_2016, KOSTKA_PAYNE_ALMEN_2023, SCHULTZ_ET_AL_2018, TEMPERLEY_2001, TEMPERLEY_2007,
};
use crate::engine::key_contexts::KeySliceContext;
use crate::engine::key_hooks::KeyHook;
use crate::engine::rule::{Citation, Hook, Param, Rule, Topic};
use crate::pitch::pitch_class;
use crate::score::slicing::Slice;
use crate::theory::key::Mode;
use std::cell::Cell;

pub const RULES: &[Rule] = &[Rule {
    id: "key.context_profile",
    topic: Topic::Key,
    summary: "Prefer keys that fit the music heard so far, not only the slice itself. Each slice \
              scores the key profile against the pitch classes sounding in the slice, in the music \
              before it, and in at most the next two quarter notes, the same lag the benchmark \
              allows offered readings. Earlier music counts for less the further back it is, half \
              as much every sixteen quarter notes, and each note counts by its duration. The fit \
              is multiplied by the slice's duration. A chord that briefly sounds like a tonic is then heard within \
              what led up to it. The window stops at a fermata, which ends a phrase in a chorale, \
              so that each phrase is heard afresh. A minor key is scored with a natural minor \
              profile, in which the subtonic and the raised leading tone exchange values, once the \
              key has been established, by its raised leading tone earlier in the piece or by a \
              piece that opens on its tonic triad over the tonic, and its raised leading tone has \
              not sounded in the last 64 quarter notes or the next two. A stretch in natural minor \
              is then not heard as the relative major.",
    citations: &[
        Citation {
            source: TEMPERLEY_2001,
            locator: "pp. 187–188 (a segment heard in context; key has inertia; KPR 1)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 299 (time spent in the new key and consistent accidentals as evidence of modulation)",
        },
        Citation {
            source: SCHULTZ_ET_AL_2018,
            locator: "p. 69 (a phrase of a Bach chorale ends where the voices reach a fermata)",
        },
        Citation {
            source: BURSTEIN_STRAUS_2016,
            locator: "p. 300 (the raised seventh degree establishes a minor key; the subtonic can undermine it)",
        },
        Citation {
            source: KOSTKA_PAYNE_ALMEN_2023,
            locator: "p. 65 (in minor, scale degrees 6 and 7 are variable)",
        },
        Citation {
            source: TEMPERLEY_2007,
            locator: "p. 91 (the folk-song key profiles reflect the natural minor scale)",
        },
        Citation {
            source: TEMPERLEY_2007,
            locator: "p. 121 (the lowered seventh degree is routinely used in minor, quasi-scalar)",
        },
        Citation {
            source: BURSTEIN_STRAUS_2016,
            locator: "p. 368 (pieces usually begin and end in the same key, the main key)",
        },
    ],
    default_weight: 0.5,
    params: &[LOOK_AHEAD, HALF_LIFE, REACH, MODAL_LOOK_BACK],
    hook: Hook::Key(KeyHook::Slice(context_fit)),
}];

/// How far the window reaches past the end of the slice. It is at most the lag the benchmark allows
/// offered readings, so the window holds only music a listener has heard, or will hear within a
/// beat or two.
const LOOK_AHEAD: Param = Param {
    index: 0,
    name: "look_ahead",
    default: 2.0,
    unit: "quarter notes",
    provenance: "limit: the listener may hear at most two quarter notes ahead (README, How it works); 14ea08d, round 4, also tried no look-ahead; measured: sweeps/round7-constants.json, round 7 (0 / 1: tune RN -0.04 / -0.07; kept)",
};
/// The distance back at which a slice's weight halves.
const HALF_LIFE: Param = Param {
    index: 1,
    name: "half_life",
    default: 16.0,
    unit: "quarter notes",
    provenance: "measured: 14ea08d, round 4 (6 / 10 / 16 at weight 1: tune RN -0.59 / -0.32 / -0.26; 12 / 16 / 20 at weight 0.5: -0.14 / -0.03 / -0.04); measured: sweeps/round7-constants.json, round 7 (8 / 12 / 24 / 32: tune RN -0.33 / -0.08 / -0.07 / -0.16; kept)",
};
/// How far back the window reaches.
const REACH: Param = Param {
    index: 2,
    name: "reach",
    default: 64.0,
    unit: "quarter notes",
    provenance: "chosen: 14ea08d, round 4 (four half-lives, where weights have fallen to a sixteenth); measured: sweeps/round7-constants.json, round 7 (32 / 48 / 96 / 128: tune RN -0.20 / -0.06 / -0.02 / -0.02; kept)",
};
/// How far back a minor key's raised leading tone must be absent for the natural minor profile to apply.
const MODAL_LOOK_BACK: Param = Param {
    index: 3,
    name: "modal_look_back",
    default: 64.0,
    unit: "quarter notes",
    provenance: "measured: 6083e2a, round 5 (32 / 48 / 64 / 96: tune RN +0.03 / +0.04 / +0.03 / +0.03, both offered 21.02 / 21.07 / 21.10 / 21.09); measured: sweeps/round7-constants.json and round7-constants-refine.json, round 7 (8 / 16 / 24 / 32 / 48 / 96 / 128: tune RN +0.07 / +0.07 / +0.05 / +0.03 / +0.01 / +0.01 / -0.00; 16 gives +0.07 [-0.03, +0.20] and tune both offered -0.15 [-0.29, -0.05]; kept)",
};

/// The extent of the window around a slice, in quarter notes.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Window {
    look_ahead: f64,
    half_life: f64,
    reach: f64,
}

fn context_fit(context: &KeySliceContext, params: &[f64]) -> f32 {
    let window = Window {
        look_ahead: LOOK_AHEAD.read(params),
        half_life: HALF_LIFE.read(params),
        reach: REACH.read(params),
    };
    let weights = cached_window_weights(context.slices, context.index, window);
    let total: f32 = weights.iter().sum();
    if total == 0.0 {
        return 0.0;
    }
    let key = context.reading.function.key;
    let tonic = pitch_class(key.tonic) as usize;
    let look_back = MODAL_LOOK_BACK.read(params);
    let is_modal =
        key.mode == Mode::Minor && modal_minor_tonics(context.slices, context.index, look_back)[tonic];
    let logs = if is_modal { natural_minor_logs() } else { profile_logs(key.mode) };
    let fit: f32 = (0..12).map(|pc| weights[pc] * logs[(pc + 12 - tonic) % 12]).sum();
    fit / total * context.slice.duration as f32
}

/// Identifies a slice of a particular score: the addresses of the slice list and of the slice's
/// notes, the list's length and the slice's index.
type SliceIdentity = (usize, usize, usize, usize);

thread_local! {
    /// The most recent window. The engine scores many readings of one slice in a row, so one
    /// entry avoids nearly all recomputation without changing any result.
    static LAST_WINDOW: Cell<(SliceIdentity, Window, [f32; 12])> = const {
        let window = Window { look_ahead: 0.0, half_life: 0.0, reach: 0.0 };
        Cell::new(((0, 0, 0, usize::MAX), window, [0.0; 12]))
    };
}

fn cached_window_weights(slices: &[Slice], index: usize, window: Window) -> [f32; 12] {
    let identity = (slices.as_ptr() as usize, slices[index].notes.as_ptr() as usize, slices.len(), index);
    let (cached_identity, cached_window, weights) = LAST_WINDOW.get();
    if cached_identity == identity && cached_window == window {
        return weights;
    }
    let weights = window_weights(slices, index, window);
    LAST_WINDOW.set((identity, window, weights));
    weights
}

/// Sums the sounding duration of each pitch class in the slice, in the music of its phrase heard
/// before it, tapering with distance, and in the music just ahead.
fn window_weights(slices: &[Slice], index: usize, window: Window) -> [f32; 12] {
    let slice = &slices[index];
    let horizon = slice.start + slice.duration + window.look_ahead;
    let in_phrase = |other: &&Slice| other.phrase == slice.phrase;
    let before = slices[..index]
        .iter()
        .rev()
        .take_while(in_phrase)
        .take_while(|s| s.start + s.duration > slice.start - window.reach);
    let after = slices[index + 1..].iter().take_while(in_phrase).take_while(|s| s.start < horizon);
    let mut weights = [0.0; 12];
    let mut add = |other: &Slice, weight: f64| {
        for note in &other.notes {
            weights[pitch_class(note.tpc) as usize] += weight as f32;
        }
    };
    add(slice, slice.duration);
    for other in before {
        let distance = slice.start - (other.start + other.duration / 2.0);
        add(other, 0.5f64.powf(distance / window.half_life) * other.duration);
    }
    for other in after {
        add(other, horizon.min(other.start + other.duration) - other.start);
    }
    weights
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Function, Reading};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn slice(start: f64, names: &[&str]) -> Slice {
        let notes = names
            .iter()
            .enumerate()
            .map(|(i, name)| SliceNote {
                tpc: tpc_from_name(name).unwrap(),
                midi: 60 + i as i32,
                attacked: true,
                onset_level: 0,
            })
            .collect();
        Slice {
            start,
            duration: 4.0,
            notes,
            measure: start as usize / 4,
            metric_level: 0,
            phrase: 0,
            key_signature: None,
        }
    }

    /// A C major passage, I–IV–V–I–IV–V–I, with an F major chord in the middle.
    fn passage() -> Vec<Slice> {
        let chords: [&[&str]; 7] = [
            &["C", "E", "G"],
            &["F", "A", "C"],
            &["G", "B", "D"],
            &["C", "E", "G"],
            &["F", "A", "C"],
            &["G", "B", "D"],
            &["C", "E", "G"],
        ];
        chords.iter().enumerate().map(|(i, names)| slice(4.0 * i as f64, names)).collect()
    }

    fn fit(slices: &[Slice], index: usize, tonic: &str) -> f32 {
        fit_in(slices, index, Key::new(tpc_from_name(tonic).unwrap(), Mode::Major))
    }

    fn fit_in(slices: &[Slice], index: usize, key: Key) -> f32 {
        let chord = Chord::new(tpc_from_name("F").unwrap(), Quality::Major);
        let reading = Reading { chord, function: Function::diatonic(key) };
        let context = KeySliceContext {
            slice: &slices[index],
            next_slice: slices.get(index + 1),
            slices,
            index,
            reading: &reading,
            is_opening: true,
        };
        context_fit(&context, &RULES[0].default_params())
    }

    #[test]
    fn subdominant_chord_in_context_fits_the_home_key_better_than_its_own() {
        let slices = passage();
        assert!(fit(&slices, 4, "C") > fit(&slices, 4, "F"));
    }

    /// A passage in A minor without G sharp, i–v–iv–i, opening on the given voicing of the tonic triad.
    fn natural_minor_passage(opening: &[&str]) -> Vec<Slice> {
        let chords: [&[&str]; 4] = [opening, &["E", "G", "B"], &["D", "F", "A"], &["A", "C", "E"]];
        chords.iter().enumerate().map(|(i, names)| slice(4.0 * i as f64, names)).collect()
    }

    /// The same pitch classes fit A minor better when the piece opened on its tonic triad over A,
    /// since the natural minor profile then applies.
    #[test]
    fn a_natural_minor_passage_fits_better_once_the_minor_key_is_established() {
        let a_minor = Key::new(tpc_from_name("A").unwrap(), Mode::Minor);
        let established = fit_in(&natural_minor_passage(&["A", "C", "E"]), 3, a_minor);
        assert!(established > fit_in(&natural_minor_passage(&["C", "E", "A"]), 3, a_minor));
    }

    fn default_window() -> Window {
        Window { look_ahead: 2.0, half_life: 16.0, reach: 64.0 }
    }

    fn weight(weights: &[f32; 12], name: &str) -> f32 {
        weights[pitch_class(tpc_from_name(name).unwrap()) as usize]
    }

    #[test]
    fn window_weights_stop_at_the_start_of_the_phrase() {
        let mut slices = [slice(0.0, &["C"]), slice(4.0, &["D"])];
        slices[1].phrase = 1;
        assert_eq!(weight(&window_weights(&slices, 1, default_window()), "C"), 0.0);
    }

    #[test]
    fn window_weights_stop_at_the_end_of_the_phrase() {
        let mut slices = [slice(0.0, &["C"]), slice(4.0, &["D"])];
        slices[1].phrase = 1;
        assert_eq!(weight(&window_weights(&slices, 0, default_window()), "D"), 0.0);
    }

    #[test]
    fn window_weights_fade_with_distance() {
        let slices = [slice(0.0, &["C"]), slice(12.0, &["D"]), slice(16.0, &["E"])];
        let weights = window_weights(&slices, 2, default_window());
        assert!(weight(&weights, "D") > weight(&weights, "C"));
    }

    #[test]
    fn window_weights_count_the_next_two_quarter_notes() {
        let slices = [slice(0.0, &["C"]), slice(4.0, &["D"])];
        assert_eq!(weight(&window_weights(&slices, 0, default_window()), "D"), 2.0);
    }

    #[test]
    fn window_weights_ignore_music_more_than_two_quarter_notes_ahead() {
        let slices = [slice(0.0, &["C"]), slice(4.0, &["D"]), slice(8.0, &["E"])];
        assert_eq!(weight(&window_weights(&slices, 0, default_window()), "E"), 0.0);
    }
}
