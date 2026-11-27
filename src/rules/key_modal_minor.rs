//! Decides, for each minor key, whether a slice is heard against the natural minor profile rather
//! than the harmonic minor one.
//!
//! A minor key is established by its raised leading tone, or by a piece that opens on its tonic
//! triad. Once the key is established, a later stretch without the raised leading tone is heard
//! as the natural form of the same minor key, whose seventh degree is the subtonic, rather than as
//! its relative major.

use crate::pitch::pitch_class;
use crate::score::slicing::Slice;
use std::cell::Cell;

/// How far the recent music reaches past the end of the slice, in quarter notes, the lag the
/// benchmark allows offered readings.
const LOOK_AHEAD: f64 = 2.0;

/// The raised leading tone, in semitones above the tonic.
const LEADING_TONE: usize = 11;

/// The minor third, in semitones above the tonic.
const MINOR_THIRD: usize = 3;

/// Identifies a slice of a particular score: the addresses of the slice list and of the slice's
/// notes, the list's length and the slice's index.
type SliceIdentity = (usize, usize, usize, usize);

thread_local! {
    /// The modal tonics of the most recent slice asked about, with the look-back they were found
    /// with. The engine scores many readings of one slice in a row, so one entry avoids nearly all
    /// recomputation without changing any result.
    static LAST_MODAL: Cell<(SliceIdentity, f64, [bool; 12])> =
        const { Cell::new(((0, 0, 0, usize::MAX), 0.0, [false; 12])) };
}

/// Marks, by tonic pitch class, the minor keys to score with the natural minor profile at a
/// slice: those established earlier, by their raised leading tone or by the piece's opening
/// chord, whose raised leading tone has not sounded in the last `look_back` quarter notes, the slice
/// itself or the next two quarter notes.
pub fn modal_minor_tonics(slices: &[Slice], index: usize, look_back: f64) -> [bool; 12] {
    let identity = (slices.as_ptr() as usize, slices[index].notes.as_ptr() as usize, slices.len(), index);
    let (cached_identity, cached_look_back, modal) = LAST_MODAL.get();
    if cached_identity == identity && cached_look_back == look_back {
        return modal;
    }
    let earlier = earlier_pitch_classes(slices, index);
    let recent = recent_pitch_classes(slices, index, look_back);
    let opening = opening_minor_tonic(&slices[0]);
    let modal = std::array::from_fn(|tonic| {
        let leading_tone = (tonic + LEADING_TONE) % 12;
        (earlier[leading_tone] || opening == Some(tonic)) && !recent[leading_tone]
    });
    LAST_MODAL.set((identity, look_back, modal));
    modal
}

/// Returns the pitch class of the bass of an opening slice when its minor third sounds above it.
fn opening_minor_tonic(slice: &Slice) -> Option<usize> {
    let bass = pitch_class(slice.notes.iter().min_by_key(|note| note.midi)?.tpc) as usize;
    let third = (bass + MINOR_THIRD) % 12;
    slice.notes.iter().any(|note| pitch_class(note.tpc) as usize == third).then_some(bass)
}

/// Marks the pitch classes sounding anywhere before the slice.
fn earlier_pitch_classes(slices: &[Slice], index: usize) -> [bool; 12] {
    let mut earlier = [false; 12];
    for other in slices[..index].iter().rev() {
        for note in &other.notes {
            earlier[pitch_class(note.tpc) as usize] = true;
        }
        if earlier.iter().all(|&heard| heard) {
            break;
        }
    }
    earlier
}

/// Marks the pitch classes sounding in the slice, in the `look_back` quarter notes before it and in
/// the next two quarter notes.
fn recent_pitch_classes(slices: &[Slice], index: usize, look_back: f64) -> [bool; 12] {
    let slice = &slices[index];
    let horizon = slice.start + slice.duration + LOOK_AHEAD;
    let before = slices[..index].iter().rev().take_while(|s| s.start + s.duration > slice.start - look_back);
    let after = slices[index + 1..].iter().take_while(|s| s.start < horizon);
    let mut recent = [false; 12];
    for other in before.chain(after).chain(std::iter::once(slice)) {
        for note in &other.notes {
            recent[pitch_class(note.tpc) as usize] = true;
        }
    }
    recent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::score::slicing::SliceNote;

    /// Builds a four-quarter-note slice whose notes rise from the first name.
    fn slice(start: f64, names: &[&str]) -> Slice {
        let notes = names
            .iter()
            .enumerate()
            .map(|(i, name)| SliceNote {
                tpc: tpc_from_name(name).unwrap(),
                midi: 48 + 4 * i as i32,
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

    fn is_modal(slices: &[Slice], index: usize, tonic: &str) -> bool {
        modal_minor_tonics(slices, index, 64.0)[pitch_class(tpc_from_name(tonic).unwrap()) as usize]
    }

    #[test]
    fn a_minor_key_is_modal_once_its_leading_tone_has_faded() {
        let slices = [slice(0.0, &["E", "G#", "B"]), slice(80.0, &["G", "B", "D"])];
        assert!(is_modal(&slices, 1, "A"));
    }

    #[test]
    fn a_minor_key_whose_leading_tone_never_sounded_is_not_modal() {
        let slices = [slice(0.0, &["C", "E", "G"]), slice(80.0, &["G", "B", "D"])];
        assert!(!is_modal(&slices, 1, "A"));
    }

    #[test]
    fn a_minor_key_whose_leading_tone_sounded_recently_is_not_modal() {
        let slices = [slice(0.0, &["E", "G#", "B"]), slice(40.0, &["G", "B", "D"])];
        assert!(!is_modal(&slices, 1, "A"));
    }

    #[test]
    fn a_leading_tone_in_the_next_two_quarter_notes_counts_as_recent() {
        let slices = [slice(0.0, &["E", "G#", "B"]), slice(80.0, &["G", "B", "D"]), slice(84.0, &["G#"])];
        assert!(!is_modal(&slices, 1, "A"));
    }

    #[test]
    fn a_piece_opening_on_a_minor_triad_makes_its_key_modal() {
        let slices = [slice(0.0, &["A", "C", "E"]), slice(4.0, &["G", "B", "D"])];
        assert!(is_modal(&slices, 1, "A"));
    }

    #[test]
    fn a_minor_triad_over_its_third_does_not_establish_its_key() {
        let slices = [slice(0.0, &["C", "E", "A"]), slice(4.0, &["G", "B", "D"])];
        assert!(!is_modal(&slices, 1, "A"));
    }
}
