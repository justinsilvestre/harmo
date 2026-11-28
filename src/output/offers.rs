//! The readings offered for a span's first slice, as the bench computes them:
//! the joint list of whole readings, and the keys and chords offered one layer at a time.

use super::analysis::{ReadingOutput, history_for, reading_output};
use super::rank_history::{HistoryContext, RankChangeOutput};
use crate::bench::layer_frames::{layer_candidates, layer_offers_at};
use crate::bench::layer_offers::ChordName;
use crate::bench::runner::{EvalConfig, lagged_beam, lagged_slice, offered_holders, reading_at};
use crate::engine::history::ReadingHistory;
use crate::engine::model::KeyHypothesisId;
use crate::engine::path::{SegmentReading, segment_covering};
use crate::engine::ruleset::RuleSet;
use crate::engine::trace::{Contribution, trace_segment};
use crate::pitch::tpc_name;
use crate::theory::key::Key;
use crate::theory::label::Label;
use serde::Serialize;

/// The settings that decide which readings are offered, as the bench takes them.
#[derive(Serialize)]
pub struct OfferSettings {
    /// The most readings offered in the joint list, and in each layer.
    pub count: usize,
    /// How far below the best score a reading in the joint list may fall.
    pub margin: f32,
    /// How many quarter notes after a slice begins its beam is read.
    pub lag: f64,
    /// The largest shortfall of an offered key.
    pub key_margin: f32,
    /// The largest shortfall of an offered chord.
    pub chord_margin: f32,
}

impl OfferSettings {
    pub fn of(config: &EvalConfig) -> OfferSettings {
        OfferSettings {
            count: config.offer_count,
            margin: config.offer_margin,
            lag: config.offer_lag,
            key_margin: config.key_offer_margin,
            chord_margin: config.chord_offer_margin,
        }
    }
}

/// The readings offered for one slice.
#[derive(Serialize)]
pub struct OffersOutput {
    /// The slice whose beam offers them: the last one that begins within the lag of the slice.
    pub beam_slice: usize,
    /// Whole readings, best first.
    pub joint: Vec<JointOfferOutput>,
    /// Keys, smallest shortfall first.
    pub keys: Vec<KeyOfferOutput>,
    /// Chords (root, quality and inversion), smallest shortfall first.
    pub chords: Vec<ChordOfferOutput>,
}

#[derive(Serialize)]
pub struct JointOfferOutput {
    pub reading: ReadingOutput,
    /// How far the best hypothesis holding the reading falls below the best of the beam.
    pub score_gap: f32,
    /// The rules behind the segment of that hypothesis that covers the slice.
    pub trace: Vec<Contribution>,
    pub rank_history: Vec<RankChangeOutput>,
}

#[derive(Serialize)]
pub struct KeyOfferOutput {
    pub key: String,
    #[serde(flatten)]
    pub holder: HolderOutput,
}

#[derive(Serialize)]
pub struct ChordOfferOutput {
    pub chord: String,
    pub inversion: u8,
    #[serde(flatten)]
    pub holder: HolderOutput,
}

/// The best hypothesis of the beam holding an offered key or chord.
#[derive(Serialize)]
pub struct HolderOutput {
    /// The best score of the beam minus this hypothesis's score.
    pub shortfall: f32,
    /// This hypothesis's whole reading of the slice.
    pub held_by: ReadingOutput,
    /// The rules behind this hypothesis's segment that covers the slice.
    pub trace: Vec<Contribution>,
}

/// What finding and explaining the offered readings of a slice needs.
pub struct OfferContext<'a> {
    pub history: &'a HistoryContext<'a>,
    pub histories: &'a [ReadingHistory],
    pub rules: &'a RuleSet,
    pub config: &'a EvalConfig,
}

/// Returns the readings offered for the context's slice, jointly and by layer.
pub fn offers_output(context: &OfferContext) -> OffersOutput {
    let (slices, at) = (context.history.slices, context.history.at);
    OffersOutput {
        beam_slice: lagged_slice(slices, at, context.config.offer_lag),
        joint: joint_offers(context),
        keys: key_offers(context),
        chords: chord_offers(context),
    }
}

fn joint_offers(context: &OfferContext) -> Vec<JointOfferOutput> {
    let (analysis, slices, at) = (context.history.analysis, context.history.slices, context.history.at);
    let Some(beam) = lagged_beam(analysis, slices, at, context.config.offer_lag) else {
        return vec![];
    };
    let best = beam.first().map(|n| analysis.layers.score(n)).unwrap_or_default();
    let offer = |(hypothesis, _): (KeyHypothesisId, Label)| {
        let segment = covering(context, hypothesis);
        JointOfferOutput {
            reading: reading_output(&segment.label, segment.reading.function.category),
            score_gap: best - analysis.layers.score(hypothesis),
            trace: trace_segment(analysis, slices, context.rules, &segment),
            rank_history: history_for(context.histories, &segment.reading, context.history),
        }
    };
    offered_holders(analysis, beam, at, context.config).into_iter().map(offer).collect()
}

fn key_offers(context: &OfferContext) -> Vec<KeyOfferOutput> {
    let (analysis, slices, at) = (context.history.analysis, context.history.slices, context.history.at);
    let keys = layer_offers_at(analysis, slices, at, context.config).keys;
    let offer = |key: Key| KeyOfferOutput {
        key: key.name(),
        holder: holder_output(context, &|label| label.key == key),
    };
    keys.into_iter().map(offer).collect()
}

fn chord_offers(context: &OfferContext) -> Vec<ChordOfferOutput> {
    let (analysis, slices, at) = (context.history.analysis, context.history.slices, context.history.at);
    let chords = layer_offers_at(analysis, slices, at, context.config).chords;
    let offer = |chord: ChordName| ChordOfferOutput {
        chord: format!("{}{}", tpc_name(chord.root), chord.quality.label()),
        inversion: chord.inversion,
        holder: holder_output(context, &|label| ChordName::of(label) == chord),
    };
    chords.into_iter().map(offer).collect()
}

/// Describes the best hypothesis, among those offered by layer, whose reading `holds` accepts.
/// The bench offers a key or chord only when some such hypothesis holds it.
fn holder_output(context: &OfferContext, holds: &dyn Fn(&Label) -> bool) -> HolderOutput {
    let (analysis, slices, at) = (context.history.analysis, context.history.slices, context.history.at);
    let (best, candidates) =
        layer_candidates(analysis, slices, at, context.config).expect("an offered reading has a beam");
    let score = |hypothesis: KeyHypothesisId| analysis.layers.score(hypothesis);
    let is_holder = |&n: &KeyHypothesisId| reading_at(analysis, n, at).is_some_and(|l| holds(&l));
    let holder = candidates
        .into_iter()
        .filter(is_holder)
        .reduce(|kept, n| if score(n) > score(kept) { n } else { kept })
        .expect("an offered reading has a holder");
    let segment = covering(context, holder);
    HolderOutput {
        shortfall: best - score(holder),
        held_by: reading_output(&segment.label, segment.reading.function.category),
        trace: trace_segment(analysis, slices, context.rules, &segment),
    }
}

fn covering(context: &OfferContext, hypothesis: KeyHypothesisId) -> SegmentReading {
    let (analysis, slices, at) = (context.history.analysis, context.history.slices, context.history.at);
    segment_covering(analysis, slices, hypothesis, at).expect("an offering hypothesis covers the slice")
}
