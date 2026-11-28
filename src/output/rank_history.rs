//! Rank histories in the analysis document, with an account of when and why a reading left the beam.

use super::analysis::{ReadingOutput, reading_output};
use crate::engine::history::{ReadingHistory, chord_reading_at, layer_rank_histories, segment_at};
use crate::engine::model::KeyHypothesisId;
use crate::engine::path::SegmentReading;
use crate::engine::pruning::Departure;
use crate::engine::rank_history::{RankChange, RankHistory};
use crate::engine::search::Analysis;
use crate::score::model::Score;
use crate::score::slicing::{Slice, measure_at};
use serde::Serialize;

#[derive(Serialize)]
pub struct RankChangeOutput {
    pub slice: usize,
    /// The rank (1 = best) of the best hypothesis holding the reading, or `None` once it left the beam.
    pub rank: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_beam: Option<DepartureOutput>,
}

/// When a reading left the beam, at what rank it last stood, how far behind the best it was and why.
#[derive(Serialize)]
pub struct DepartureOutput {
    /// Onset of the slice at which it was pruned, in quarter notes.
    pub time: f64,
    pub measure: String,
    #[serde(flatten)]
    pub departure: Departure,
    /// For `distinct_depth`, the reading that the hypothesis kept in its place gives the span.
    pub merged_into_reading: Option<ReadingOutput>,
}

/// A reading that the beam held for a span's first slice but later dropped.
#[derive(Serialize)]
pub struct PrunedAlternativeOutput {
    pub reading: ReadingOutput,
    pub rank_history: Vec<RankChangeOutput>,
}

/// The rank histories of a span's reading in each layer, named for the layer's unit:
/// its chord and bass in the chord layer's rankings, and its function in the key layer's.
#[derive(Serialize)]
pub struct RankHistoriesByLayerOutput {
    pub chord: Vec<RankChangeOutput>,
    pub key: Vec<RankChangeOutput>,
}

/// What converting a rank history needs: the slice whose reading is followed, and the music.
pub struct HistoryContext<'a> {
    pub at: usize,
    pub score: &'a Score,
    pub slices: &'a [Slice],
    pub analysis: &'a Analysis,
}

pub fn rank_history_output<T, U>(
    history: &RankHistory<T, U>,
    context: &HistoryContext,
) -> Vec<RankChangeOutput> {
    history.changes.iter().map(|change| rank_change_output(change, context)).collect()
}

/// Gives the rank histories of the span's reading of its first slice in each layer.
/// The chord layer's reading has the span's final bass, and so its inversion.
pub fn rank_histories_by_layer(
    segment: &SegmentReading,
    context: &HistoryContext,
) -> RankHistoriesByLayerOutput {
    let (analysis, at) = (context.analysis, context.at);
    let histories = layer_rank_histories(analysis, at);
    let last = analysis.layers.key.get(segment.hypothesis).reads;
    let reading = chord_reading_at(analysis, last, at);
    let chord_history = histories.chord.iter().find(|h| Some(h.reading) == reading);
    let key_history = histories.key.iter().find(|h| h.reading == segment.reading.function);
    RankHistoriesByLayerOutput {
        chord: chord_history.map_or(vec![], |h| rank_history_output(h, context)),
        key: key_history.map_or(vec![], |h| rank_history_output(h, context)),
    }
}

/// Lists, best first, up to `count` readings of the span's first slice that left the beam.
pub fn pruned_alternatives(
    histories: &[ReadingHistory],
    context: &HistoryContext,
    count: usize,
) -> Vec<PrunedAlternativeOutput> {
    let has_left = |h: &&ReadingHistory| h.changes.last().is_some_and(|c| c.rank.is_none());
    let output = |history: &ReadingHistory| PrunedAlternativeOutput {
        reading: held_reading(history.hypothesis, context).expect("a hypothesis covers its own slice"),
        rank_history: rank_history_output(history, context),
    };
    histories.iter().filter(has_left).take(count).map(output).collect()
}

fn rank_change_output(change: &RankChange, context: &HistoryContext) -> RankChangeOutput {
    let departure_output = |departure: &Departure| {
        let start = context.slices[change.slice].start;
        let measures = &context.score.measures;
        DepartureOutput {
            time: start,
            measure: measures[measure_at(measures, start)].number.clone(),
            departure: departure.clone(),
            merged_into_reading: departure.merged_into.and_then(|keeper| held_reading(keeper, context)),
        }
    };
    RankChangeOutput {
        slice: change.slice,
        rank: change.rank,
        left_beam: change.left_beam.as_ref().map(departure_output),
    }
}

/// Returns the reading that the hypothesis ending at `hypothesis` gives the followed slice.
fn held_reading(hypothesis: KeyHypothesisId, context: &HistoryContext) -> Option<ReadingOutput> {
    let segment = segment_at(context.analysis, hypothesis, context.at)?;
    Some(reading_output(&segment.label(), segment.function.category))
}
