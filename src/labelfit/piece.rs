//! Measures how well each ground-truth label's chord tones fit the notes sounding while it is in force.

use super::sounding::{Fit, PitchClassSet, Sounding, pitch_class_set};
use crate::bench::frames::FRAME;
use crate::bench::groundtruth::PlacedHarmony;
use crate::score::model::{Measure, Offset, Score};
use crate::score::slicing::measure_at;
use crate::theory::label::Label;

/// A label and the time it is in force, until the next label starts.
#[derive(Clone, Debug)]
pub struct LabelSpan {
    pub start: Offset,
    pub end: Offset,
    pub label: Label,
    pub measure: i64,
    pub measure_suffix: String,
    pub pitches: Vec<crate::pitch::Tpc>,
    pub chord: PitchClassSet,
    pub fit: Fit,
}

impl LabelSpan {
    pub fn frames(&self) -> f64 {
        (self.end - self.start) / FRAME
    }

    /// The measure as the analysis numbers it, such as "12" or "12b".
    pub fn measure_name(&self) -> String {
        format!("{}{}", self.measure, self.measure_suffix)
    }
}

/// One analysis of a piece, with its labels placed and measured against the score.
pub struct PieceFit {
    pub id: String,
    pub analyst: String,
    pub sets: Vec<String>,
    pub measures: Vec<Measure>,
    pub sounding: Sounding,
    /// The labels that are neither excluded nor undecodable, in time order.
    pub spans: Vec<LabelSpan>,
    /// How many labels the analysis places past the end of their bar in the score.
    pub labels_beyond_bar: usize,
}

impl PieceFit {
    pub fn new(
        identity: (String, String, Vec<String>),
        score: &Score,
        harmonies: &[PlacedHarmony],
    ) -> PieceFit {
        let sounding = Sounding::new(&crate::score::slicing::slice_score(score));
        let ends = harmonies.iter().skip(1).map(|h| h.time).chain([score.end()]);
        let to_span = |(harmony, end): (&PlacedHarmony, Offset)| {
            let label = harmony.label.filter(|_| !harmony.is_excluded)?;
            let chord = pitch_class_set(harmony.pitches.iter().copied());
            Some(LabelSpan {
                start: harmony.time,
                end,
                label,
                measure: harmony.measure,
                measure_suffix: harmony.measure_suffix.clone(),
                pitches: harmony.pitches.clone(),
                chord,
                fit: sounding.fit(harmony.time, end, chord),
            })
        };
        let spans = harmonies.iter().zip(ends).filter_map(to_span).filter(|s| s.end > s.start).collect();
        let (id, analyst, sets) = identity;
        let labels_beyond_bar = harmonies.iter().filter(|h| h.is_beyond_bar).count();
        PieceFit { id, analyst, sets, measures: score.measures.clone(), sounding, spans, labels_beyond_bar }
    }

    pub fn total_fit(&self) -> Fit {
        self.spans.iter().fold(Fit::default(), |sum, span| sum + span.fit)
    }

    pub fn frames(&self) -> f64 {
        self.spans.iter().map(LabelSpan::frames).sum()
    }

    /// Returns the score measure in force at a time.
    pub fn measure_at(&self, time: Offset) -> &Measure {
        &self.measures[measure_at(&self.measures, time)]
    }
}

/// The fit of a group of consecutive labels: a key span or an analysis measure.
#[derive(Clone, Debug, serde::Serialize)]
pub struct GroupFit {
    pub name: String,
    pub first_measure: String,
    pub last_measure: String,
    pub frames: f64,
    pub fit: Option<f64>,
}

/// Groups consecutive labels that share a key, or a measure, and measures each group.
pub fn group_fits(spans: &[LabelSpan], name_of: impl Fn(&LabelSpan) -> String) -> Vec<GroupFit> {
    let mut groups: Vec<(String, Vec<&LabelSpan>)> = Vec::new();
    for span in spans {
        let name = name_of(span);
        match groups.last_mut() {
            Some((last, members)) if *last == name => members.push(span),
            _ => groups.push((name, vec![span])),
        }
    }
    let measure = |(name, members): (String, Vec<&LabelSpan>)| GroupFit {
        name,
        first_measure: members[0].measure_name(),
        last_measure: members[members.len() - 1].measure_name(),
        frames: members.iter().map(|s| s.frames()).sum(),
        fit: members.iter().fold(Fit::default(), |sum, s| sum + s.fit).share(),
    };
    groups.into_iter().map(measure).collect()
}
