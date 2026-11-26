//! The rule catalog. Each file holds the rules of one layer and topic, prefixed by layer.

pub mod chord_completeness;
pub mod chord_fit;
pub mod chord_proposal;
pub mod chord_rhythm;
pub mod function_progression;
pub mod function_vocabulary;
pub mod key_modulation;
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
        key_profile::RULES,
        key_modulation::RULES,
        function_vocabulary::RULES,
        function_progression::RULES,
    ]
    .concat()
}
