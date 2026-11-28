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
- **The chord layer** holds hearings of the chords: where segments begin, each segment's chord and its bass. A hearing records the chord layer's state at one slice and its score from the chord rules alone.
- **The key layer** holds hypotheses that interpret those hearings: each hypothesis reads one hearing's latest segment in a key, with its function there (the tonicized key, if any, and the category). Its own score comes from the key and function rules.
- **A complete interpretation** is a hypothesis. Its score is its hearing's score plus the key layer's, the sum that Temperley uses for interacting preference rule systems (2001, pp. 157–158).

Several hypotheses can interpret the same hearing, so a hearing is scored once and shared. This is also the order of Temperley's own Roman numeral program, which runs the harmonic analysis first and gives its output to the key analysis (2001, pp. 188–189).

## The open questions

**The chord layer's unit is the chord segment.** A segment spans whole slices and holds a chord and a bass. Segmentation stays in the chord layer, since every chord rule that judges a boundary reads only the notes and the chords. Non-chord tones stay a matter for the chord rules that score a slice against a chord.

**A tonicized key belongs to the key layer, with function.** The vocabulary gives the tonicization and category together from a key and a chord, and the departure rules already treat a tonicization as staying in the key. Key and function are therefore one layer, as the design note's order (chord, then key and function) puts them. A segment still changes key only where a chord segment begins.

**Interaction rules are rules of the later layer.** A rule that judges a key against the chord heard at the same moment interprets the chord layer, so it belongs to the key layer. The rule survey found that no scoring chord rule reads the key or function, and the engine enforces this: hooks of the chord layer receive only the chord layer's state, and a test checks that every rule hooks into its own layer, except `chord.seventh_rival` (see below). Influence from the key layer back to the chords takes two forms, both traceable:
- **Revision.** A new segment opens after the hypothesis that leads into it best, complete interpretation included, so what the key layer hears now can choose between chord hearings held before. This is the step that Temperley's dynamic programming takes (2001, pp. 15–19, 206–207).
- **Ranking.** A chord hearing is kept while some interpretation of it is among the best. No rule of an earlier layer reads a later layer's belief at the same moment.

**Complete interpretations are ranked by the sum of the layers' scores.** Each layer's alternatives are ranked by the best complete interpretation that holds them, the same shortfall that the bench uses for coverage by layer.

**How alternatives are counted in coverage is left to the user.** The bench reports both whole readings and readings offered by layer. With layers, offering by layer follows the model: keys are offered from the key layer and chords from the chord layer, so a second reading of the same chord takes no place from a key. Whole readings stay as the yardstick that round 8 used.

## What was measured

Each stage was measured on `tune` and `agreement-tune` against the one before. Coverage is compared at an equal mean number offered.

**Splitting the layers changed nothing measurable** (commit dba10d3). Every figure stayed within 0.1 of round 8, since scores are now summed in another order, and the search took 10% less CPU time, since a hearing's chord scores are computed once for all the hypotheses that read it.

**A limit on hearings helps a little; per-layer cuts and quotas do not** (commit 46c5c6b).
- **At most 40 hearings in the beam of 80** left RN and RN eq unchanged and offered keys by layer about 1.4 points more often where the analysts disagree, with chords by layer about 0.5 higher. The search took 4% less time. It is now the default.
- **A fixed number of hypotheses per hearing** cost RN at equal CPU time. A beam of 32 hearings, each with up to 4 hypotheses, lost 0.3 RN, and only 64 by 4, at 1.7 times the CPU time, matched the flat beam.
- **Cutting the candidates by layer** (the chords that best fit the opening slice, then each chord's best keys) cost 0.7 to 1.5 RN. A chord and key that fit the opening slice together remain the better guide to which readings to consider, so whole readings are still cut by their joint fit. This is the cost of separating the layers that the design note foresaw.

**The dominant seventh rival passes when coverage is counted by layer.** With `chord.seventh_rival` on, a triad and its seventh chord are two hearings of the same span. A rival is the best hypothesis in the same group whose hearing hears another chord in that segment, so two keys over one hearing are not rivals. Where the analysts disagree on `agreement-tune`, RN offered by layer rose 2.8 points and chords 4.0, while keys by layer did not fall (-0.1, interval -0.7 to +0.4). Counted as whole readings, keys offered still fell 0.7 (-1.3 to -0.2), because the two readings take two of the four offered places. RN on `tune` did not change. The rule stays off until the yardstick is chosen.

**Rival basses still fail** (branch `bass-layered`, commit 7bd3fef, not merged). Each rival bass became a hearing of its own. Where the analysts disagree on the inversion, both readings were offered 20% of the time instead of 11%. But keys offered fell 6.0 points as whole readings and 2.6 by layer, and RN on `tune` fell 0.26. A rival ties with the hearing it came from, so it takes an offered place from a reading in another key, and it also takes one of the 40 hearing places in the beam.

## Still open

- Whether a key may change within a chord segment.
- A cadence and phrase layer. The interaction rules that judge cadences and progressions are the natural start, but they stay in the key layer for now.
- Rank histories for chord hearings on their own, beside the histories of whole readings.
- `chord.seventh_rival` decides whether a chord is a dominant from its key, so it is not yet a pure chord rule.
