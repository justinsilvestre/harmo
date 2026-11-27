//! The rule catalog. Each file holds the rules of one layer and topic, prefixed by layer.

pub mod chord_bass;
pub mod chord_bass_note;
pub mod chord_completeness;
pub mod chord_fit;
pub mod chord_implied_harmony;
pub mod chord_proposal;
pub mod chord_rhythm;
pub mod chord_seventh_delay;
pub mod chord_seventh_sonority;
pub mod function_distinct_readings;
pub mod function_picardy_third;
pub mod function_progression;
pub mod function_rare_mediant;
pub mod function_seventh_style;
pub mod function_six_four;
pub mod function_vocabulary;
pub mod key_absent_tonic;
pub mod key_cadence;
pub mod key_confirmation;
pub mod key_context;
pub mod key_modal_minor;
pub mod key_modulation;
pub mod key_opening_tonic;
pub mod key_phrase_close;
pub mod key_profile;
pub mod key_proposal;
pub mod key_raised_fourth;
pub mod key_segment_profile;
pub mod key_signature;
pub mod key_six_four_tonicization;
pub mod sources;

use crate::engine::rule::Rule;

/// Returns every rule in the catalog with its default weight.
pub fn default_rules() -> Vec<Rule> {
    [
        chord_proposal::RULES,
        chord_fit::RULES,
        chord_implied_harmony::RULES,
        chord_rhythm::RULES,
        chord_completeness::RULES,
        chord_bass::RULES,
        chord_bass_note::RULES,
        chord_seventh_sonority::RULES,
        key_proposal::RULES,
        chord_seventh_delay::RULES,
        key_profile::RULES,
        key_context::RULES,
        key_segment_profile::RULES,
        key_signature::RULES,
        key_modulation::RULES,
        key_cadence::RULES,
        key_six_four_tonicization::RULES,
        key_raised_fourth::RULES,
        key_confirmation::RULES,
        key_absent_tonic::RULES,
        key_phrase_close::RULES,
        key_opening_tonic::RULES,
        function_vocabulary::RULES,
        function_progression::RULES,
        function_picardy_third::RULES,
        function_seventh_style::RULES,
        function_rare_mediant::RULES,
        function_six_four::RULES,
        function_distinct_readings::RULES,
    ]
    .concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_parameter_is_read_from_its_own_position() {
        let is_in_place = |rule: &Rule| rule.params.iter().enumerate().all(|(i, p)| p.index == i);
        assert!(default_rules().iter().all(is_in_place));
    }

    #[test]
    fn every_parameter_states_where_its_value_comes_from() {
        let has_provenance = |rule: &Rule| rule.params.iter().all(|p| !p.provenance.is_empty());
        assert!(default_rules().iter().all(has_provenance));
    }
}
