//! Checks ground-truth labels against the notes, independently of the engine.
//!
//! For each label, the fit is the share of the notes' sounding time, summed over voices,
//! that belongs to the label's chord tones.
//! Runs of labels that fit markedly better when moved by a bar or a beat,
//! or when read in another key, point to misnumbered measures or a wrong key in the analysis.

pub mod candidates;
pub mod piece;
pub mod rekey;
pub mod report;
pub mod sounding;
