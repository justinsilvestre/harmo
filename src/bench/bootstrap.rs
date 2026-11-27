//! Paired piece-level bootstrap intervals for the change in accuracy from a previous run.

use super::metrics::FEATURES;
use super::report::{RunRecord, SetRecord};
use super::runner::PieceResult;
use std::collections::HashMap;

pub const RESAMPLES: usize = 1000;
const SEED: u64 = 1;

/// The frame-weighted features that get an interval, as indices into `FEATURES`:
/// rn, key, chord, offered, key now, rn now, rn eq and rn eq now.
const COLUMNS: [usize; 8] = [5, 0, 6, 8, 9, 10, 11, 12];

/// A piece present in both runs.
#[derive(Clone, Debug, PartialEq)]
pub struct PairedPiece {
    pub frames: f64,
    /// For each of `COLUMNS`, the change in rate times the piece's frames.
    pub weighted_changes: Vec<f64>,
}

/// A change in percentage points with its 95% percentile interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub change: f64,
    pub low: f64,
    pub high: f64,
}

/// Prints, for each set, the change in each of `COLUMNS` from `previous` with a 95% interval from
/// resampling the set's pieces, paired by id.
/// Pieces missing from either run are left out, and so are the columns that the previous run lacks.
pub fn print_intervals(sets: &[SetRecord], results: &[PieceResult], previous: &RunRecord) {
    let old_rates: HashMap<&str, &[f64]> =
        previous.pieces.iter().map(|p| (p.id.as_str(), p.rates.as_slice())).collect();
    let is_in_previous = |c: &&usize| previous.pieces.iter().all(|p| p.rates.len() > **c);
    let columns: Vec<usize> = COLUMNS.iter().filter(is_in_previous).copied().collect();
    let header: String = columns.iter().map(|&c| format!("{:>20}", FEATURES[c])).collect();
    println!("\n{:<28} {:>6}{header}", "bootstrap 95% vs previous", "pieces");
    for set in sets {
        let members = results.iter().filter(|r| set.name == "all" || r.sets.contains(&set.name));
        let pair_piece =
            |r: &PieceResult| pair(&r.accuracy.rates(), r.accuracy.frames, (&old_rates, &r.id), &columns);
        let pieces: Vec<PairedPiece> = members.filter_map(pair_piece).collect();
        if pieces.is_empty() {
            continue;
        }
        let intervals = bootstrap(&pieces, RESAMPLES, SEED);
        let cells: String = intervals.iter().map(format_interval).collect();
        println!("{:<28} {:>6}{cells}", set.name, pieces.len());
    }
}

fn pair(
    rates: &[f64],
    frames: u64,
    (old_rates, id): (&HashMap<&str, &[f64]>, &str),
    columns: &[usize],
) -> Option<PairedPiece> {
    let old = old_rates.get(id)?;
    if frames == 0 || columns.iter().any(|&c| old.get(c).is_none()) {
        return None;
    }
    let frames = frames as f64;
    let weighted_changes = columns.iter().map(|&c| (rates[c] - old[c]) * frames).collect();
    Some(PairedPiece { frames, weighted_changes })
}

/// Resamples `pieces` with replacement and returns, for each column, the frame-weighted change in
/// percentage points with the 2.5th and 97.5th percentiles of the resampled changes.
pub fn bootstrap(pieces: &[PairedPiece], resamples: usize, seed: u64) -> Vec<Interval> {
    let columns = pieces[0].weighted_changes.len();
    let mut rng = SplitMix64(seed);
    let mut samples: Vec<Vec<f64>> = vec![Vec::with_capacity(resamples); columns];
    let mut sums = vec![0.0; columns];
    for _ in 0..resamples {
        sums.iter_mut().for_each(|s| *s = 0.0);
        let mut frames = 0.0;
        for _ in 0..pieces.len() {
            let piece = &pieces[rng.below(pieces.len())];
            frames += piece.frames;
            sums.iter_mut().zip(&piece.weighted_changes).for_each(|(s, c)| *s += c);
        }
        samples.iter_mut().zip(&sums).for_each(|(column, s)| column.push(100.0 * s / frames));
    }
    let total_frames: f64 = pieces.iter().map(|p| p.frames).sum();
    let change = |c: usize| 100.0 * pieces.iter().map(|p| p.weighted_changes[c]).sum::<f64>() / total_frames;
    samples.into_iter().enumerate().map(|(c, column)| percentile_interval(change(c), column)).collect()
}

fn percentile_interval(change: f64, mut samples: Vec<f64>) -> Interval {
    samples.sort_by(f64::total_cmp);
    let index = |q: f64| ((q * samples.len() as f64) as usize).min(samples.len() - 1);
    Interval { change, low: samples[index(0.025)], high: samples[index(0.975).saturating_sub(1)] }
}

fn format_interval(interval: &Interval) -> String {
    let text = format!("{:+.2} [{:+.2},{:+.2}]", interval.change, interval.low, interval.high);
    format!("{text:>20}")
}

/// The SplitMix64 generator (Steele, Lea and Flood 2014), which is enough for resampling.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns a value in `0..n`, by taking the high bits of a 128-bit product.
    fn below(&mut self, n: usize) -> usize {
        ((self.next() as u128 * n as u128) >> 64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(frames: f64, change: f64) -> PairedPiece {
        PairedPiece { frames, weighted_changes: vec![change * frames] }
    }

    #[test]
    fn the_same_seed_gives_the_same_intervals() {
        let pieces = [piece(10.0, 0.1), piece(30.0, -0.2), piece(5.0, 0.05)];
        assert_eq!(bootstrap(&pieces, 200, 7), bootstrap(&pieces, 200, 7));
    }

    #[test]
    fn identical_changes_give_a_point_interval() {
        let pieces = [piece(10.0, 0.01), piece(30.0, 0.01)];
        let interval = bootstrap(&pieces, 100, 1)[0];
        assert!([interval.change, interval.low, interval.high].iter().all(|v| (v - 1.0).abs() < 1e-9));
    }

    #[test]
    fn the_change_is_weighted_by_frames() {
        let pieces = [piece(30.0, 0.04), piece(10.0, 0.0)];
        assert!((bootstrap(&pieces, 10, 1)[0].change - 3.0).abs() < 1e-9);
    }

    #[test]
    fn the_interval_spans_the_extremes_of_two_pieces() {
        let pieces = [piece(10.0, 0.0), piece(10.0, 0.02)];
        let interval = bootstrap(&pieces, 1000, 1)[0];
        assert_eq!((interval.low, interval.high), (0.0, 2.0));
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = SplitMix64(3);
        assert!((0..1000).all(|_| rng.below(7) < 7));
    }

    #[test]
    fn pieces_missing_from_the_previous_run_are_skipped() {
        let old_rates: HashMap<&str, &[f64]> = HashMap::new();
        assert_eq!(pair(&[0.5; 13], 8, (&old_rates, "a"), &COLUMNS), None);
    }
}
