//! Proposes candidate chords from the notes of the slice where a segment would open.

use super::sources::KOSTKA_PAYNE_ALMEN_2023;
use crate::engine::chord_hooks::ChordHook;
use crate::engine::rule::{Citation, Hook, Rule, Topic};
use crate::pitch::Tpc;
use crate::score::slicing::Slice;
use crate::theory::chord::{Chord, QUALITIES};

pub const RULES: &[Rule] = &[Rule {
    id: "chord.propose_templates",
    topic: Topic::Chord,
    summary: "Propose every triad and seventh-chord type that contains at least two of the slice's \
              pitches as spelled (or its only pitch, when there is just one).",
    citations: &[Citation {
        source: KOSTKA_PAYNE_ALMEN_2023,
        locator: "ch. 3, pp. 38–44 (triad and seventh chord types); ch. 22, pp. 382–389 (augmented sixths)",
    }],
    default_weight: 1.0,
    params: &[],
    hook: Hook::Chord(ChordHook::Propose(propose_templates)),
}];

fn propose_templates(slice: &Slice) -> Vec<Chord> {
    let mut pitches: Vec<Tpc> = slice.notes.iter().map(|n| n.tpc).collect();
    pitches.sort();
    pitches.dedup();
    let required = pitches.len().min(2);
    let mut chords: Vec<Chord> = Vec::new();
    for &pitch in &pitches {
        for &quality in &QUALITIES {
            for interval in quality.template() {
                let chord = Chord::new(pitch - interval, quality);
                let matched = pitches.iter().filter(|&&p| chord.member_index(p).is_some()).count();
                if matched >= required && !chords.contains(&chord) {
                    chords.push(chord);
                }
            }
        }
    }
    chords
}
