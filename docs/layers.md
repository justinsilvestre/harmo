# What a layer is

The search now has two layers: chord hypotheses, and key hypotheses that read them (see `layered-listener.md` and `layered-search.md`). The two layers are built differently, and the second has no unit of its own. This note states what every layer should have in common, for the work that generalizes the engine to more layers.

## Where the two layers stand

- **Chord hypotheses** have segments of their own: where a chord begins, what it is and its bass.
- **Key hypotheses** have no unit of their own. A key hypothesis's segment is a chord segment with a key and a function added. A layer whose unit spans several chords, such as a cadence or a phrase, cannot be built this way.
- **The machinery differs between the two layers for no musical reason.** The two layers:
  - use different kinds of index;
  - keep different links;
  - offer hooks that exist in only one layer;
  - support rank histories in only one layer.
- **The engine names exactly two layers.** Each is special-cased rather than handled as one step in a sequence.

## A topic can be judged at more than one layer

What key the music is in is first judged from the notes and chords. It can be judged again once cadences are recognised. Whether a new key is a modulation or only a tonicization depends on whether the music goes on in it or returns to the key it left (Kostka, Payne and Almén 2023, p. 256). So the key cannot belong to one layer alone. Rules already state their topic (chord, key or function) apart from the layer they hook into. A layer should therefore be named for its unit, not for a topic.

## What every layer has

- **A unit and its hypotheses.** Each hypothesis reads one hypothesis of the layer below, or the notes in the case of the first layer, and holds segments of its layer's unit.
- **Its own rules and score.** A layer's hypotheses are scored only by that layer's rules. A complete interpretation's score is the sum of every layer's score along it.
- **The same machinery.** Every layer has:
  - links to its own past;
  - a ranking by the best complete interpretation that contains each hypothesis;
  - a record of what pruning drops;
  - rule access to the layer's own history and to the layers below;
  - traces and rank histories.
- **Revision.** Choosing which earlier hypothesis a new one follows, by the score of the complete interpretation, is how later music revises earlier beliefs.

## Layers in the engine and in names

- **In the engine,** layers form an ordered sequence, and the same machinery runs for each in turn. No layer is special-cased.
- **For people,** each layer is named for its unit, so that a trace can say, for example, that the key areas revised after a cadence changed the key. Sequence numbers would be renumbered whenever a layer is inserted, and would say nothing in a trace.

## Open questions

- **The key layer's unit.** It could stay one key reading per chord, or become a key area spanning several chords. This bears on whether a key may change within a chord segment.
- **Ranking the chord layer.** Chord hypotheses are now ranked only through the key hypotheses that read them, which measured better than quotas per layer. Whether every lower layer should work this way, or have a beam of its own, is still open.
- **Score bookkeeping.** Key hypotheses store a running total of both layers. Storing only each layer's own score shifts figures by about 0.1 RN through rounding.
- **Revising a topic at a later layer.** A later layer that judges the key again could add hypotheses of its own, or revise the earlier layer's. How the two judgments relate in the output and in traces is open.
- **Hooks in the wrong layer.** The hooks that merge near-duplicate hypotheses and that open rival readings are chord-layer ideas, but they now exist only in the key layer.

## Constraints

The constraints of `layered-listener.md` hold. Every stage is measured on the full benchmark suite and on runtime, and a pure restructuring should leave the figures unchanged.
