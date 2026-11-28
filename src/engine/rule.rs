//! The interface between the engine and the rules it hooks into.

use super::chord_hooks::ChordHook;
use super::key_hooks::KeyHook;
use serde::Serialize;

/// What a rule judges: chords, keys, or the functions of chords in keys.
/// A rule's id and file are prefixed by its topic.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Topic {
    Chord,
    Key,
    Function,
}

impl Topic {
    /// Returns the layer of the search whose rules judge the topic.
    /// Function rules belong to the key layer, which hears keys and functions together.
    pub fn layer(self) -> Layer {
        match self {
            Topic::Chord => Layer::Chord,
            Topic::Key | Topic::Function => Layer::Key,
        }
    }
}

/// A layer of the search.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Layer {
    Chord,
    Key,
}

/// A pointer into the source catalog, e.g. a page range in a textbook.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Citation {
    pub source: &'static str,
    pub locator: &'static str,
}

/// A rule: one hook into the engine, documented with its sources.
///
/// Scoring hooks return unweighted evidence; the engine multiplies it by the rule's weight.
/// Generator and constraint hooks are switched off by a weight of zero.
#[derive(Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub topic: Topic,
    pub summary: &'static str,
    pub citations: &'static [Citation],
    pub default_weight: f32,
    /// The constants inside the rule that can be overridden at runtime, in the order of their `index`.
    pub params: &'static [Param],
    pub hook: Hook,
}

impl Rule {
    /// Returns the default value of each of the rule's parameters, in order.
    pub fn default_params(&self) -> Vec<f64> {
        self.params.iter().map(|p| p.default).collect()
    }
}

/// A constant inside a rule, such as a threshold, a duration or a ratio,
/// that can be overridden at runtime as `<rule id>.<name>` beside the rule weights.
///
/// A scoring hook receives the values of its rule's parameters in order and reads each one with `read`.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Param {
    /// The parameter's position in its rule's `params`.
    pub index: usize,
    pub name: &'static str,
    pub default: f64,
    pub unit: &'static str,
    /// Where the default comes from: "cited: <source, page>", or "measured:
    /// <sweep or commit>" with the values tried and what they gave.
    pub provenance: &'static str,
}

impl Param {
    #[inline]
    pub fn read(&self, values: &[f64]) -> f64 {
        values[self.index]
    }
}

/// The point at which a rule hooks into the search, by the layer whose state it receives.
#[derive(Clone, Copy)]
pub enum Hook {
    Chord(ChordHook),
    Key(KeyHook),
}

impl Hook {
    /// Returns the layer of the search whose state the hook receives.
    pub fn layer(&self) -> Layer {
        match self {
            Hook::Chord(_) => Layer::Chord,
            Hook::Key(_) => Layer::Key,
        }
    }

    /// Returns the name of the hook's kind, prefixed by its layer, such as `chord_slice`.
    pub fn kind(&self) -> &'static str {
        match self {
            Hook::Chord(hook) => hook.kind(),
            Hook::Key(hook) => hook.kind(),
        }
    }
}
