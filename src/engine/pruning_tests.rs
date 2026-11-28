use super::*;
use crate::engine::hypothesis::Hypothesis;
use crate::engine::model::{
    ChordHypothesis, ChordHypothesisId, ChordSegment, Function, KeyHypothesisId, Reading,
};
use crate::engine::pending::{PendingChordHypothesisId, Proposed};
use crate::engine::test_layers::key_segment;
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

/// A pending hypothesis reading G in C major, after hypothesis `previous`,
/// that reads pending chord hypothesis `chord_hypothesis`.
fn to_g(previous: u32, chord_hypothesis: u32, score: f32) -> PendingKeyHypothesis {
    let (unit, reads) = (reading("G").function, PendingChordHypothesisId::new(chord_hypothesis));
    let previous = KeyHypothesisId::new(previous);
    Hypothesis { parent: previous, previous, reads, unit, score }
}

/// Two pending hypotheses that move to G after hypotheses 0 and 1 of the beam respectively,
/// with scores -2 and -1. Each reads a pending chord hypothesis of its own: the first opens on slice 1,
/// the second on slice 2.
fn fixture() -> Pending {
    let chord_hypothesis = |previous: u32, start: u32| ChordHypothesis {
        parent: ChordHypothesisId::new(previous),
        previous: ChordHypothesisId::new(previous),
        reads: start,
        unit: ChordSegment { start, ..key_segment(reading("G")).heard },
        score: 0.0,
    };
    let chord = Proposed { hypotheses: vec![chord_hypothesis(0, 1), chord_hypothesis(1, 2)] };
    let key = Proposed { hypotheses: vec![to_g(0, 0, -2.0), to_g(1, 1, -1.0)] };
    Pending { chord, key }
}

/// Returns the scores of the hypotheses that pruning keeps, and the causes of those it drops.
fn pruned(pending: Pending, config: &Config) -> (Vec<f32>, Vec<PruneCause>) {
    let (kept, dropped) = prune(&pending, config);
    (kept.iter().map(|h| h.score).collect(), dropped.iter().map(|p| p.cause).collect())
}

/// Returns the start of the last segment of each hypothesis that pruning keeps.
fn kept_starts(pending: Pending, config: &Config) -> Vec<u32> {
    let (kept, _) = prune(&pending, config);
    kept.iter().map(|h| pending.chord.get(h.reads).unit.start).collect()
}

fn dropped_causes(pending: Pending, config: &Config) -> Vec<PruneCause> {
    pruned(pending, &Config { record_pruning: true, ..config.clone() }).1
}

#[test]
fn prune_keeps_hypotheses_that_differ_only_in_where_their_segments_begin() {
    assert_eq!(kept_starts(fixture(), &Config::default()), [2, 1]);
}

#[test]
fn prune_records_hypotheses_cut_by_beam_width() {
    let config = Config { beam_width: 1, ..Config::default() };
    assert_eq!(dropped_causes(fixture(), &config), [PruneCause::Width { layer: Layer::Key }]);
}

#[test]
fn prune_keeps_at_most_beam_width_hypotheses() {
    let config = Config { beam_width: 1, ..Config::default() };
    assert_eq!(kept_starts(fixture(), &config), [2]);
}

/// The fixture with a third pending hypothesis, the worst, that reads the same chord hypothesis as the best.
fn sharing() -> Pending {
    let mut pending = fixture();
    pending.key.hypotheses.push(to_g(1, 1, -3.0));
    pending
}

/// The search with room for one chord hypothesis in the beam.
fn one_chord_hypothesis() -> Config {
    Config { chord_hypothesis_width: 1, ..Config::default() }
}

#[test]
fn prune_records_hypotheses_cut_by_the_chord_hypothesis_limit() {
    assert_eq!(
        dropped_causes(sharing(), &one_chord_hypothesis()),
        [PruneCause::Width { layer: Layer::Chord }]
    );
}

#[test]
fn prune_keeps_hypotheses_that_read_a_kept_chord_hypothesis_beyond_the_chord_hypothesis_limit() {
    assert_eq!(pruned(sharing(), &one_chord_hypothesis()).0, [-1.0, -3.0]);
}
