//! The analysis document: the favored reading of each chord span, its competitors, the rules
//! behind each reading and how its ranking changed over time.

use super::rank_history::{
    HistoryContext, PrunedAlternativeOutput, RankChangeOutput, pruned_alternatives, rank_history_output,
};
use crate::engine::history::{ReadingHistory, rank_histories};
use crate::engine::model::{Category, Reading};
use crate::engine::path::{SegmentReading, segments_of};
use crate::engine::rule::{Citation, Layer, Rule};
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
    pub sources: &'static [Source],
    pub rules: Vec<RuleInfo>,
    pub segments: Vec<SegmentOutput>,
}

#[derive(Serialize)]
pub struct RuleInfo {
    pub id: &'static str,
    pub layer: Layer,
    pub hook: &'static str,
    pub summary: &'static str,
    pub citations: &'static [Citation],
    pub weight: f32,
}

/// Whether a span's reading is settled by the rules or open to interpretation.
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
    pub alternatives: Vec<AlternativeOutput>,
    pub trace: Vec<Contribution>,
    pub rank_history: Vec<RankChangeOutput>,
    /// Readings that the beam held for the span's first slice but later dropped, best first.
    /// Each rank history ends with when and why the reading left the beam.
    pub pruned_alternatives: Vec<PrunedAlternativeOutput>,
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

#[derive(Serialize)]
pub struct AlternativeOutput {
    pub reading: ReadingOutput,
    /// Rank of the best final hypothesis holding this reading.
    pub final_rank: usize,
    /// How far that hypothesis's score falls below the best hypothesis.
    pub score_gap: f32,
    pub trace: Vec<Contribution>,
    pub rank_history: Vec<RankChangeOutput>,
}

pub struct OutputOptions {
    pub alternatives: usize,
    /// Alternatives within this score gap make a span interpretive.
    pub margin: f32,
}

/// Everything needed to describe one segment of the favored hypothesis.
struct Context<'a> {
    score: &'a Score,
    slices: &'a [Slice],
    analysis: &'a Analysis,
    rules: &'a RuleSet,
    ranked: Vec<(Vec<SegmentReading>, f32)>,
    margin: f32,
    alternatives: usize,
}

pub fn build_output(
    score: &Score,
    slices: &[Slice],
    analysis: &Analysis,
    rules: &RuleSet,
    options: &OutputOptions,
) -> AnalysisOutput {
    let leading = analysis.ranking.iter().take(options.alternatives + 1);
    let ranked = leading.map(|&(node, s)| (segments_of(analysis, slices, node), s)).collect();
    let context = Context {
        score,
        slices,
        analysis,
        rules,
        ranked,
        margin: options.margin,
        alternatives: options.alternatives,
    };
    let best = context.ranked.first().map(|(path, _)| path.clone()).unwrap_or_default();
    let segments = best.iter().map(|s| segment_output(s, &context)).collect();
    AnalysisOutput { sources: SOURCES, rules: rule_infos(rules), segments }
}

fn segment_output(segment: &SegmentReading, context: &Context) -> SegmentOutput {
    let reading = reading_output(&segment.label, segment.reading.category);
    let at = segment.slices.0;
    let histories = rank_histories(context.analysis, at);
    let history_context =
        HistoryContext { at, score: context.score, slices: context.slices, analysis: context.analysis };
    let history_of = |reading: &Reading| history_for(&histories, reading, &history_context);
    let alternatives = alternatives_at(segment, context, &reading, history_of);
    let is_open = alternatives.iter().any(|a| a.score_gap <= context.margin);
    let measures = &context.score.measures;
    SegmentOutput {
        start: segment.start,
        end: segment.end,
        measure: measures[measure_at(measures, segment.start)].number.clone(),
        reading,
        status: if is_open { Status::Interpretive } else { Status::Determined },
        alternatives,
        trace: trace_segment(context.analysis, context.slices, context.rules, segment),
        rank_history: history_of(&segment.reading),
        pruned_alternatives: pruned_alternatives(&histories, &history_context, context.alternatives),
    }
}

fn history_for(
    histories: &[ReadingHistory],
    reading: &Reading,
    context: &HistoryContext,
) -> Vec<RankChangeOutput> {
    let history = histories.iter().find(|h| h.reading == *reading);
    history.map(|h| rank_history_output(h, context)).unwrap_or_default()
}

/// Collects the distinct readings that lower-ranked hypotheses give to the segment's first slice.
fn alternatives_at(
    segment: &SegmentReading,
    context: &Context,
    favored: &ReadingOutput,
    history_of: impl Fn(&Reading) -> Vec<RankChangeOutput>,
) -> Vec<AlternativeOutput> {
    let best_score = context.ranked[0].1;
    let at = segment.slices.0;
    let mut alternatives: Vec<AlternativeOutput> = Vec::new();
    for (rank, (path, score)) in context.ranked.iter().enumerate().skip(1) {
        let Some(rival) = path.iter().find(|s| s.slices.0 <= at && at < s.slices.1) else {
            continue;
        };
        let reading = reading_output(&rival.label, rival.reading.category);
        if reading == *favored || alternatives.iter().any(|a| a.reading == reading) {
            continue;
        }
        let trace = trace_segment(context.analysis, context.slices, context.rules, rival);
        alternatives.push(AlternativeOutput {
            reading,
            final_rank: rank + 1,
            score_gap: best_score - score,
            trace,
            rank_history: history_of(&rival.reading),
        });
    }
    alternatives
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
        layer: rule.layer,
        hook: rule.hook.kind(),
        summary: rule.summary,
        citations: rule.citations,
        weight: *weight,
    };
    rules.rules.iter().zip(&rules.weights).map(info).collect()
}
