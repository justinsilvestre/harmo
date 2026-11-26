//! A small command-line parser: positional arguments plus repeatable `--name value` options.

use std::collections::HashMap;

/// Options that take no value.
const SWITCHES: &[&str] = &["json"];

pub struct Args {
    positional: Vec<String>,
    options: HashMap<String, Vec<String>>,
}

impl Args {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Args {
        let mut positional = Vec::new();
        let mut options: HashMap<String, Vec<String>> = HashMap::new();
        let mut arguments = arguments.peekable();
        while let Some(argument) = arguments.next() {
            let Some(name) = argument.strip_prefix("--") else {
                positional.push(argument);
                continue;
            };
            let value = if SWITCHES.contains(&name) { Some("true".to_string()) } else { arguments.next() };
            options.entry(name.to_string()).or_default().extend(value);
        }
        Args { positional, options }
    }

    pub fn command(&self) -> Option<&str> {
        self.positional.first().map(String::as_str)
    }

    /// Returns the positional argument after the command.
    pub fn positional(&self, index: usize) -> Option<&str> {
        self.positional.get(index + 1).map(String::as_str)
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.options.get(name).and_then(|v| v.last()).map(String::as_str)
    }

    pub fn values(&self, name: &str) -> Vec<String> {
        self.options.get(name).cloned().unwrap_or_default()
    }

    pub fn has(&self, name: &str) -> bool {
        self.options.contains_key(name)
    }

    pub fn number(&self, name: &str) -> Result<Option<f64>, String> {
        let parse = |v: &str| v.parse::<f64>().map_err(|_| format!("--{name} expects a number, got {v}"));
        self.value(name).map(parse).transpose()
    }
}
