use super::*;
use crate::engine::key_hooks::{KeyHook, Rival, RivalContext};
use crate::engine::model::{Function, NONE, Reading};
use crate::engine::pending::PendingHypothesis;
use crate::engine::rule::{Hook, Rule, Topic};
use crate::engine::test_layers::{i_v_i, key_segment, layers_of, slice_of};
use crate::pitch::tpc_from_name;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::{Key, Mode};

fn in_c(root: &str, quality: Quality) -> Reading {
    let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
    Reading { chord: Chord::new(tpc_from_name(root).unwrap(), quality), function: Function::diatonic(key) }
}

/// Analyzes I–V–I in C major, with the given rule weights.
fn analyze_i_v_i(overrides: &[(&str, f32)]) -> Analysis {
    let weights = overrides.iter().map(|(id, weight)| (id.to_string(), *weight)).collect();
    let rules = RuleSet::new(crate::rules::default_rules(), &weights).unwrap();
    analyze(&i_v_i(), &rules, &Config::default())
}

/// The ids of the default rules of the key layer that score, so that they can be switched off.
fn key_scoring_rules() -> Vec<(&'static str, f32)> {
    let scores = |rule: &&Rule| {
        use KeyHook::{Close, Departure, Slice, Transition};
        matches!(rule.hook, Hook::Key(Slice(_) | Transition(_) | Departure(_) | Close(_)))
    };
    crate::rules::default_rules().iter().filter(scores).map(|rule| (rule.id, 0.0)).collect()
}

#[test]
fn without_key_scores_a_hypothesis_scores_exactly_its_hearing() {
    let layers = analyze_i_v_i(&key_scoring_rules()).layers;
    let differs = |h: &Hypothesis| h.score != layers.hearing(h.hearing).score;
    assert!(!layers.hypotheses.iter().any(differs));
}

/// Puts every segment in one rival group, as a member named by its chord.
fn same_group(context: &RivalContext, _: &[f64]) -> Option<Rival> {
    let chord = context.segment.heard.chord;
    Some(Rival { group: 0, member: (chord.root as u64) << 8 | chord.quality as u64 })
}

/// The default rules, with a rival hook that puts every segment in the same group if `rival_hook`.
fn rules_with(rival_hook: bool) -> RuleSet {
    let rival = Rule {
        id: "test.rival",
        topic: Topic::Chord,
        summary: "",
        citations: &[],
        default_weight: 1.0,
        params: &[],
        hook: Hook::Key(KeyHook::Rival(same_group)),
    };
    let rules = crate::rules::default_rules().into_iter().chain(rival_hook.then_some(rival));
    RuleSet::new(rules.collect(), &Default::default()).unwrap()
}

/// Proposes the hypotheses of a C major slice after a beam that holds the given readings of the
/// slice before, best first.
fn step_after(readings: &[Reading], rules: &RuleSet) -> Pending {
    let previous = vec![NONE; readings.len()];
    let mut layers = layers_of(&readings.iter().map(|&r| key_segment(r)).collect::<Vec<_>>(), &previous);
    for (i, hypothesis) in layers.hypotheses.iter_mut().enumerate() {
        hypothesis.score = -(i as f32);
    }
    let slices = [
        slice_of(0.0, &[("G", 43), ("B", 62), ("D", 67)]),
        slice_of(1.0, &[("C", 48), ("E", 64), ("G", 67)]),
    ];
    let beam: Vec<u32> = (0..readings.len() as u32).collect();
    step(&Moment { slices: &slices, index: 1, layers: &layers }, &beam, rules, &Config::default())
}

/// Returns the pending hearing of each hypothesis that opens a C major chord after `parent`.
fn c_major_hearings(pending: &Pending, parent: u32) -> Vec<PendingHearingId> {
    let c_major = Chord::new(tpc_from_name("C").unwrap(), Quality::Major);
    let opens_c = |h: &&PendingHypothesis| {
        h.parent == parent && h.previous == parent && pending.hearing(h.hearing).segment.chord == c_major
    };
    pending.hypotheses.iter().filter(opens_c).map(|h| h.hearing).collect()
}

#[test]
fn readings_of_one_chord_after_one_hearing_share_a_hearing() {
    let pending = step_after(&[in_c("G", Quality::Major)], &rules_with(false));
    let hearings = c_major_hearings(&pending, 0);
    assert!(hearings.len() > 1 && hearings.iter().all(|&h| h == hearings[0]));
}

#[test]
fn a_new_segment_after_a_rival_has_a_hearing_of_its_own() {
    let rules = rules_with(true);
    let pending = step_after(&[in_c("G", Quality::Major), in_c("G", Quality::DominantSeventh)], &rules);
    let (after_best, after_rival) = (c_major_hearings(&pending, 0), c_major_hearings(&pending, 1));
    assert!(!after_rival.is_empty() && after_rival.iter().all(|h| !after_best.contains(h)));
}
