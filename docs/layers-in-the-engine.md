# Layers in the engine

This record says how the engine expresses the layers of [the layer model](layers.md) as one ordered sequence, with the same machinery for each. It covers the chord and key layers that exist, what a further layer would add, and how the questions settled in `layers.md` limit what the machinery must allow.

## What a layer is in code

A layer is defined by five things, and only these differ from one layer to another:
- **Its unit:** the type of what each of its hypotheses holds of its latest segment. The chord layer's unit is the `ChordSegment` (where the segment begins, its chord and its bass). The key layer's unit is the `Function` of the chord segment it reads: its key, the key it tonicizes and its category.
- **What it reads:** each hypothesis reads one hypothesis of the layer below. The chord layer reads the notes, so a chord hypothesis records the last slice it has heard.
- **Its view:** the segment as the layer's rules receive it, built from the unit and what the hypothesis reads. The chord layer's view is the chord segment itself. The key layer's view is the `KeySegment`, a chord segment with its function.
- **Its hooks and rules:** the hook kinds of the layer, what each receives, and the layer's group of rules in a rule set.
- **Its steps:** how the layer proposes units for a segment opening on a slice, extends a hypothesis by a slice, opens a hypothesis after another, and scores leaving one.

## What every layer shares

The rest is written once and used by every layer:
- **A typed index and a hypothesis record.** `HypothesisId<U>` indexes the hypotheses of the layer whose unit is `U`. `Hypothesis<U, R>` holds the links of every layer: its parent (its layer's hypothesis at the slice before), its previous hypothesis (the last one of the segment before), what it reads (`R`), its unit and its own layer's score.
- **A store.** Each layer keeps its hypotheses in an arena, `Store<U, R>`, to which each slice's kept hypotheses are added together, best first. A slice's stretch of a store is therefore that layer's ranking at the slice.
- **Paths and histories.** Paths, traces and the histories that transition hooks read follow a layer's `previous` links within its own store.
- **Sharing.** A hypothesis of a lower layer is extended once per slice, and opened once for each hypothesis it follows and each unit proposed, however many hypotheses above read it.
- **The score of a complete interpretation.** It is the sum of the scores of the hypothesis at the top and of each hypothesis it reads, down to the first layer.
- **Ranking.** Each layer's hypotheses are ranked by the best complete interpretation that reads them. When the beam is pruned, the kept hypotheses of the top layer are stored in order, and each lower layer stores the hypotheses they read, each once, in the order first read. This gives every layer its ranking with no further sorting.
- **Pruning and its records.** Each layer has a width: the top layer's is the beam width, and a lower layer's is the most hypotheses of that layer that the kept hypotheses may read. A hypothesis that would exceed a width is dropped, and the record names the layer whose width it would exceed.
- **Traces.** Each rule is traced by evaluating it again on its own layer's hypothesis along the path, in rule order.
- **Rank histories.** A history follows one layer's reading of a slice through that layer's later rankings, by its parent links. The rank histories of whole readings follow the top layer.

## The sequence, and keeping it portable

The layers form a sequence in three places, always in the same order, lowest first:
- **`Layer`** names the layers, each for its unit, and settings that differ by layer, such as widths and pruning causes, are looked up by it.
- **`Layers`** holds one store per layer.
- **The search's step** runs each layer's steps in turn, since each layer's new hypotheses read those of the layer below. The top layer's hypotheses are complete interpretations, and the beam holds them.

The units of different layers are different types, so the sequence is written out, one store and one step per layer, rather than held in a list. A list of layers would need trait objects, or an enum of units that every stage would match on, which would special-case each layer everywhere instead of in one place.

This keeps the code portable to TypeScript:
- **Generic records and functions only hold and link data.** They never call a method of a unit, so they need no traits. In TypeScript they become generic types and functions, and `HypothesisId<U>` a branded number.
- **A layer's behaviour is plain functions** in the layer's own modules, as rules are plain structs of function pointers.
- **Settings by layer** are matches on `Layer`, which become records keyed by a string union.

## Adding a layer

A cadence layer above the key layer, for example, would need:
- a unit (a cadence segment) and its view (the cadence with the key segments it spans);
- a hook enum and rule group, with the contexts its hooks receive;
- its steps, written as the key layer's are;
- a store in `Layers`, a width setting, and its place in the search's step.

The shared machinery then gives it links, paths, histories for its hooks, sharing, ranking, pruning records, traces and rank histories. The key layer would become a lower layer, read by cadence hypotheses, and the beam would hold cadence hypotheses.

## How the settled questions shape the structure

Three decisions of `layers.md` set how much the shared machinery must allow:
- **Every layer opens a segment where the layer below does.** The key layer's unit is one key reading per chord segment, so a key hypothesis opens exactly when its chord hypothesis opens, and extends when it extends. The engine therefore opens and extends the layers together, and keeps no segment start of the key layer's own. A layer whose unit spans several segments of the layer below, such as a cadence, would be the first to need a hypothesis that extends while the one it reads opens. The links already allow that, so it would change that layer's steps, not the shared records.
- **A layer never changes the stores below it.** A later layer that judges a topic again adds hypotheses of its own, which read the layer below and are scored by its own rules. Stored hypotheses are never edited, and a later layer's judgment takes effect through the score of the complete interpretation. A topic could then be judged in more than one layer, so the check that each rule hooks into its topic's layer would list the layers each topic may use.
- **Rival chords are a hook of the chord layer.** A chord exit holds its rival groups, and a new segment's second predecessor is the best hypothesis whose chord hypothesis ends in another member of the group. Rival candidates stay in the key layer, which judges whole readings. Merging hypotheses is removed.
