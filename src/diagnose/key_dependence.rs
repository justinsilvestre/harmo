//! Chord accuracy split by whether the key is right, to show how much chord errors depend on key errors.

use crate::bench::metrics::feature_matches;
use crate::theory::label::Label;
use serde::Serialize;

/// Indices into the benchmark's feature flags: key, then quality, inversion, root and chord.
const KEY: usize = 0;
const CHORD_LAYER: [usize; 4] = [2, 3, 4, 6];

/// Frames and how many of them match on quality, inversion, root and chord.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct ChordLayerCounts {
    pub frames: u64,
    pub quality: u64,
    pub inversion: u64,
    pub root: u64,
    pub chord: u64,
}

impl ChordLayerCounts {
    fn add(&mut self, flags: [bool; 4]) {
        self.frames += 1;
        self.quality += flags[0] as u64;
        self.inversion += flags[1] as u64;
        self.root += flags[2] as u64;
        self.chord += flags[3] as u64;
    }

    fn merge(&mut self, other: &ChordLayerCounts) {
        self.frames += other.frames;
        self.quality += other.quality;
        self.inversion += other.inversion;
        self.root += other.root;
        self.chord += other.chord;
    }
}

/// The top reading's chord-layer matches against the main analysis, on frames with the key right and wrong.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct KeyDependence {
    pub key_right: ChordLayerCounts,
    pub key_wrong: ChordLayerCounts,
}

impl KeyDependence {
    pub fn merge(&mut self, other: &KeyDependence) {
        self.key_right.merge(&other.key_right);
        self.key_wrong.merge(&other.key_wrong);
    }

    /// Returns the share of frames with the chord wrong whose key is right, as a fraction.
    pub fn chord_wrong_key_right_share(&self) -> f64 {
        let right = self.key_right.frames - self.key_right.chord;
        let wrong = right + self.key_wrong.frames - self.key_wrong.chord;
        if wrong == 0 { 0.0 } else { right as f64 / wrong as f64 }
    }
}

/// Counts the frames that both the expert and the engine label.
pub fn tally_key_dependence(expert: &[Option<Label>], engine: &[Option<Label>]) -> KeyDependence {
    let mut tally = KeyDependence::default();
    for (expert, engine) in expert.iter().zip(engine) {
        let (Some(expert), Some(engine)) = (expert, engine) else {
            continue;
        };
        let flags = feature_matches(engine, expert);
        let side = if flags[KEY] { &mut tally.key_right } else { &mut tally.key_wrong };
        side.add(CHORD_LAYER.map(|i| flags[i]));
    }
    tally
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::tpc_from_name;
    use crate::theory::chord::{Chord, Quality};
    use crate::theory::key::Key;

    fn label(key: &str, root: &str) -> Option<Label> {
        let root = tpc_from_name(root).unwrap();
        Some(Label::new(Key::parse(key, "major").unwrap(), None, Chord::new(root, Quality::Major), root))
    }

    fn sample() -> KeyDependence {
        let expert = [label("C", "C"), label("C", "G"), label("C", "F"), label("C", "C"), None];
        let engine = [label("C", "C"), label("C", "D"), label("G", "F"), label("G", "D"), label("C", "C")];
        tally_key_dependence(&expert, &engine)
    }

    #[test]
    fn counts_chord_matches_where_the_key_is_right() {
        let right = sample().key_right;
        assert_eq!((right.frames, right.chord), (2, 1));
    }

    #[test]
    fn counts_chord_matches_where_the_key_is_wrong() {
        let wrong = sample().key_wrong;
        assert_eq!((wrong.frames, wrong.chord), (2, 1));
    }

    #[test]
    fn shares_chord_errors_by_key() {
        assert_eq!(sample().chord_wrong_key_right_share(), 0.5);
    }

    #[test]
    fn share_is_zero_without_chord_errors() {
        assert_eq!(KeyDependence::default().chord_wrong_key_right_share(), 0.0);
    }
}
