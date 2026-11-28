//! Proposes the keys in which a segment opening on a slice may be read.

use super::sources::TEMPERLEY_2001;
use crate::engine::key_hooks::KeyHook;
use crate::engine::rule::{Citation, Hook, Layer, Rule};
use crate::score::slicing::Slice;
use crate::theory::key::{Key, all_keys};

pub const RULES: &[Rule] = &[Rule {
    id: "key.propose_all",
    layer: Layer::Key,
    summary: "Consider every key on every slice, from B𝄫 to B♯ in both modes, so that the keys \
              available never depend on music not yet heard. Enharmonically equivalent keys are \
              distinct keys, told apart by spelling. Against admitting only the keys a piece spells, \
              this cost 0.02 RN on tune (commit f8acf4e).",
    citations: &[
        Citation {
            source: TEMPERLEY_2001,
            locator: "p. 176 (the key-profile score is computed for all 24 keys)",
        },
        Citation {
            source: TEMPERLEY_2001,
            locator: "p. 187 (with spelled pitches, enharmonically related keys have different key profiles)",
        },
    ],
    default_weight: 1.0,
    params: &[],
    hook: Hook::Key(KeyHook::Propose(propose_all)),
}];

fn propose_all(_: &Slice) -> Vec<Key> {
    all_keys().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_slice() -> Slice {
        let (start, duration, notes, measure, metric_level, phrase) = (0.0, 1.0, vec![], 0, 0, 0);
        Slice { start, duration, notes, measure, metric_level, phrase, key_signature: None }
    }

    #[test]
    fn every_key_is_proposed() {
        assert_eq!(propose_all(&empty_slice()).len(), 44);
    }
}
