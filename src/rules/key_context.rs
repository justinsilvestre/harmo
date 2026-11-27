//! Scores how well the music heard up to a slice fits the key of a reading.

use super::key_profile::profile_logs;
use super::sources::{KOSTKA_PAYNE_ALMEN_2023, SCHULTZ_ET_AL_2018, TEMPERLEY_2001};
use crate::engine::rule::{Citation, Hook, Layer, Rule, SliceContext};
use crate::pitch::pitch_class;
use crate::score::slicing::Slice;
use std::cell::Cell;

pub const RULES: &[Rule] = &[Rule {
    id: "key.context_profile",
    layer: Layer::Key,
    summary: "Prefer keys that fit the music heard so far, not only the slice itself. Each slice \
              scores the key profile against the pitch classes sounding in the slice, in the music \
              before it, and in at most the next two quarter notes, the same lag the benchmark \
              allows offered readings. Earlier music counts for less the further back it is, half \
              as much every sixteen quarter notes, and each note counts by its duration. The fit \
              is multiplied by the slice's duration. A chord that briefly sounds like a tonic is then heard within \
              what led up to it. The window stops at a fermata, which ends a phrase in a chorale, \
              so that each phrase is heard afresh.",
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
    ],
    default_weight: 0.5,
    hook: Hook::Slice(context_fit),
}];

/// How far the window reaches past the end of the slice, in quarter notes. It is the same lag the
/// benchmark allows offered readings, so the window holds only music a listener has heard, or will
/// hear within a beat or two.
const LOOK_AHEAD: f64 = 2.0;

/// The distance back, in quarter notes, at which a slice's weight halves.
const HALF_LIFE: f64 = 16.0;

/// How far back the window reaches, in quarter notes. Weights there have fallen to a sixteenth.
const REACH: f64 = 4.0 * HALF_LIFE;

fn context_fit(context: &SliceContext) -> f32 {
    let weights = cached_window_weights(context.slices, context.index);
    let total: f32 = weights.iter().sum();
    if total == 0.0 {
        return 0.0;
    }
    let key = context.reading.key;
    let logs = profile_logs(key.mode);
    let tonic = pitch_class(key.tonic) as usize;
    let fit: f32 = (0..12).map(|pc| weights[pc] * logs[(pc + 12 - tonic) % 12]).sum();
    fit / total * context.slice.duration as f32
}

/// Identifies a slice of a particular score: the addresses of the slice list and of the slice's
/// notes, the list's length and the slice's index.
type SliceIdentity = (usize, usize, usize, usize);

thread_local! {
    /// The most recent window. The engine scores many readings of one slice in a row, so one
    /// entry avoids nearly all recomputation without changing any result.
    static LAST_WINDOW: Cell<(SliceIdentity, [f32; 12])> = const { Cell::new(((0, 0, 0, usize::MAX), [0.0; 12])) };
}

fn cached_window_weights(slices: &[Slice], index: usize) -> [f32; 12] {
    let identity = (slices.as_ptr() as usize, slices[index].notes.as_ptr() as usize, slices.len(), index);
    let (cached_identity, weights) = LAST_WINDOW.get();
    if cached_identity == identity {
        return weights;
    }
    let weights = window_weights(slices, index);
    LAST_WINDOW.set((identity, weights));
    weights
}

/// Sums the sounding duration of each pitch class in the slice, in the music of its phrase heard
/// before it, tapering with distance, and in the next two quarter notes.
fn window_weights(slices: &[Slice], index: usize) -> [f32; 12] {
    let slice = &slices[index];
    let horizon = slice.start + slice.duration + LOOK_AHEAD;
    let in_phrase = |other: &&Slice| other.phrase == slice.phrase;
    let before = slices[..index]
        .iter()
        .rev()
        .take_while(in_phrase)
        .take_while(|s| s.start + s.duration > slice.start - REACH);
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
        add(other, 0.5f64.powf(distance / HALF_LIFE) * other.duration);
    }
    for other in after {
        add(other, horizon.min(other.start + other.duration) - other.start);
    }
    weights
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Category, Reading};
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::{Key, Mode};

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
        let key = Key::new(tpc_from_name(tonic).unwrap(), Mode::Major);
        let chord = Chord::new(tpc_from_name("F").unwrap(), Quality::Major);
        let reading = Reading { key, chord, tonicized: None, category: Category::Diatonic };
        let context = SliceContext {
            slice: &slices[index],
            next_slice: slices.get(index + 1),
            slices,
            index,
            reading: &reading,
            is_opening: true,
        };
        context_fit(&context)
    }

    #[test]
    fn subdominant_chord_in_context_fits_the_home_key_better_than_its_own() {
        let slices = passage();
        assert!(fit(&slices, 4, "C") > fit(&slices, 4, "F"));
    }

    fn weight(weights: &[f32; 12], name: &str) -> f32 {
        weights[pitch_class(tpc_from_name(name).unwrap()) as usize]
    }

    #[test]
    fn window_weights_stop_at_the_start_of_the_phrase() {
        let mut slices = [slice(0.0, &["C"]), slice(4.0, &["D"])];
        slices[1].phrase = 1;
        assert_eq!(weight(&window_weights(&slices, 1), "C"), 0.0);
    }

    #[test]
    fn window_weights_stop_at_the_end_of_the_phrase() {
        let mut slices = [slice(0.0, &["C"]), slice(4.0, &["D"])];
        slices[1].phrase = 1;
        assert_eq!(weight(&window_weights(&slices, 0), "D"), 0.0);
    }

    #[test]
    fn window_weights_fade_with_distance() {
        let slices = [slice(0.0, &["C"]), slice(12.0, &["D"]), slice(16.0, &["E"])];
        let weights = window_weights(&slices, 2);
        assert!(weight(&weights, "D") > weight(&weights, "C"));
    }

    #[test]
    fn window_weights_count_the_next_two_quarter_notes() {
        let slices = [slice(0.0, &["C"]), slice(4.0, &["D"])];
        assert_eq!(weight(&window_weights(&slices, 0), "D"), 2.0);
    }

    #[test]
    fn window_weights_ignore_music_more_than_two_quarter_notes_ahead() {
        let slices = [slice(0.0, &["C"]), slice(4.0, &["D"]), slice(8.0, &["E"])];
        assert_eq!(weight(&window_weights(&slices, 0), "E"), 0.0);
    }
}
