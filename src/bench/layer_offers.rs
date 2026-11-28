//! Readings offered one layer at a time: keys and chords ranked separately by their shortfall, and
//! Roman numerals built from the offered keys and chords.
//!
//! A reading's shortfall is the best value of any hypothesis minus the best value of a hypothesis
//! holding that reading. Ranking keys and chords apart lets a key be offered for the music however
//! many chords compete within it, and the reverse.

use crate::pitch::Tpc;
use crate::theory::chord::{Chord, Quality};
use crate::theory::key::Key;
use crate::theory::label::Label;

/// A chord whatever its key, as the "chord" column compares it: root, quality and inversion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChordName {
    pub root: Tpc,
    pub quality: Quality,
    pub inversion: u8,
}

impl ChordName {
    pub fn of(label: &Label) -> ChordName {
        ChordName { root: label.root, quality: label.quality, inversion: label.inversion }
    }
}

/// How many readings each layer may offer, and within what shortfall.
#[derive(Clone, Copy, Debug)]
pub struct LayerLimits {
    pub count: usize,
    pub key_margin: f32,
    pub chord_margin: f32,
    /// The most that a numeral's key and chord shortfalls may add up to.
    pub numeral_margin: f32,
}

/// The keys, chords and numerals offered for one slice, each best first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerOffers {
    pub keys: Vec<Key>,
    pub chords: Vec<ChordName>,
    pub numerals: Vec<Label>,
}

/// Offers keys, chords and numerals from `held`, the value of each hypothesis with its label of
/// the slice, measured against the best value `best`.
/// A numeral pairs an offered key with an offered chord, since a numeral follows from a chord and
/// the key it is read in (Kostka, Payne and Almén 2023, pp. 54, 65).
/// Numerals are ranked by the sum of their key's and chord's shortfalls.
/// A numeral takes the tonicization of the best hypothesis holding its key and chord, if any.
pub fn layer_offers(held: &[(f32, Label)], best: f32, limits: &LayerLimits) -> LayerOffers {
    let keys = by_shortfall(held.iter().map(|(v, l)| (*v, l.key)), best, limits.key_margin, limits.count);
    let chords = by_shortfall(
        held.iter().map(|(v, l)| (*v, ChordName::of(l))),
        best,
        limits.chord_margin,
        limits.count,
    );
    let mut pairs: Vec<(f32, Key, ChordName)> = Vec::new();
    for &(key, key_shortfall) in &keys {
        for &(chord, chord_shortfall) in &chords {
            pairs.push((key_shortfall + chord_shortfall, key, chord));
        }
    }
    pairs.retain(|(shortfall, _, _)| *shortfall <= limits.numeral_margin);
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let numerals =
        pairs.iter().take(limits.count).map(|&(_, key, chord)| numeral(held, key, chord)).collect();
    LayerOffers {
        keys: keys.into_iter().map(|(key, _)| key).collect(),
        chords: chords.into_iter().map(|(chord, _)| chord).collect(),
        numerals,
    }
}

/// Returns the distinct values of `held` whose shortfall from `best` is within `margin`, with that
/// shortfall, smallest first and at most `count` of them.
/// A value held at negative infinity has no shortfall and is never offered.
/// Equal shortfalls keep the order of first appearance.
pub fn by_shortfall<T: PartialEq>(
    held: impl Iterator<Item = (f32, T)>,
    best: f32,
    margin: f32,
    count: usize,
) -> Vec<(T, f32)> {
    let mut distinct: Vec<(T, f32)> = Vec::new();
    for (value, item) in held {
        match distinct.iter_mut().find(|(seen, _)| *seen == item) {
            Some((_, top)) => *top = top.max(value),
            None => distinct.push((item, value)),
        }
    }
    let mut ranked: Vec<(T, f32)> = distinct
        .into_iter()
        .filter(|(_, value)| value.is_finite())
        .map(|(item, value)| (item, best - value))
        .filter(|(_, shortfall)| *shortfall <= margin)
        .collect();
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1));
    ranked.truncate(count);
    ranked
}

fn numeral(held: &[(f32, Label)], key: Key, chord: ChordName) -> Label {
    let holders = held.iter().filter(|(_, l)| l.key == key && ChordName::of(l) == chord);
    let best_holder = holders.max_by(|a, b| a.0.total_cmp(&b.0));
    let tonicized = best_holder.and_then(|(_, l)| l.tonicized);
    let named = Chord::new(chord.root, chord.quality);
    let bass = named.members().nth(chord.inversion as usize).unwrap_or(chord.root);
    Label::new(key, tonicized, named, bass)
}

#[cfg(test)]
#[path = "layer_offers_tests.rs"]
mod tests;
