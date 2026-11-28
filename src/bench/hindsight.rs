//! The best final score that each hypothesis led to, for ranking readings in hindsight.
//!
//! This is a report on the finished analysis. No rule and no in-the-moment figure may use it.

use crate::engine::model::NONE;
use crate::engine::search::Analysis;

/// Returns, for every hypothesis, the best final score among the final hypotheses that
/// descend from it (itself included), or negative infinity when none survived to the end.
///
/// A hypothesis is added after its parent, so one sweep from the last hypothesis to the first
/// carries every final score back to all of its ancestors.
pub fn descendant_bests(analysis: &Analysis) -> Vec<f32> {
    let mut bests = vec![f32::NEG_INFINITY; analysis.layers.hypotheses.len()];
    for &(node, score) in &analysis.ranking {
        bests[node as usize] = bests[node as usize].max(score);
    }
    for index in (0..analysis.layers.hypotheses.len()).rev() {
        let parent = analysis.layers.hypotheses[index].parent;
        if parent != NONE {
            debug_assert!((parent as usize) < index, "a parent precedes its children");
            bests[parent as usize] = bests[parent as usize].max(bests[index]);
        }
    }
    bests
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ruleset::{RuleSet, Weights};
    use crate::engine::search::{Config, analyze};
    use crate::rules::default_rules;
    use crate::score::musicxml::read_score;
    use crate::score::slicing::slice_score;
    use std::path::Path;

    fn analysis() -> Analysis {
        let score = read_score(Path::new("tests/fixtures/cadence.musicxml")).unwrap();
        let rules = RuleSet::new(default_rules(), &Weights::new()).unwrap();
        analyze(&slice_score(&score), &rules, &Config { record_snapshots: true, ..Config::default() })
    }

    #[test]
    fn every_first_beam_node_leading_to_the_best_final_hypothesis_carries_its_score() {
        let analysis = analysis();
        let bests = descendant_bests(&analysis);
        let first =
            analysis.snapshots[0].iter().map(|&n| bests[n as usize]).fold(f32::NEG_INFINITY, f32::max);
        assert_eq!(first, analysis.ranking[0].1);
    }

    #[test]
    fn a_final_hypothesis_carries_its_own_score() {
        let analysis = analysis();
        let (node, score) = analysis.ranking[analysis.ranking.len() - 1];
        assert_eq!(descendant_bests(&analysis)[node as usize], score);
    }

    #[test]
    fn every_parent_precedes_its_children() {
        let analysis = analysis();
        let hypotheses = &analysis.layers.hypotheses;
        let precedes = |i: usize| hypotheses[i].parent == NONE || (hypotheses[i].parent as usize) < i;
        assert!((0..hypotheses.len()).all(precedes));
    }

    #[test]
    fn a_node_without_surviving_descendants_has_no_best() {
        let analysis = analysis();
        let bests = descendant_bests(&analysis);
        let pruned_away = analysis
            .snapshots
            .iter()
            .rev()
            .skip(1)
            .flatten()
            .any(|&n| bests[n as usize] == f32::NEG_INFINITY);
        assert!(pruned_away);
    }
}
