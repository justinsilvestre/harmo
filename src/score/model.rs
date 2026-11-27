//! The score representation consumed by the engine.

use crate::pitch::Tpc;
use serde::Serialize;

/// Time in quarter notes from the start of the score.
pub type Offset = f64;

#[derive(Clone, Debug, Serialize)]
pub struct Note {
    pub onset: Offset,
    pub duration: Offset,
    pub tpc: Tpc,
    pub midi: i32,
    pub part: u16,
    /// True when the note continues a tied note, so it is not a new attack.
    pub tied_from_previous: bool,
    pub has_fermata: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Measure {
    /// The measure number as written in the score, e.g. "0", "12" or "12a".
    pub number: String,
    pub start: Offset,
    pub duration: Offset,
    pub beats: u32,
    pub beat_type: u32,
}

impl Measure {
    /// The duration implied by the time signature, which pickup measures fall short of.
    pub fn nominal_duration(&self) -> Offset {
        self.beats as f64 * 4.0 / self.beat_type as f64
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Score {
    pub notes: Vec<Note>,
    pub measures: Vec<Measure>,
    /// The times at which a note under a fermata ends, in order and without repeats.
    pub fermata_ends: Vec<Offset>,
}

impl Score {
    pub fn end(&self) -> Offset {
        self.measures.last().map(|m| m.start + m.duration).unwrap_or(0.0)
    }
}
