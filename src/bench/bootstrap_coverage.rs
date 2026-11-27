//! Bootstrap intervals for the change in how often both analysts' readings are offered.

use super::bootstrap::{PairedPiece, RESAMPLES, bootstrap};
use super::report::{PieceCoverage, RunRecord, SetRecord};
use super::runner::PieceResult;
use std::collections::HashMap;

const SEED: u64 = 2;

/// The coverage columns that get an interval, as indices into the agreement features: key, rn,
/// chord and rn eq.
const COLUMNS: [(usize, &str); 4] = [(0, "key"), (5, "rn"), (6, "chord"), (7, "rn eq")];

/// Prints, for each set with two analyses, the change in "both readings offered" in the key, rn,
/// chord and rn eq columns, with a 95% interval from resampling the set's pieces, paired by id, and the mean
/// number of readings offered in both runs.
pub fn print_coverage_intervals(sets: &[SetRecord], results: &[PieceResult], previous: &RunRecord) {
    let old: HashMap<&str, &PieceCoverage> =
        previous.pieces.iter().filter_map(|p| Some((p.id.as_str(), p.coverage.as_ref()?))).collect();
    let old_sets: HashMap<&str, &SetRecord> = previous.sets.iter().map(|s| (s.name.as_str(), s)).collect();
    for set in sets.iter().filter(|s| s.agreement.is_some()) {
        let members: Vec<&PieceResult> =
            results.iter().filter(|r| set.name == "all" || r.sets.contains(&set.name)).collect();
        let cells: Vec<String> = COLUMNS
            .iter()
            .filter_map(|&(feature, name)| {
                let pieces = paired_pieces(&members, &old, feature);
                if pieces.is_empty() {
                    return None;
                }
                let interval = bootstrap(&pieces, RESAMPLES, SEED).into_iter().next()?;
                Some(format!("{name} {:+.2} [{:+.2},{:+.2}]", interval.change, interval.low, interval.high))
            })
            .collect();
        if cells.is_empty() {
            continue;
        }
        let offered = |s: &SetRecord| s.agreement.as_ref().map_or(f64::NAN, |a| a.mean_offered);
        let before = old_sets.get(set.name.as_str()).map_or(f64::NAN, |s| offered(s));
        println!(
            "both offered, 95% vs previous: {:<22} {}  at n offered {:.3} (previous {before:.3})",
            set.name,
            cells.join("  "),
            offered(set)
        );
    }
}

/// Pairs each piece's coverage of `feature` with the previous run's, leaving out pieces missing
/// from either run, or without the feature in the previous run, or without disagreements on it.
pub fn paired_pieces(
    members: &[&PieceResult],
    old: &HashMap<&str, &PieceCoverage>,
    feature: usize,
) -> Vec<PairedPiece> {
    let pair = |r: &&PieceResult| {
        let agreement = r.agreement.as_ref()?;
        let previous = old.get(r.id.as_str())?.offered_both.get(feature)?;
        let disagreements = agreement.disagreements()[feature];
        (disagreements > 0).then(|| PairedPiece {
            frames: disagreements as f64,
            weighted_changes: vec![agreement.offered_both[feature] as f64 - *previous as f64],
        })
    };
    members.iter().filter_map(pair).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::metrics::{AGREEMENT_FEATURES, Accuracy, Agreement, MATCHED_FEATURES};

    fn result(id: &str, frames: u64, offered_both: u64) -> PieceResult {
        let agreement =
            Agreement { frames, offered_both: [offered_both; AGREEMENT_FEATURES], ..Agreement::default() };
        PieceResult {
            id: id.into(),
            sets: vec![],
            slices: 0,
            seconds: 0.0,
            accuracy: Accuracy::default(),
            agreement: Some(agreement),
            offer_delays: None,
        }
    }

    fn coverage(offered_both: u64) -> PieceCoverage {
        PieceCoverage {
            disagreements: vec![10; MATCHED_FEATURES],
            offered_both: vec![offered_both; MATCHED_FEATURES],
        }
    }

    #[test]
    fn a_piece_pairs_its_change_in_offered_frames_with_its_disagreements() {
        let new = result("a", 10, 7);
        let old_coverage = coverage(4);
        let old = HashMap::from([("a", &old_coverage)]);
        let pieces = paired_pieces(&[&new], &old, 5);
        assert_eq!(pieces, vec![PairedPiece { frames: 10.0, weighted_changes: vec![3.0] }]);
    }

    #[test]
    fn pieces_without_disagreements_are_left_out() {
        let new = result("a", 0, 0);
        let old_coverage = coverage(0);
        let old = HashMap::from([("a", &old_coverage)]);
        assert!(paired_pieces(&[&new], &old, 5).is_empty());
    }

    #[test]
    fn a_feature_missing_from_the_previous_run_is_left_out() {
        let new = result("a", 10, 7);
        let old_coverage = coverage(4);
        let old = HashMap::from([("a", &old_coverage)]);
        assert!(paired_pieces(&[&new], &old, MATCHED_FEATURES).is_empty());
    }

    #[test]
    fn pieces_missing_from_the_previous_run_are_left_out() {
        let new = result("a", 10, 7);
        assert!(paired_pieces(&[&new], &HashMap::new(), 5).is_empty());
    }
}
