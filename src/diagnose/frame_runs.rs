//! Frame-sequence tests: shifted chord changes, runs of identical labels and runs of wrong keys.

use crate::bench::metrics::is_rn_match;
use crate::theory::label::Label;

/// Reports whether the engine's label at `frame` is wrong only because a chord change is shifted
/// by at most `reach` frames: the expert's label changes within `reach` frames on one side,
/// and the engine shows the expert's label within `reach` frames on the other.
pub fn is_shifted(expert: &[Option<Label>], engine: &[Option<Label>], frame: usize, reach: usize) -> bool {
    let Some(label) = &expert[frame] else {
        return false;
    };
    let matches_at = |f: usize| engine.get(f).copied().flatten().is_some_and(|e| is_rn_match(&e, label));
    let (start, end) = run_bounds(expert, frame);
    let is_late = frame - start < reach && (1..=reach).any(|d| matches_at(frame + d));
    let is_early = end - frame <= reach && (1..=reach.min(frame)).any(|d| matches_at(frame - d));
    is_late || is_early
}

/// Returns the first frame and one past the last frame of the run of equal labels around `frame`.
pub fn run_bounds(frames: &[Option<Label>], frame: usize) -> (usize, usize) {
    let same = |f: usize| is_same(&frames[f], &frames[frame]);
    let start = (0..frame).rev().find(|&f| !same(f)).map_or(0, |f| f + 1);
    let end = (frame + 1..frames.len()).find(|&f| !same(f)).unwrap_or(frames.len());
    (start, end)
}

/// Returns, for each frame, the length in frames of the run of consecutive frames over
/// which the expert's and the engine's keys stay the same pair.
/// Frames with a missing label count as their own pair.
pub fn key_run_lengths(expert: &[Option<Label>], engine: &[Option<Label>]) -> Vec<usize> {
    let pair = |f: usize| (expert[f].map(|l| l.key), engine.get(f).copied().flatten().map(|l| l.key));
    let mut lengths = vec![0; expert.len()];
    let mut start = 0;
    for frame in 1..=expert.len() {
        if frame == expert.len() || pair(frame) != pair(start) {
            lengths[start..frame].fill(frame - start);
            start = frame;
        }
    }
    lengths
}

/// Counts the chord changes in a frame sequence, skipping frames without a label.
pub fn count_changes(frames: impl Iterator<Item = Option<Label>>) -> u64 {
    let labels: Vec<Label> = frames.flatten().collect();
    labels.windows(2).filter(|pair| !is_rn_match(&pair[0], &pair[1])).count() as u64
}

fn is_same(a: &Option<Label>, b: &Option<Label>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => is_rn_match(a, b),
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::frames::label_runs;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn chord(root: &str) -> Option<Label> {
        let root = tpc_from_name(root).unwrap();
        Some(Label::new(Key::parse("C", "major").unwrap(), None, Chord::new(root, Quality::Major), root))
    }

    fn frames(roots: &str) -> Vec<Option<Label>> {
        roots.chars().map(|c| chord(&c.to_string())).collect()
    }

    #[test]
    fn late_chord_change_is_a_shift() {
        assert!(is_shifted(&frames("CCGGGG"), &frames("CCCGGG"), 2, 2));
    }

    #[test]
    fn early_chord_change_is_a_shift() {
        assert!(is_shifted(&frames("CCCCGG"), &frames("CCCGGG"), 3, 2));
    }

    #[test]
    fn change_beyond_the_reach_is_not_a_shift() {
        assert!(!is_shifted(&frames("CGGGGG"), &frames("CCCCGG"), 1, 2));
    }

    #[test]
    fn wrong_chord_far_from_any_change_is_not_a_shift() {
        assert!(!is_shifted(&frames("CCCCCC"), &frames("CCCFCC"), 3, 2));
    }

    #[test]
    fn runs_split_at_each_change() {
        assert_eq!(label_runs(&frames("CCGF")), vec![(0, 2), (2, 3), (3, 4)]);
    }

    #[test]
    fn run_bounds_cover_the_equal_neighbors() {
        assert_eq!(run_bounds(&frames("CGGGF"), 2), (1, 4));
    }

    #[test]
    fn key_runs_measure_each_pair_of_keys() {
        let g = Key::parse("G", "major").unwrap();
        let engine: Vec<_> = frames("CCCC")
            .into_iter()
            .enumerate()
            .map(|(i, l)| l.map(|l| if i < 3 { Label { key: g, ..l } } else { l }))
            .collect();
        assert_eq!(key_run_lengths(&frames("CCCC"), &engine), vec![3, 3, 3, 1]);
    }

    #[test]
    fn changes_are_counted_between_labels() {
        assert_eq!(count_changes(frames("CCGGC").into_iter()), 2);
    }
}
