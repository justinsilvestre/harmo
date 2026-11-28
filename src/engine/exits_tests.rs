use super::*;
use crate::engine::key_hooks::{KeyHook, Rival};
use crate::engine::model::{Function, NONE, Reading};
use crate::engine::proposals::LayerScores;
use crate::engine::rule::{Hook, Rule, Topic};
use crate::engine::test_layers::{chord_segment, key_segment, layers_of, slice_of};
use crate::pitch::tpc_from_name;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::{Key, Mode};

fn reading(root: &str) -> Reading {
    let key = Key::new(tpc_from_name("C").unwrap(), Mode::Major);
    Reading {
        chord: Chord::new(tpc_from_name(root).unwrap(), Quality::Major),
        function: Function::diatonic(key),
    }
}

/// Puts every segment in one rival group, as a member named by its chord.
fn same_group(context: &RivalContext, _: &[f64]) -> Option<Rival> {
    let chord = context.segment.heard.chord;
    Some(Rival { group: 0, member: (chord.root as u64) << 8 | chord.quality as u64 })
}

fn rules(rival_hook: bool) -> RuleSet {
    let rule = Rule {
        id: "test.rival",
        topic: Topic::Chord,
        summary: "",
        citations: &[],
        default_weight: 1.0,
        params: &[],
        hook: Hook::Key(KeyHook::Rival(same_group)),
    };
    RuleSet::new(if rival_hook { vec![rule] } else { vec![] }, &Default::default()).unwrap()
}

/// Returns the parents that a new reading of C on slice 3 opens after, from a beam of two
/// hypotheses whose last segments end on slice 2 with the given readings and scores.
fn parents(rival_hook: bool, readings: [Reading; 2], scores: [f32; 2]) -> Vec<u32> {
    let rules = rules(rival_hook);
    let mut layers = layers_of(&readings.map(key_segment), &[NONE, NONE]);
    for (i, score) in scores.into_iter().enumerate() {
        (layers.hypotheses[i].score, layers.hearings[i].slice) = (score, 2);
    }
    let slices = vec![slice_of(0.0, &[]); 4];
    let moment = Moment { slices: &slices, index: 3, layers: &layers };
    let opening = Opening { reading: reading("C"), fit: LayerScores::default(), chord_index: 0 };
    let proposed = [ProposedChord {
        chord: opening.reading.chord,
        fit: 0.0,
        segment: chord_segment(opening.reading.chord),
    }];
    let exits = Exits::of(&moment, &[0, 1], &proposed, &rules);
    let found = exits.predecessors(&opening, &moment, &rules);
    found.iter().map(|p| exits.keys[p.exit].hypothesis).collect()
}

#[test]
fn a_new_segment_opens_after_the_best_hypothesis_alone() {
    assert_eq!(parents(false, [reading("G"), reading("F")], [0.0, -1.0]), [0]);
}

#[test]
fn a_new_segment_also_opens_after_the_best_rival_reading() {
    assert_eq!(parents(true, [reading("G"), reading("F")], [0.0, -1.0]), [0, 1]);
}

#[test]
fn a_rival_needs_another_reading() {
    assert_eq!(parents(true, [reading("G"), reading("G")], [0.0, -1.0]), [0]);
}

#[test]
fn the_same_chord_in_another_key_is_no_rival() {
    let in_g = Reading {
        function: Function::diatonic(Key::new(tpc_from_name("G").unwrap(), Mode::Major)),
        ..reading("G")
    };
    assert_eq!(parents(true, [reading("G"), in_g], [0.0, -1.0]), [0]);
}
