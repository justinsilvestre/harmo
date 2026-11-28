//! Tracks how readings' standing in the layers' rankings changes as the music unfolds.

use super::model::{ChordHypothesisId, ChordSegment, Function, KeyHypothesisId, KeySegment, Reading};
use super::pruning::{Departure, PruneCause};
use super::rank_history::{Holders, RankHistory, follow, follow_each};
use super::search::Analysis;
use crate::pitch::Tpc;
use crate::theory::chord::Chord;

/// The rank history of one whole reading of a slice (its chord and function), followed through the beam.
pub type ReadingHistory = RankHistory<Reading, Function>;

/// The chord layer's reading of a slice: its chord, and the spelling of the bass that its segment ends with,
/// or has heard so far if the segment is still open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChordReading {
    pub chord: Chord,
    pub bass: Tpc,
}

/// The rank histories of each layer's readings of one slice, each followed through its own layer's rankings.
pub struct LayerRankHistories {
    pub chord: Vec<RankHistory<ChordReading, ChordSegment>>,
    /// The key layer's histories, which record why a function left the beam.
    pub key: Vec<RankHistory<Function, Function>>,
}

/// Follows every reading that the beam holds for slice `at` through the later beams,
/// in order of their rank at `at`.
pub fn rank_histories(analysis: &Analysis, at: usize) -> Vec<ReadingHistory> {
    let reading_of =
        |hypothesis| reading_at(analysis, hypothesis, at).expect("a hypothesis covers its own slice");
    follow(&analysis.layers.key, at, reading_of, |slice, last_rank, holders| {
        departure(analysis, slice, last_rank, holders)
    })
}

/// Follows each layer's readings of slice `at` through that layer's later rankings.
pub fn layer_rank_histories(analysis: &Analysis, at: usize) -> LayerRankHistories {
    let chord_reading_of = |hypothesis| {
        let reading =
            chord_reading_at(analysis, hypothesis, at).expect("a chord hypothesis covers the slice");
        (reading, analysis.layers.chord.get(hypothesis).unit.start as usize > at)
    };
    let function_of = |hypothesis| {
        segment_at(analysis, hypothesis, at).expect("a hypothesis covers its own slice").function
    };
    LayerRankHistories {
        chord: follow_each(&analysis.layers.chord, at, chord_reading_of),
        key: follow(&analysis.layers.key, at, function_of, |slice, last_rank, holders| {
            departure(analysis, slice, last_rank, holders)
        }),
    }
}

/// Explains why a reading left the beam at `slice`, from the best pruned hypothesis that held it.
/// Returns `None` unless the analysis recorded pruning.
fn departure(
    analysis: &Analysis,
    slice: usize,
    last_rank: usize,
    holders: &Holders<Function>,
) -> Option<Departure> {
    let pruned = analysis.pruned.get(slice)?.iter().find(|p| holders.holds(p.parent))?;
    let beam = analysis.layers.key.ranking(slice).expect("the slice is stored");
    let best = beam.first().map_or(pruned.score, |first| analysis.layers.score(first));
    let keeper = match pruned.cause {
        PruneCause::Merged { keeper } => Some(keeper),
        PruneCause::Width { .. } => None,
    };
    Some(Departure {
        last_rank,
        competing_rank: pruned.rank,
        score_gap: best - pruned.score,
        cause: pruned.cause.name(),
        merged_into_rank: keeper.map(|k| beam.position(k).expect("a keeper is in the beam") + 1),
        merged_into: keeper,
    })
}

/// Returns the reading that the hypothesis ending at `hypothesis` assigns to slice `at`.
pub fn reading_at(analysis: &Analysis, hypothesis: KeyHypothesisId, at: usize) -> Option<Reading> {
    segment_at(analysis, hypothesis, at).map(|s| s.reading())
}

/// Returns the chord layer's reading of slice `at` by the chord hypothesis ending at `hypothesis`,
/// with the bass of the segment holding `at` as of the slice the hypothesis ends on.
pub fn chord_reading_at(
    analysis: &Analysis,
    hypothesis: ChordHypothesisId,
    at: usize,
) -> Option<ChordReading> {
    let chord = &analysis.layers.chord;
    let mut segments = chord.path_back(hypothesis).map(|last| chord.get(last).unit);
    let segment = segments.find(|s| s.start as usize <= at)?;
    Some(ChordReading { chord: segment.chord, bass: segment.bass_tpc() })
}

/// Returns the segment that the hypothesis ending at `hypothesis` assigns to slice `at`.
pub fn segment_at(analysis: &Analysis, hypothesis: KeyHypothesisId, at: usize) -> Option<KeySegment> {
    let layers = &analysis.layers;
    layers.key.path_back(hypothesis).map(|last| layers.segment(last)).find(|s| s.heard.start as usize <= at)
}
