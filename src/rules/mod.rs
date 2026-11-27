//! The rule catalog. Each file holds the rules of one layer and topic, prefixed by layer.

pub mod chord_bass;
pub mod chord_completeness;
pub mod chord_fit;
pub mod chord_proposal;
pub mod chord_rhythm;
pub mod chord_seventh_sonority;
pub mod function_progression;
pub mod function_seventh_style;
pub mod function_six_four;
pub mod function_vocabulary;
pub mod key_cadence;
pub mod key_context;
pub mod key_modulation;
pub mod key_phrase_close;
pub mod key_profile;
pub mod sources;

use crate::engine::rule::Rule;

/// Returns every rule in the catalog with its default weight.
pub fn default_rules() -> Vec<Rule> {
    [
        chord_proposal::RULES,
        chord_fit::RULES,
        chord_rhythm::RULES,
        chord_completeness::RULES,
        chord_bass::RULES,
        chord_seventh_sonority::RULES,
        key_profile::RULES,
        key_context::RULES,
        key_modulation::RULES,
        key_cadence::RULES,
        key_phrase_close::RULES,
        function_vocabulary::RULES,
        function_progression::RULES,
        function_seventh_style::RULES,
        function_six_four::RULES,
    ]
    .concat()
}
