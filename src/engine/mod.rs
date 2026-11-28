//! The engine core and its interface to rules.

pub mod bass;
pub mod chord_hooks;
pub mod chord_rules;
pub mod history;
pub mod key_hooks;
pub mod key_rules;
pub mod layers;
pub mod model;
pub mod path;
pub mod pruning;
pub mod rule;
pub mod ruleset;
pub mod search;
#[cfg(test)]
pub mod test_layers;
pub mod trace;
