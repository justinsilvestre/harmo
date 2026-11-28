# What a layer is

The search now has two layers: chord hypotheses, and key hypotheses that read them (see `layered-listener.md` and `layered-search.md`). The two layers are built differently, and the second has no unit of its own. This note states what every layer should have in common, for the work that generalizes the engine to more layers.

## Where the two layers stand

- **Chord hypotheses** have segments of their own: where a chord begins, what it is and its bass.
- **Key hypotheses** have no unit of their own. A key hypothesis's segment is a chord segment with a key and a function added. A layer whose unit spans several chords, such as a cadence or a phrase, cannot be built this way.
- **The machinery is shared where nothing musical tells the two layers apart.** Both layers:
  - index their hypotheses with the same kind of typed index;
  - link each hypothesis to its own layer's hypothesis at the slice before and to the last one of the segment before;
  - give transition hooks their own layer's earlier segments;
  - walk paths and traces by their own links;
  - store only their own layer's score.

  One difference remains: rank histories exist only for whole readings. The hooks that merge readings and name rival candidates stay in the key layer, and the hook that names rival chords is in the chord layer (see the settled questions).
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

## Settled questions

- **Ranking the chord layer.** Chord hypotheses are ranked only through the key hypotheses that read them, with a limit on how many chord hypotheses the beam may hold. This measured better than quotas per layer, and it stays.
- **Score bookkeeping.** Each layer's hypotheses store only their own layer's score, and a complete interpretation's score is the sum along it. No running total is stored. The change moved figures by less than 0.1 point, through rounding alone.

- **The key layer's unit is one key reading per chord segment.** A key area is the run of chord segments read in one key, and the key layer already holds it as that run. The evidence:
  - **The analyses give one key per chord.** In the main analyses of `tune`, every one of the 110,854 labels has a single key, and a key area runs for a median of 8 labels. Temperley's Roman numeral program likewise gives each chord the key of its place in the key analysis (2001, pp. 188–189).
  - **A key area unit would add nothing that can be read or scored.** The rules that judge areas, such as `key.area_duration` and `key.unconfirmed_departure`, read an area from the key layer's own history, and the function of each chord needs a key for each chord in any case.
  - **A change of key on a held chord needs no new unit.** Of the 5,649 key changes in those analyses, 240 (4.2%) relabel the same chord in a new key. The search reads these by opening a new segment of the same chord in the new key, which its top readings already do (in one Mozart and one Beethoven movement among five pieces checked). A key therefore never changes within a chord segment.
- **A later layer that judges a topic again adds hypotheses of its own and never edits those below.** A layer that judged the key again, such as one of cadences or key areas, would read key hypotheses and score them with its own rules. Its judgment changes which key is believed by changing which complete interpretations score best, so it revises the earlier layer's beliefs by promotion, as later music does now. The evidence:
  - **Revision by promotion is what Temperley's model does,** keeping several provisional analyses and choosing among them as later segments are heard (2001, pp. 205–207).
  - **It already carries the engine's revisions.** On `tune`, the retrospective reading gets the key right in 83.0% of frames and the reading at the moment in 78.9%, and RN 58.5% against 54.0%, without any layer editing another.
  - **Editing a stored hypothesis would break the model.** A lower hypothesis is shared by every hypothesis that reads it, so an edit made for one interpretation would change the others, and the beam at a past slice would no longer hold what was believed then. Each layer's own judgment stays in its own hypotheses and in its own rules' traces.

- **Rival chords belong to the chord layer, while rival candidates and merging stay in the key layer.** The three hooks were all in the key layer. Each was tested where its judgment is made:
  - **Rival chords move to the chord layer.** `chord.seventh_rival` keeps a triad and the seventh chord on its root alive as two chord hypotheses of one span. It read the key only to require that the chord be a dominant, which its source does not ask: a seventh that sounds for only part of a chord may be read either way (Kostka, Payne and Almén 2023, p. 219). As a chord rule that groups a major triad with the dominant seventh chord on its root, it changes no figure while off, as it is by default. Switched on, it matches the key layer's version on `tune` (RN and key within 0.01; whole readings both offered: RN +0.06 [-0.00,+0.13], chord +0.06 [-0.03,+0.16] at an equal number offered). Every rule now hooks into its own topic's layer, and the key layer has no rival hook.
  - **Rival candidates stay in the key layer.** `function.cadential_rival` keeps the dominant beside a cadential six-four read as I64, in the same key. Whether a six-four is cadential depends on the key, since its bass must be the key's dominant, and the tonic triad's quality follows the key's mode. The candidates it protects are whole readings, which are cut by the fit of chord and key together (see `layered-search.md`).
  - **Merging stays in the key layer, and stays off.** Merging hypotheses by their chord layer's recent segments, compared by chord and bass, lost more than merging by whole readings on `tune` (prototype on branch `merge-chord`, commit d90b08d): at a depth of 4, RN -0.57 [-0.75,-0.38] and key -0.40 against no merging, with 30% more CPU time, where merging by whole readings loses RN -0.22 [-0.39,-0.08]. The default depth stays 0.

## Constraints

The constraints of `layered-listener.md` hold. Every stage is measured on the full benchmark suite and on runtime, and a pure restructuring should leave the figures unchanged.
