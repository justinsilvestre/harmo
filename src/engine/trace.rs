//! Explains a hypothesis by re-evaluating each rule along its path.
//!
//! Rules are pure, so the search stores only scores and the trace is rebuilt on demand.

use super::chord_trace::ChordTrace;
use super::key_trace::KeyTrace;
use super::path::SegmentReading;
use super::rule::{Hook, Topic};
use super::ruleset::RuleSet;
use super::search::Analysis;
use crate::score::slicing::Slice;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Contribution {
    pub rule: &'static str,
    pub hook: &'static str,
    pub topic: Topic,
    /// The rule's unweighted evidence, summed over the segment.
    pub raw: f32,
    pub weighted: f32,
}

/// Returns each scoring rule's contribution to one segment of a hypothesis, in rule order.
/// Each rule is evaluated again on the hypothesis of its own layer that the segment ends in.
pub fn trace_segment(
    analysis: &Analysis,
    slices: &[Slice],
    rules: &RuleSet,
    segment: &SegmentReading,
) -> Vec<Contribution> {
    let layers = &analysis.layers;
    let chord = ChordTrace::of(layers, slices, layers.key.get(segment.hypothesis).reads, segment.slices);
    let key = KeyTrace::of(layers, slices, segment.hypothesis, segment.slices);
    let mut contributions = Vec::new();
    let rules_with_params = rules.rules.iter().zip(&rules.weights).zip(&rules.params);
    for ((rule, &weight), params) in rules_with_params.filter(|((_, w), _)| **w != 0.0) {
        let raw = match rule.hook {
            Hook::Chord(hook) => chord.raw(hook, params),
            Hook::Key(hook) => key.raw(hook, params),
        };
        let Some(raw) = raw else {
            continue;
        };
        contributions.push(Contribution {
            rule: rule.id,
            hook: rule.hook.kind(),
            topic: rule.topic,
            raw,
            weighted: raw * weight,
        });
    }
    contributions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::exits::{chord_close, key_close};
    use crate::engine::model::KeyHypothesisId;
    use crate::engine::path::segments_of;
    use crate::engine::rule::Layer;
    use crate::engine::search::{Config, analyze};
    use crate::engine::test_layers::i_v_i;

    /// For each final hypothesis of I–V–I,
    /// returns its trace's weighted sum in the given layer beside the score it should equal:
    /// the ranking score for both layers together,
    /// or for one layer, that layer's hypothesis's score plus the close of its last segment.
    fn sums(layer: Option<Layer>) -> Vec<(f32, f32)> {
        let (slices, rules) =
            (i_v_i(), RuleSet::new(crate::rules::default_rules(), &Default::default()).unwrap());
        let analysis = analyze(&slices, &rules, &Config::default());
        let layer_of = |id: &str| rules.rules.iter().find(|r| r.id == id).unwrap().hook.layer();
        let traced = |hypothesis: KeyHypothesisId| -> f32 {
            let segments = segments_of(&analysis, &slices, hypothesis);
            let contributions = segments.iter().flat_map(|s| trace_segment(&analysis, &slices, &rules, s));
            contributions.filter(|c| layer.is_none_or(|l| layer_of(c.rule) == l)).map(|c| c.weighted).sum()
        };
        let expected = |&(hypothesis, score): &(KeyHypothesisId, f32)| match layer {
            Some(Layer::Chord) => {
                let chord_hypothesis = analysis.layers.chord_hypothesis_of(hypothesis);
                chord_hypothesis.score + chord_close(&slices, &chord_hypothesis.unit, slices.len(), &rules)
            }
            Some(Layer::Key) => {
                let segment = analysis.layers.segment(hypothesis);
                let own = analysis.layers.key.get(hypothesis).score;
                own + key_close(&slices, &segment, slices.len(), &rules)
            }
            None => score,
        };
        analysis.ranking.iter().map(|entry| (traced(entry.0), expected(entry))).collect()
    }

    fn agree(pairs: &[(f32, f32)]) -> bool {
        !pairs.is_empty()
            && pairs.iter().all(|(traced, score)| (traced - score).abs() < 1e-3 * score.abs().max(1.0))
    }

    #[test]
    fn a_trace_sums_to_the_ranking_score_of_every_final_hypothesis() {
        assert!(agree(&sums(None)));
    }

    #[test]
    fn the_chord_layer_of_a_trace_sums_to_the_chord_hypothesis_score_and_its_last_close() {
        assert!(agree(&sums(Some(Layer::Chord))));
    }

    #[test]
    fn the_key_layer_of_a_trace_sums_to_the_key_hypothesis_score_and_its_last_close() {
        assert!(agree(&sums(Some(Layer::Key))));
    }
}
