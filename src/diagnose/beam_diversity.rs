//! Measures how varied the beam is, to show whether near-duplicate hypotheses crowd it.

use crate::engine::model::Segment;
use crate::engine::search::Analysis;
use crate::theory::label::Label;
use serde::Serialize;
use std::collections::HashSet;

/// Sums over slices of how varied the beam is.
#[derive(Clone, Debug, Default, Serialize)]
pub struct BeamTally {
    pub slices: u64,
    pub hypotheses: u64,
    /// Distinct full readings (key, numeral, quality, inversion) held at the slice.
    pub readings: u64,
    /// Distinct pairs of chord and key held at the slice.
    pub chord_keys: u64,
    /// Distinct full readings held by the leading hypotheses, as many as may be offered.
    pub leading_readings: u64,
}

impl BeamTally {
    pub fn merge(&mut self, other: &BeamTally) {
        self.slices += other.slices;
        self.hypotheses += other.hypotheses;
        self.readings += other.readings;
        self.chord_keys += other.chord_keys;
        self.leading_readings += other.leading_readings;
    }
}

/// Counts, at every slice, the hypotheses in the beam and their distinct readings of that slice,
/// among all hypotheses and among the first `leading` ones.
/// Requires an analysis run with `Config::record_snapshots`.
pub fn tally_beam(analysis: &Analysis, leading: usize) -> BeamTally {
    let mut tally = BeamTally::default();
    for beam in &analysis.snapshots {
        let segments: Vec<Segment> = beam.iter().map(|&node| analysis.segment(node)).collect();
        let readings: HashSet<Label> = segments.iter().map(segment_label).collect();
        let chord_keys: HashSet<_> = segments.iter().map(|s| (s.reading.chord, s.reading.key)).collect();
        let leading_readings: HashSet<Label> = segments.iter().take(leading).map(segment_label).collect();
        tally.slices += 1;
        tally.hypotheses += beam.len() as u64;
        tally.readings += readings.len() as u64;
        tally.chord_keys += chord_keys.len() as u64;
        tally.leading_readings += leading_readings.len() as u64;
    }
    tally
}

/// Returns the label of a segment, with the inversion given by the lowest chord tone heard so far.
pub fn segment_label(segment: &Segment) -> Label {
    let reading = segment.reading;
    Label::new(reading.key, reading.function.tonicized, reading.chord, segment.bass_tpc())
}
