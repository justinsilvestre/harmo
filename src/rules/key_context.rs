//! Scores how well the music around a slice fits the key of a reading.

use super::key_profile::profile_logs;
use super::sources::{KOSTKA_PAYNE_ALMEN_2023, SCHULTZ_ET_AL_2018, TEMPERLEY_2001};
use crate::engine::rule::{Citation, Hook, Layer, Rule, SliceContext};
use crate::pitch::pitch_class;
use crate::score::slicing::Slice;
use std::cell::Cell;

pub const RULES: &[Rule] = &[Rule {
    id: "key.context_profile",
    layer: Layer::Key,
    summary: "Prefer keys that fit the music around a slice, not only the slice itself. Each slice \
              scores the key profile against the pitch classes sounding within six quarter-note \
              measures to either side, weighted by duration and by nearness, times the slice's \
              duration. A chord that briefly sounds like a tonic is then heard within its \
              surroundings. The window stops at a fermata, which ends a phrase in a chorale, so \
              that each phrase is heard in the key it cadences in.",
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
    default_weight: 1.0,
    hook: Hook::Slice(context_fit),
}];

/// Half the width of the window, in quarter notes. Weights fall linearly to zero at its edges.
const REACH: f64 = 24.0;

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

/// Sums the sounding duration of each pitch class near a slice and in its phrase, tapering with
/// distance.
fn window_weights(slices: &[Slice], index: usize) -> [f32; 12] {
    let slice = &slices[index];
    let (from, to) = (slice.start - REACH, slice.start + slice.duration + REACH);
    let in_phrase = |other: &&Slice| other.phrase == slice.phrase;
    let before =
        slices[..index].iter().rev().take_while(in_phrase).take_while(|s| s.start + s.duration > from);
    let after = slices[index..].iter().take_while(in_phrase).take_while(|s| s.start < to);
    let mut weights = [0.0; 12];
    for other in before.chain(after) {
        let middle = other.start + other.duration / 2.0;
        let distance = (slice.start - middle).max(middle - slice.start - slice.duration).max(0.0);
        let weight = ((1.0 - distance / REACH).max(0.0) * other.duration) as f32;
        for note in &other.notes {
            weights[pitch_class(note.tpc) as usize] += weight;
        }
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
        Slice { start, duration: 4.0, notes, measure: start as usize / 4, metric_level: 0, phrase: 0 }
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

    #[test]
    fn window_weights_stop_at_the_end_of_the_phrase() {
        let mut slices = [slice(0.0, &["C"]), slice(4.0, &["D"])];
        slices[1].phrase = 1;
        let weights = window_weights(&slices, 0);
        assert_eq!(weights[pitch_class(tpc_from_name("D").unwrap()) as usize], 0.0);
    }

    #[test]
    fn window_weights_fade_with_distance() {
        let slices = [slice(0.0, &["C"]), slice(4.0, &["D"]), slice(16.0, &["E"])];
        let weights = window_weights(&slices, 0);
        let pc = |name: &str| pitch_class(tpc_from_name(name).unwrap()) as usize;
        assert!(weights[pc("D")] > weights[pc("E")]);
    }
}
