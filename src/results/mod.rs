//! Writes RESULTS.md: every figure that `bench` and `diagnose` compute for the standard sets,
//! with notes on how to read them.

pub mod markdown;
mod markdown_accuracy;
mod markdown_agreement;
mod markdown_disagreements;
mod markdown_equivalents;
mod markdown_errors;
mod markdown_errors_examples;
mod markdown_header;
mod markdown_kinds;
mod markdown_offer_delays;
mod markdown_set_figures;
mod markdown_table;
#[cfg(test)]
mod markdown_tests;
pub mod record;
mod set_names;
