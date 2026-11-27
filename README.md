# harmo

Rules-based harmonic analysis of MusicXML scores that offers **several competing readings** instead of a single answer. Every reading comes with a trace of the rules behind it, each citing its source. The analysis also records how a reading's rank changed while the piece unfolded.

## How it works

The engine reads the score left to right, one **slice** at a time (a span in which no note starts or stops). At every slice, each hypothesis in the beam either extends its current chord segment or opens a new one. A new segment gets a freshly proposed **reading**: a chord, the key it is heard in, and its function there (diatonic, mixture, chromatic or applied). The beam after each slice stands for the interpretations a listener might hold at that moment, so no rule may use music more than two quarter notes past the slice it scores, the same lag allowed to offered readings below, and no rule may use statistics of the whole score. The reading reported for each moment is the best final hypothesis traced back, so later music can still decide between hypotheses that were each scored this way, as a listener revises an earlier hearing. When the beam is pruned, hypotheses that give their last four segments the same readings and differ only in where those segments begin keep just the best of them, so that the beam's places go to distinct interpretations (`--distinct-depth`, 0 to turn this off).

The engine knows nothing about music. All musical knowledge lives in **rules**, which hook into the engine at seven points:

| hook | what it does | example |
|---|---|---|
| `ProposeChords` | proposes chords for a segment opening on a slice | `chord.propose_templates` |
| `Interpret` | lists how a chord can function in a key (a hard constraint: no entry, no reading) | `function.vocabulary` |
| `Slice` | scores one slice against a reading | `chord.compatibility`, `key.profile` |
| `Transition` | scores opening a new segment after the previous one | `key.modulation`, `function.applied_resolution` |
| `Departure` | scores leaving the previous segment's key, from the hypothesis alone (scored once per hypothesis, not per new reading) | `key.area_duration`, `key.unconfirmed_departure` |
| `Close` | scores a segment once its extent is known | `chord.seventh_evidence`, `function.six_four` |
| `Admit` | rejects transitions outright | (none yet) |

Rules are pure functions with a weight, a layer (`chord`, `key`, `function`), a summary and citations. Because they are pure, the search stores only scores, and traces are rebuilt on demand by evaluating the rules again along a path. Run `harmo rules` to list the catalog.

### Layout

- `src/engine/` is the core: the search (`search.rs`), the rule interface (`rule.rs`, `ruleset.rs`), path reconstruction, traces and rank histories.
- `src/rules/` is the rule catalog. Files are prefixed by layer (`chord_*`, `key_*`, `function_*`), and `sources.rs` holds the bibliography.
- `src/theory/` holds keys, chord templates and Roman numeral labels.
- `src/score/` holds the MusicXML reader, meter and slicing.
- `src/output/` builds the analysis document: readings, alternatives, `Determined`/`Interpretive` status, traces and rank histories.
- `src/bench/` loads the corpus, computes metrics, runs weight sweeps and writes reports.
- `tools/prepare_corpus.py` is the only Python. It runs once, offline, to download the corpora and convert the RomanText analyses to JSON with music21. `tools/prepare_corpus_anet.py` adds the AugmentedNet corpora that When in Rome has no MusicXML for, converting Humdrum scores to MusicXML.

The code avoids trait objects and macros: rules are plain structs holding function pointers, and hypotheses live in an index-linked arena. This keeps a TypeScript port straightforward.

## Usage

```sh
uv run tools/prepare_corpus.py            # once: fetch corpora into data/ (about 5 minutes on a fresh checkout)
cargo build --release

./target/release/harmo analyze score.mxl          # readings, with "?" marking interpretive spans
./target/release/harmo analyze score.mxl --json   # full document: traces, alternatives, rank histories
./target/release/harmo inspect k282-3             # ground truth beside the engine's reading
./target/release/harmo bench --set tune --label my-change
./target/release/harmo bench --set test --set agreement --label my-change
./target/release/harmo diagnose --set tune --set agreement-tune   # wrong frames by kind, ranked
./target/release/harmo sweep sweeps/key-area.json
```

`bench` prints accuracy by set, the change against the previous run (`runs/latest.json`, or `--compare FILE`), and AugmentedNet's published numbers where they apply. Each run is saved to `runs/`. To try different weights, pass `--weights FILE` with a JSON object of rule ids and weights; a weight of 0 switches a rule off.

`diagnose` runs the same evaluation and assigns every frame the top reading gets wrong to exactly one kind, ranked by frames lost, for each set and its subsets:
- **Where the analysts agree, or there is only one:** key errors by relation (relative, dominant, subdominant, parallel, tonicization against modulation split by which side tonicizes, other), each sustained or brief; then, in the right key, a root a third, fifth or step away, a function (applied or not), a seventh added or missing, or an inversion. Labels the engine cannot produce at all are counted apart, by reason, using the rule set's own proposers and vocabulary. A label that is right but shifted by up to a quarter note counts as timing.
- **Where they disagree:** frames are classified by what the analysts differ on (cadential six-four as I64 or V, tonicization against modulation, seventh against triad, and so on). For each class it reports how often the top reading matches either analyst and how often both readings are offered, matching the `disagreements` lines of `bench`, and why a missing reading is missing: the engine cannot produce it, it was in the beam below the offer cut-off, or it was not in the beam at all.
- For each set, it splits the frames into those with the key right and wrong, gives the quality, inversion, root and chord accuracy in each, and reports the share of chord-wrong frames whose key is right.
- It also prints chord changes per bar, how many distinct readings the beam holds at each slice, and the longest wrong spans with their measure numbers. `--json` writes everything, including the common label pairs of each kind.

## Benchmarks

**Sets**
- `test` and `dev` are the AugmentedNet test and validation splits, with one subset per AugmentedNet module:
  - `wir`, `abc_dcml` (Beethoven quartets), `mps` (Mozart sonatas) and `wirwtc` use the MusicXML scores in When in Rome.
  - `haydnsun` (Haydn's Op. 20 quartets) uses Humdrum scores converted to MusicXML, and `tavern` (Mozart and Beethoven variations) and `bps` (Beethoven sonata first movements) use MuseScore exports. Their analyses are the When in Rome versions that AugmentedNet pins.
- `tune` has every piece outside the test split: 975 pieces, against 45 in `dev`. Its subsets are `tune-<module>` for the training and validation pieces of each module, and `tune-<category>` for When in Rome categories outside AugmentedNet, such as `tune-early_choral`, `tune-openscore-liedercorpus` and `tune-textbooks`.
- `agreement` has the 150 When in Rome pieces with two or more expert analyses: 100 Bach chorales and the Mozart sonatas. `agreement-tune` leaves out its pieces in the test split. TAVERN's two analyses per piece are reported under `tune-tavern` but are not part of `agreement`.

**Ground truth** is placed on the score by measure number. Bars that a score splits around a repeat sign are joined, scores whose numbering skips or restarts are numbered by position, and first and second endings (`m12a`, `m12b`) are matched whether the score numbers both endings or leaves the second unnumbered. Spans known to be mislabeled are listed in `EXCLUDED_SPANS` in `tools/prepare_corpus.py` and skipped.

**Accuracy** scores the engine against one analysis of each piece, as AugmentedNet does. Labels are compared at every thirty-second note (a frame), and each column gives the share of frames that are right:
- **Key, degree, quality, inversion, root:** AugmentedNet's five features. Degree includes the secondary degree, such as the "/V" of V7/V.
- **RN:** the full Roman numeral, right only when all five features match.
- **Chord:** root, quality and inversion all match, whatever the key.
- **RN|key:** RN accuracy over only the frames whose key is right.
- **Offered:** the share of frames whose full Roman numeral is among the readings the engine offers (defined below). The accuracy table prints the mean number offered beside it.
- **Key now, RN now:** the reading a listener would hold at the moment, which is the best hypothesis in the beam two quarter notes later. The other columns score the retrospective reading, the best final hypothesis traced back through the piece, as AugmentedNet's do. The gap between RN and RN now shows how much the engine relies on later music to revise earlier labels.

**Expert agreement** scores the engine against two analyses of the same piece, where a piece has them (`agreement`, `agreement-tune` and `tune-tavern`):
- **Where the analysts agree,** the target is for the top reading to match them at the square root of their pairwise agreement rate, about as often as a third expert would.
- **Where they disagree,** the measure is how often both readings are among those the engine offers ("both offered"). How often the top reading matches either analyst is reported beside it.
- **Offered readings** are up to four distinct readings held by hypotheses within a score margin of the best, two quarter notes after the moment, as a listener who has heard a little further might entertain them. The bench reports the mean number offered, so that coverage is read against how much is offered.

Tune on `tune`, and report on `test`.

### Current results

These figures come from the test run at the end of round 5 (`bench --set test --set agreement`). The corpora are WiR (When in Rome), ABC (Beethoven string quartets), MPS (Mozart piano sonatas), WTC (Bach's Well-Tempered Clavier), HaydnSun (Haydn's Op. 20 quartets), TAVERN (Mozart and Beethoven variations) and BPS (Beethoven piano sonata first movements).

**Accuracy on the test split.** Each figure is the percentage of the music, counted in thirty-second-note frames, on which the engine's top reading matches the expert's label in that respect. For example, a key of 79.0 for WiR means that the engine names the expert's key for 79% of the WiR test pieces. AugmentedNet's published figures are in brackets; it publishes none for MPS.

| corpus | key | degree | quality | inversion | root | RN |
|---|---|---|---|---|---|---|
| WiR | 79.0 (81.8) | 71.7 (69.2) | 83.4 (85.9) | 87.8 (90.3) | 89.7 (90.3) | 61.2 (56.4) |
| ABC | 80.9 (83.6) | 63.4 (65.6) | 72.2 (78.0) | 70.9 (76.9) | 75.9 (78.9) | 47.7 (44.5) |
| MPS | 89.1 | 75.7 | 77.7 | 79.4 | 82.2 | 63.0 |
| WTC | 78.6 (77.2) | 63.5 (69.7) | 67.1 (75.0) | 72.9 (74.4) | 78.4 (82.7) | 48.0 (46.2) |
| HaydnSun | 74.6 (81.2) | 61.7 (62.9) | 74.7 (80.2) | 79.0 (82.7) | 82.6 (86.5) | 47.7 (48.6) |
| TAVERN | 84.3 (88.7) | 68.4 (60.0) | 77.0 (77.4) | 76.6 (78.8) | 80.0 (81.5) | 55.6 (42.6) |
| BPS | 83.2 (85.0) | 72.9 (73.4) | 81.1 (79.0) | 79.1 (73.4) | 84.7 (84.4) | 57.0 (45.4) |
| all seven | 82.3 | 68.9 | 77.0 | 77.3 | 81.5 | 54.6 |

The engine beats AugmentedNet on RN everywhere except HaydnSun, but trails it on most single features. Each of the engine's labels is one chord in one key, so a label that is right is right on every feature at once. AugmentedNet predicts each feature separately, so its features are more often right one at a time but less often right together.

**Further measures on the test split.** AugmentedNet publishes nothing comparable to these.
- **Chord:** the expert's chord (root, quality and inversion), whatever the key.
- **RN given key:** RN, counting only the music where the key is right.
- **Key now, RN now:** key and RN for the reading the engine holds two quarter notes after each moment, rather than after hearing the whole piece.
- **Offered:** the expert's RN is among the readings the engine offers. "Per frame" is how many it offers on average.

| corpus | chord | RN given key | key now | RN now | offered | per frame |
|---|---|---|---|---|---|---|
| WiR | 78.0 | 77.4 | 75.9 | 57.8 | 75.1 | 3.5 |
| ABC | 57.3 | 59.0 | 76.8 | 43.3 | 61.7 | 3.6 |
| MPS | 69.1 | 70.8 | 84.5 | 58.3 | 76.0 | 3.5 |
| WTC | 59.3 | 61.0 | 78.4 | 47.4 | 65.6 | 3.5 |
| HaydnSun | 64.5 | 64.0 | 70.6 | 44.3 | 59.6 | 3.2 |
| TAVERN | 65.7 | 66.0 | 82.3 | 52.9 | 65.9 | 3.1 |
| BPS | 68.1 | 68.5 | 77.5 | 51.8 | 71.6 | 3.6 |
| all seven | 65.5 | 66.4 | 78.2 | 50.5 | 68.2 | 3.5 |

RN now is 3 to 5 points below RN on most corpora, which is what the engine gains by revising earlier readings once it has heard more. WTC, whose preludes rarely call for such revision, loses almost nothing.

**Agreement with two experts.** The `agreement` set has the 150 pieces with two analyses: 100 Bach chorales and the Mozart piano sonatas, some of them outside the test split. Each row is a percentage of frames.

| | key | degree | quality | inversion | root | chord | RN |
|---|---|---|---|---|---|---|---|
| The two analysts give the same label | 93.6 | 89.6 | 92.0 | 91.9 | 94.0 | 86.8 | 81.7 |
| Where they agree, the engine's top reading matches them | 92.4 | 80.9 | 84.7 | 85.4 | 87.2 | 78.7 | 73.5 |
| Target: how often a third expert would be expected to match them | 96.8 | 94.7 | 95.9 | 95.9 | 96.9 | 93.2 | 90.4 |
| Where they disagree, the top reading matches one of them | 87.8 | 73.4 | 79.8 | 84.7 | 80.8 | 67.5 | 60.7 |
| Where they disagree, both readings are among those offered | 52.8 | 40.1 | 41.7 | 39.3 | 34.2 | 19.2 | 21.7 |

The target is the square root of the analysts' agreement rate: if each expert matched an ideal analysis at rate *p*, two experts would agree at about *p*², and a third would match their consensus at about *p*. The engine offers 3.4 readings per frame on average here.

**Notes.**
- Four test pieces have corrected labels (see `KEY_CORRECTIONS` and `MEASURE_SHIFTS` in `tools/prepare_corpus.py`), which adds about 1 point of RN to WiR and 2 to ABC. Two of the corrections change labels that AugmentedNet's published figures were scored against.
- On 18 cores, a full test run takes about 1.7 s and a `tune` run about 5 s.

## Improving the engine

Work proceeds in rounds. Each round finds where the engine is weakest, compares its analysis with the experts' there, and changes the engine one step at a time.

Find faults and tune on `tune`. Use `test` only to report results at the end of a round. A fault noticed in a test piece may still be fixed, but the fix is judged on `tune`.

### 1. Survey the errors

Run `harmo diagnose --set tune --set agreement-tune`. It assigns each wrong frame on `tune` to one error kind (key, root, quality, inversion, a label the engine cannot produce, and so on) and ranks the kinds by frames lost, for each corpus.

Treat the frames where the two analysts in `agreement-tune` disagree separately:
- The top reading counts as right if it matches either analyst, and the goal is to offer both readings.
- Classify the disagreements, for example cadential six-four as I64 or V, or tonicization against modulation.
- For each class, check whether the second reading is missing from the vocabulary, ranked too low, or pushed out of the beam.

Only `agreement-tune` has two analysts, so elsewhere ambiguity and error cannot be told apart.

### 2. Compare with the experts

For the largest error kinds, read weak passages in the score beside `harmo inspect` and the rule traces from `analyze --json`. Find what the expert hears that the engine does not, and trace it to a rule, a missing rule or a weight.

### 3. Check the ground truth

A systematic offset or a wrong key in the labels can look like an engine error. Fix these in the corpus preparation, not in the engine.

### 4. Make one change at a time

Each change is a cited rule, a vocabulary entry or a weight. Before benching it, write down which `diagnose` error kind it targets and about how many frames are at stake.

Keep a change only if it passes all four checks:

| check | passes when |
|---|---|
| **Accuracy** | RN on `tune` improves and its 95% interval excludes zero, or the interval of the layer measure the change targets does |
| **No corpus harmed** | no corpus loses more than about 1 RN point |
| **Coverage** | how often both readings are offered where analysts disagree does not fall, in either the RN or the chord column |
| **Mechanism** | `diagnose` shows most of the gain coming from the targeted error kind |

Record rejected and masked ideas in the commit message or the round report.

**Intervals.** `bench --compare` resamples the pieces of each set with replacement, 1000 times and paired by piece, and prints a 95% interval for every change. This includes the change in coverage for each set with two analyses.

**Coverage.**
- Compare coverage at an equal mean number of readings offered. On `agreement-tune`, coverage rises about 5 points for each extra reading offered, so otherwise a change could win or lose coverage only by offering more or fewer readings. When the means differ, rerun the side that offers more with a smaller `--offer-margin` until they agree within about 0.02, or interpolate between two such reruns.
- Check the chord column on its own, because a change can keep RN coverage by offering more keys for the same chord while offering fewer chords.
- Coverage counts as fallen only when the 95% interval of its change lies entirely below zero.
- A change that raises `offered` without raising the mean number offered also counts in its favor, as long as it passes the RN checks.

**Masked changes.** RN needs the key and the chord both right, so a chord-layer change shows only where the key is right, and a key-layer change only where the chord is right. Judge each change on its own layer's measure as well. A chord-layer change that clearly improves `chord` and `RN|key` but leaves RN flat is recorded as **masked** rather than rejected, and tried again once key accuracy improves.

**Side effects.** A gain that shows up mostly outside the targeted error kind is a side effect. Report it, but do not accept the change as a fix for that kind.

### 5. Sweep the weights

Once the rules settle, run a one-at-a-time sweep on `tune`, then combine the best settings. Turn down any gain that comes mostly from one repertoire at the expense of others.

### 6. Report on `test`

At the end of the round, run `test` once and update the results above.

## Development

- `cargo test` runs the unit tests and an end-to-end test on `tests/fixtures/cadence.musicxml`.
- To check formatting, lints and tests on every commit, run `git config core.hooksPath .githooks`.
- CI (`.github/workflows/ci.yml`) runs the same checks, plus ruff on `tools/`.
