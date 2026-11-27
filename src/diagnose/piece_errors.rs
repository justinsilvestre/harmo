//! Assigns every wrong frame of a piece to exactly one error kind.

use super::chord_relation::root_position_figure;
use super::error_kind::{ErrorKind, classify_difference};
use super::frame_runs::{count_changes, is_shifted};
use super::piece_context::PieceContext;
use super::spans::{Span, group_spans};
use super::tally::{ErrorTally, Example, KindTally, keep_longest};
use crate::bench::frames::FRAME;
use crate::bench::metrics::is_rn_match;
use crate::theory::label::Label;

/// A chord change shifted by up to this many quarter notes counts as a timing error.
const TIMING_REACH: f64 = 1.0;

/// A wrong frame: its error kind, the expert's label and the engine's.
type WrongFrame = (ErrorKind, Label, Option<Label>);

pub fn tally_errors(context: &PieceContext) -> ErrorTally {
    let main = context.main_frames();
    let engine_frames = main.iter().zip(&context.top).map(|(m, t)| m.and(*t));
    let mut tally = ErrorTally {
        bars: context.piece.measures.len() as u64,
        expert_changes: count_changes(main.iter().copied()),
        engine_changes: count_changes(engine_frames),
        ..ErrorTally::default()
    };
    let mut wrong: Vec<(usize, WrongFrame)> = Vec::new();
    for (frame, expert) in main.iter().enumerate() {
        let Some(expert) = expert else {
            continue;
        };
        tally.frames += 1;
        let engine = context.top[frame];
        if engine.is_some_and(|e| is_rn_match(&e, expert)) {
            tally.correct += 1;
        } else {
            wrong.push((frame, (classify_frame(context, frame, expert, engine.as_ref()), *expert, engine)));
        }
    }
    for span in group_spans(&wrong) {
        add_span(&mut tally, context, &span);
    }
    tally
}

fn classify_frame(context: &PieceContext, frame: usize, expert: &Label, engine: Option<&Label>) -> ErrorKind {
    let reach = (TIMING_REACH / FRAME).round() as usize;
    if context.is_disagreement(frame) {
        return ErrorKind::Disagreement;
    }
    let Some(engine) = engine else {
        return ErrorKind::NoReading;
    };
    if is_shifted(context.main_frames(), &context.top, frame, reach) {
        ErrorKind::Timing
    } else if let Some(gap) = context.gaps[0][frame] {
        ErrorKind::Unproducible(gap)
    } else {
        classify_difference(expert, engine, context.key_runs[frame] >= context.sustained_frames)
    }
}

fn add_span(tally: &mut ErrorTally, context: &PieceContext, span: &Span<WrongFrame>) {
    let (kind, expert, engine) = span.value;
    let kind_tally = tally
        .kinds
        .entry(kind.name())
        .or_insert_with(|| KindTally { group: kind.group().to_string(), ..KindTally::default() });
    kind_tally.frames += span.frames as u64;
    if let Some(pair) = pair_name(kind, &expert, engine.as_ref()) {
        *kind_tally.pairs.entry(pair).or_default() += span.frames as u64;
    }
    let example = Example {
        piece: context.piece.id.clone(),
        measure: context.measure_of(span.start),
        frames: span.frames as u64,
        expert: describe(Some(&expert)),
        engine: describe(engine.as_ref()),
    };
    keep_longest(&mut kind_tally.examples, &[example]);
}

/// Names the pair of figures behind an error, such as "V vs viio", for kinds where pairs recur.
fn pair_name(kind: ErrorKind, expert: &Label, engine: Option<&Label>) -> Option<String> {
    let engine = engine?;
    match kind {
        ErrorKind::Root(_) | ErrorKind::Quality(_) | ErrorKind::Function => {
            Some(format!("{} vs {}", root_position_figure(expert), root_position_figure(engine)))
        }
        ErrorKind::Inversion => Some(format!("{} vs {}", expert.figure(), engine.figure())),
        ErrorKind::Unproducible(_) => Some(expert.figure()),
        ErrorKind::Key { .. } => Some(format!(
            "{}{} vs {}{}",
            mode_mark(expert),
            root_position_figure(expert),
            mode_mark(engine),
            root_position_figure(engine)
        )),
        _ => None,
    }
}

fn mode_mark(label: &Label) -> &'static str {
    match label.key.mode {
        crate::theory::key::Mode::Major => "M:",
        crate::theory::key::Mode::Minor => "m:",
    }
}

/// Formats a label with its key, such as "G: V65/V".
pub fn describe(label: Option<&Label>) -> String {
    label.map_or("-".into(), |l| format!("{}: {}", l.key.name(), l.figure()))
}
