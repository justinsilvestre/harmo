//! Reads the rule weights and parameter values that the command line overrides.

use crate::cli_args::Args;
use crate::engine::ruleset::Weights;

/// Reads `--weights FILE` and `--params FILE`, each a JSON object of rule ids (for weights)
/// and `<rule id>.<parameter name>` keys (for parameters), into one set of overrides.
/// Either file may hold either kind of key; where both files set a key, `--params` wins.
pub fn load_overrides(args: &Args) -> Result<Weights, String> {
    let mut overrides = load_file(args.value("weights"))?;
    overrides.extend(load_file(args.value("params"))?);
    Ok(overrides)
}

fn load_file(path: Option<&str>) -> Result<Weights, String> {
    let Some(path) = path else {
        return Ok(Weights::new());
    };
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}
