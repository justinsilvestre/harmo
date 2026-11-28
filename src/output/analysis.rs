//! The analysis document: the favored reading of each chord span, the readings offered for it,
//! the rules behind each reading and how its ranking changed over time.

use super::offers::{OfferContext, OfferSettings, OffersOutput, offers_output};
use super::rank_history::{
    HistoryContext, PrunedAlternativeOutput, RankChangeOutput, RankHistoriesByLayerOutput,
    pruned_alternatives, rank_histories_by_layer, rank_history_output,
};
use crate::bench::runner::EvalConfig;
use crate::engine::history::{ReadingHistory, rank_histories};
use crate::engine::model::{Category, Reading};
use crate::engine::path::{SegmentReading, best_segments};
use crate::engine::rule::{Citation, Rule, Topic};
use crate::engine::ruleset::RuleSet;
use crate::engine::search::Analysis;
use crate::engine::trace::{Contribution, trace_segment};
use crate::pitch::tpc_name;
use crate::rules::sources::{SOURCES, Source};
use crate::score::model::Score;
use crate::score::slicing::{Slice, measure_at};
use crate::theory::label::Label;
use serde::Serialize;

#[derive(Serialize)]
pub struct AnalysisOutput {
    pub offer_settings: OfferSettings,
    pub sources: &'static [Source],
    pub rules: Vec<RuleInfo>,
    pub segments: Vec<SegmentOutput>,
}

#[derive(Serialize)]
pub struct RuleInfo {
    pub id: &'static str,
    pub topic: Topic,
    pub hook: &'static str,
    pub summary: &'static str,
    pub citations: &'static [Citation],
    pub weight: f32,
}

/// Whether a span's reading is settled by the rules or open to interpretation:
/// interpretive when the joint list of readings offered for its first slice holds another reading.
#[derive(Serialize, PartialEq)]
pub enum Status {
    Determined,
    Interpretive,
}

#[derive(Serialize)]
pub struct SegmentOutput {
    pub start: f64,
    pub end: f64,
    pub measure: String,
    pub reading: ReadingOutput,
    pub status: Status,
    /// The readings offered for the span's first slice, as the bench offers them.
    pub offered: OffersOutput,
    pub trace: Vec<Contribution>,
    pub rank_history: Vec<RankChangeOutput>,
    /// Readings that the beam held for the span's first slice but later dropped, best first.
    /// Each rank history ends with when and why the reading left the beam.
    pub pruned_alternatives: Vec<PrunedAlternativeOutput>,
    /// The rank histories of the span's reading in each layer's own rankings.
    pub rank_histories_by_layer: RankHistoriesByLayerOutput,
}

#[derive(Serialize, Clone, PartialEq)]
pub struct ReadingOutput {
    pub key: String,
    pub figure: String,
    pub chord: String,
    pub degree: String,
    pub secondary: Option<String>,
    pub quality: &'static str,
    pub inversion: u8,
    pub root: String,
    pub category: Category,
}

/// The most readings of a span that `pruned_alternatives` lists.
const PRUNED_ALTERNATIVES: usize = 8;

/// Everything needed to describe one segment of the favored hypothesis.
struct Context<'a> {
    score: &'a Score,
    slices: &'a [Slice],
    analysis: &'a Analysis,
    rules: &'a RuleSet,
    config: &'a EvalConfig,
}

/// Describes each span of the best final hypothesis,
/// offering readings under the offer settings of `config`.
pub fn build_output(
    score: &Score,
    slices: &[Slice],
    analysis: &Analysis,
    rules: &RuleSet,
    config: &EvalConfig,
) -> AnalysisOutput {
    let context = Context { score, slices, analysis, rules, config };
    let segments = best_segments(analysis, slices).iter().map(|s| segment_output(s, &context)).collect();
    AnalysisOutput {
        offer_settings: OfferSettings::of(config),
        sources: SOURCES,
        rules: rule_infos(rules),
        segments,
    }
}

fn segment_output(segment: &SegmentReading, context: &Context) -> SegmentOutput {
    let reading = reading_output(&segment.label, segment.reading.function.category);
    let at = segment.slices.0;
    let histories = rank_histories(context.analysis, at);
    let history_context =
        HistoryContext { at, score: context.score, slices: context.slices, analysis: context.analysis };
    let offer_context = OfferContext {
        history: &history_context,
        histories: &histories,
        rules: context.rules,
        config: context.config,
    };
    let offered = offers_output(&offer_context);
    let is_open = offered.joint.iter().any(|offer| offer.reading != reading);
    let measures = &context.score.measures;
    SegmentOutput {
        start: segment.start,
        end: segment.end,
        measure: measures[measure_at(measures, segment.start)].number.clone(),
        reading,
        status: if is_open { Status::Interpretive } else { Status::Determined },
        offered,
        trace: trace_segment(context.analysis, context.slices, context.rules, segment),
        rank_history: history_for(&histories, &segment.reading, &history_context),
        pruned_alternatives: pruned_alternatives(&histories, &history_context, PRUNED_ALTERNATIVES),
        rank_histories_by_layer: rank_histories_by_layer(segment, &history_context),
    }
}

pub(super) fn history_for(
    histories: &[ReadingHistory],
    reading: &Reading,
    context: &HistoryContext,
) -> Vec<RankChangeOutput> {
    let history = histories.iter().find(|h| h.reading == *reading);
    history.map(|h| rank_history_output(h, context)).unwrap_or_default()
}

pub fn reading_output(label: &Label, category: Category) -> ReadingOutput {
    ReadingOutput {
        key: label.key.name(),
        figure: label.figure(),
        chord: format!("{}{}", tpc_name(label.root), label.quality.label()),
        degree: label.degree.label(),
        secondary: label.secondary.map(|d| d.label()),
        quality: label.quality.label(),
        inversion: label.inversion,
        root: tpc_name(label.root),
        category,
    }
}

fn rule_infos(rules: &RuleSet) -> Vec<RuleInfo> {
    let info = |(rule, weight): (&Rule, &f32)| RuleInfo {
        id: rule.id,
        topic: rule.topic,
        hook: rule.hook.kind(),
        summary: rule.summary,
        citations: rule.citations,
        weight: *weight,
    };
    rules.rules.iter().zip(&rules.weights).map(info).collect()
}
