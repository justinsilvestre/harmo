//! Breaks the engine's errors on a benchmark set down by kind, to show where it loses the most.
//!
//! Every frame that the top reading gets wrong is assigned exactly one error kind. Frames where a
//! piece's analysts disagree are set apart and classified by what the analysts differ on.

pub mod beam_absence;
pub mod beam_diversity;
pub mod chord_relation;
pub mod disagreement_class;
pub mod error_kind;
pub mod frame_runs;
pub mod key_dependence;
pub mod key_relation;
pub mod missing_reading;
pub mod piece_context;
pub mod piece_disagreements;
pub mod piece_errors;
pub mod producibility;
pub mod report_text;
pub mod report_text_detail;
pub mod report_text_disagreements;
pub mod spans;
pub mod summary;
pub mod tally;
pub mod tally_class;
