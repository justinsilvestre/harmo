# Layered search: decisions

This record settles the open questions of [the layered listener note](layered-listener.md) for a first implementation. It states what was decided, on what evidence, and what is left open.

## What the beam holds now

A probe of the round 8 search on `tune` (375,615 slices) counted, in the beam of 80 hypotheses after each slice:
- **9.2 distinct keys** for the slice;
- **20.6 distinct chords** (chord and bass);
- **46.3 distinct chord histories** (the last four chord segments, with their starts and basses);
- **47.6 distinct key-and-chord pairs.**

So most hypotheses differ from one another in their chords, and each chord history is read in fewer than two keys on average. Keys get few places because chord variants take them.

The same probe looked at new segments that open on a slice and stay in the beam. Each chord opened there has on average **1.11 distinct chord histories** behind it across the keys it is read in, and 10% have more than one. The best history for a chord is therefore nearly always the same whichever key the chord is heard in.

## The model

**Two layers, two arenas.**
- **The chord layer** holds hearings of the chords: where segments begin, each segment's chord and its bass. A chord node records the chord layer's state at one slice and its score from the chord rules alone.
- **The key layer** holds interpretations of those hearings: each key node reads one chord node's latest segment in a key, with its function there (the tonicized key, if any, and the category). Its score adds the key and function rules.
- **A complete interpretation** is a key node. Its score is the chord node's score plus the key layer's, the sum that Temperley uses for interacting preference rule systems (2001, pp. 157–158).

Several key nodes can interpret the same chord node, so a chord hearing is scored once and shared. This is also the order of Temperley's own Roman numeral program, which runs the harmonic analysis first and gives its output to the key analysis (2001, pp. 188–189).

## The open questions

**The chord layer's unit is the chord segment.** A segment spans whole slices and holds a chord and a bass. Segmentation stays in the chord layer, since every chord rule that judges a boundary reads only the notes and the chords. Non-chord tones stay a matter for the chord rules that score a slice against a chord.

**A tonicized key belongs to the key layer, with function.** The vocabulary gives the tonicization and category together from a key and a chord, and the departure rules already treat a tonicization as staying in the key. Key and function are therefore one layer, as the design note's order (chord, then key and function) puts them. A segment still changes key only where a chord segment begins.

**Interaction rules are rules of the later layer.** A rule that judges a key against the chord heard at the same moment interprets the chord layer, so it belongs to the key layer. The rule survey found that no scoring chord rule reads the key or function, and the engine now enforces this: chord rules receive only the chord layer's state. Influence from the key layer back to the chords takes two forms, both traceable:
- **Revision.** A new segment opens after the hypothesis that leads into it best, complete interpretation included, so what the key layer hears now can choose between chord hearings held before. This is the step that Temperley's dynamic programming takes (2001, pp. 15–19, 206–207).
- **Ranking.** A chord hearing is kept while some interpretation of it is among the best. No rule of an earlier layer reads a later layer's belief at the same moment.

**Complete interpretations are ranked by the sum of the layers' scores.** Each layer's alternatives are ranked by the best complete interpretation that holds them, the same shortfall that the bench uses for coverage by layer.

**How alternatives are counted in coverage is left to the user.** The bench reports both whole readings and readings offered by layer. With layers, offering by layer follows the model: keys are offered from the key layer and chords from the chord layer, so a second reading of the same chord takes no place from a key. Whole readings stay as the yardstick that round 8 used.

## Stages

1. **Restructure only.** Split the arena into the two layers and give chord rules a context without a key. The search makes the same choices as before, so the figures should not change beyond rounding.
2. **Prune and cut by layer.** Keep a number of chord hearings, and for each a number of key interpretations, instead of one beam of whole readings. Cut the chords proposed for a slice by their chord fit and each chord's keys by their key fit, instead of cutting whole readings by their joint fit.
3. **Re-judge the rivals.** Measure the dominant seventh rival and rival basses as alternatives inside the chord layer.

## Still open

- Whether a key may change within a chord segment.
- A cadence and phrase layer. The interaction rules that judge cadences and progressions are the natural start, but they stay in the key layer for now.
- Rank histories for chord hearings on their own, beside the histories of whole readings.
