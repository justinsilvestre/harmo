//! Explains why no hypothesis in the beam holds an analyst's reading, from the hypotheses the
//! search pruned.

use super::beam_diversity::segment_label;
use crate::bench::metrics::is_rn_match;
use crate::engine::history::segment_at;
use crate::engine::model::Segment;
use crate::engine::pruning::{PruneCause, Pruned};
use crate::engine::search::Analysis;
use crate::score::slicing::Slice;
use crate::theory::label::Label;

/// Why a reading is missing from the beam from which readings are offered.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Absence {
    /// The last hypotheses holding it were pruned because the beam was full of better ones.
    /// `after` is how many quarter notes after the frame's slice began they were pruned.
    Width { after: f64 },
    /// The last hypotheses holding it were pruned in favour of better ones that give their recent
    /// segments the same readings with other boundaries (see `Config::distinct_depth`).
    Merged { after: f64 },
    /// No hypothesis that reached the frame's slice gave it the reading.
    NeverHeld,
}

/// Finds where the last hypotheses giving slice `at` the label were pruned, between `at` and the
/// slice `until` whose beam lacks it, or `None` if no hypothesis ever gave it the label.
/// Requires an analysis run with `Config::record_pruning`.
pub fn find_pruning(
    analysis: &Analysis,
    slices: &[Slice],
    (at, until): (usize, usize),
    label: &Label,
) -> Option<Absence> {
    let holds = |pruned: &Pruned| {
        pruned_segment_at(analysis, pruned, at).is_some_and(|s| is_rn_match(&segment_label(&s), label))
    };
    let last = (at..=until).rev().find_map(|slice| {
        let dropped = analysis.pruned.get(slice)?;
        dropped.iter().find(|p| holds(p)).map(|p| (slice, p.cause))
    });
    let (slice, cause) = last?;
    let after = slices[slice].start - slices[at].start;
    Some(match cause {
        PruneCause::Width => Absence::Width { after },
        PruneCause::Merged { .. } => Absence::Merged { after },
    })
}

fn pruned_segment_at(analysis: &Analysis, pruned: &Pruned, at: usize) -> Option<Segment> {
    let segment = pruned.segment;
    if segment.start as usize <= at { Some(segment) } else { segment_at(analysis, segment.previous, at) }
}
