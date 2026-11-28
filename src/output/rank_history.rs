//! Rank histories in the analysis document, with an account of when and why a reading left the beam.

use super::analysis::{ReadingOutput, reading_output};
use crate::engine::history::{RankChange, ReadingHistory, segment_at};
use crate::engine::pruning::Departure;
use crate::engine::search::Analysis;
use crate::score::model::Score;
use crate::score::slicing::{Slice, measure_at};
use crate::theory::label::Label;
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

/// What converting a rank history needs: the slice whose reading is followed, and the music.
pub struct HistoryContext<'a> {
    pub at: usize,
    pub score: &'a Score,
    pub slices: &'a [Slice],
    pub analysis: &'a Analysis,
}

pub fn rank_history_output(history: &ReadingHistory, context: &HistoryContext) -> Vec<RankChangeOutput> {
    history.changes.iter().map(|change| rank_change_output(change, context)).collect()
}

/// Lists, best first, up to `count` readings of the span's first slice that left the beam.
pub fn pruned_alternatives(
    histories: &[ReadingHistory],
    context: &HistoryContext,
    count: usize,
) -> Vec<PrunedAlternativeOutput> {
    let has_left = |h: &&ReadingHistory| h.changes.last().is_some_and(|c| c.rank.is_none());
    let output = |history: &ReadingHistory| PrunedAlternativeOutput {
        reading: held_reading(history.node, context).expect("a hypothesis covers its own slice"),
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

/// Returns the reading that the hypothesis ending at `node` gives the followed slice.
fn held_reading(node: u32, context: &HistoryContext) -> Option<ReadingOutput> {
    let segment = segment_at(context.analysis, node, context.at)?;
    let reading = segment.reading;
    let label = Label::new(reading.key, reading.function.tonicized, reading.chord, segment.bass_tpc());
    Some(reading_output(&label, reading.function.category))
}
