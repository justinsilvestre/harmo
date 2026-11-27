# harmo

Rules-based harmonic analysis of MusicXML scores that offers **several competing readings** instead of a single answer. Every reading comes with a trace of the rules behind it, each citing its source. The analysis also records how a reading's rank changed while the piece unfolded.

## How it works

The engine reads the score left to right, one **slice** at a time (a span in which no note starts or stops). At every slice, each hypothesis in the beam either extends its current chord segment or opens a new one. A new segment gets a freshly proposed **reading**: a chord, the key it is heard in, and its function there (diatonic, mixture, chromatic or applied). The beam after each slice stands for the interpretations a listener might hold at that moment. When the beam is pruned, hypotheses that give their last three segments the same readings and differ only in where those segments begin keep just the best of them, so that the beam's places go to distinct interpretations (`--distinct-depth`, 0 to turn this off).

The engine knows nothing about music. All musical knowledge lives in **rules**, which hook into the engine at six points:

| hook | what it does | example |
|---|---|---|
| `ProposeChords` | proposes chords for a segment opening on a slice | `chord.propose_templates` |
| `Interpret` | lists how a chord can function in a key (a hard constraint: no entry, no reading) | `function.vocabulary` |
| `Slice` | scores one slice against a reading | `chord.compatibility`, `key.profile` |
| `Transition` | scores opening a new segment after the previous one | `key.modulation`, `function.applied_resolution` |
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
- **Where the analysts agree, or there is only one:** key errors by relation (relative, dominant, subdominant, parallel, tonicization against modulation, other), each sustained or brief; then, in the right key, a root a third, fifth or step away, a function (applied or not), a seventh added or missing, or an inversion. Labels the engine cannot produce at all are counted apart, by reason, using the rule set's own proposers and vocabulary. A label that is right but shifted by up to a quarter note counts as timing.
- **Where they disagree:** frames are classified by what the analysts differ on (cadential six-four as I64 or V, tonicization against modulation, seventh against triad, and so on). For each class it reports how often the top reading matches either analyst and how often both readings are offered, matching the `disagreements` lines of `bench`, and why a missing reading is missing: the engine cannot produce it, it was in the beam below the offer cut-off, or it was not in the beam at all.
- It also prints chord changes per bar, how many distinct readings the beam holds at each slice, and the longest wrong spans with their measure numbers. `--json` writes everything, including the common label pairs of each kind.

## Benchmarks

**Sets**
- `test` and `dev` are the AugmentedNet test and validation splits, with one subset per AugmentedNet module:
  - `wir`, `abc_dcml` (Beethoven quartets), `mps` (Mozart sonatas) and `wirwtc` use the MusicXML scores in When in Rome.
  - `haydnsun` (Haydn's Op. 20 quartets) uses Humdrum scores converted to MusicXML, and `tavern` (Mozart and Beethoven variations) and `bps` (Beethoven sonata first movements) use MuseScore exports. Their analyses are the When in Rome versions that AugmentedNet pins.
- `tune` has every piece outside the test split: 975 pieces, against 45 in `dev`. Its subsets are `tune-<module>` for the training and validation pieces of each module, and `tune-<category>` for When in Rome categories outside AugmentedNet, such as `tune-early_choral`, `tune-openscore-liedercorpus` and `tune-textbooks`.
- `agreement` has the 150 When in Rome pieces with two or more expert analyses: 100 Bach chorales and the Mozart sonatas. `agreement-tune` leaves out its pieces in the test split. TAVERN's two analyses per piece are reported under `tune-tavern` but are not part of `agreement`.

**Ground truth** is placed on the score by measure number. Bars that a score splits around a repeat sign are joined, scores whose numbering skips or restarts are numbered by position, and first and second endings (`m12a`, `m12b`) are matched whether the score numbers both endings or leaves the second unnumbered. Spans known to be mislabeled are listed in `EXCLUDED_SPANS` in `tools/prepare_corpus.py` and skipped.

**Metrics** follow AugmentedNet. Labels are compared at every thirty-second note on key, degree (primary and secondary), quality, inversion and root. RN counts as correct only when all five match. Two further columns separate chord errors from key errors. **Chord** counts a frame as correct when root, quality and inversion all match, whatever the key. **RN|key** is the RN accuracy over only the frames whose key is correct.

**Expert agreement**
- **Where two analysts agree,** the target is for the top reading to match them at the square root of their pairwise agreement rate.
- **Where they disagree,** the measure is how often both readings are among those the engine offers. Offered readings are up to four distinct readings held by hypotheses within a score margin of the best, two quarter notes after the moment, as a listener who has heard a little further might entertain them. The bench reports the mean number offered, so that coverage is read against how much is offered.

Tune on `tune`, and report on `test`.

**Current results, test split (RN %):** WiR 56.1 (AugmentedNet 56.4), ABC 42.3 (44.5), MPS 59.2, WTC 45.4 (46.2), HaydnSun 44.8 (48.6), TAVERN 52.9 (42.6), BPS 56.2 (45.4). Where analysts disagree, both readings are offered for 17.7% of frames, with 2.8 readings offered on average. On 18 cores, a full test run takes about 1 s and a `tune` run about 11 s.

## Improving the engine

Each round of work follows the same loop.

1. **Survey the errors.** Run `harmo diagnose --set tune --set agreement-tune`. It breaks the wrong frames on `tune` down by kind (key, root, quality, inversion, labels the engine cannot produce) and by corpus, and ranks the kinds by frames lost.
   - Report frames where the analysts in `agreement-tune` disagree separately. There, the top reading is right if it matches either analyst, and the goal is to offer both readings.
   - Classify these disagreements (for example, cadential six-four as I64 or V, or tonicization against modulation), and check whether the second reading is missing from the vocabulary, ranked too low, or pushed out of the beam.
   - Only `agreement-tune` has two analysts, so elsewhere ambiguity and error cannot be told apart.
2. **Compare with the experts.** For the largest kinds, read weak passages in the score beside `harmo inspect` and the rule traces from `analyze --json`. Find what the expert hears that the engine does not, and trace it to a rule, a missing rule or a weight.
3. **Check the ground truth.** A systematic offset or a wrong key in the labels can look like an engine error. Fix these in the corpus preparation rather than in the engine.
4. **Change one thing at a time.** Each change is a cited rule, a vocabulary entry or a weight. Keep it only if it helps `tune` overall, costs no corpus more than about 1 RN point, and does not lower how often both readings are offered where analysts disagree. Record rejected ideas in the commit message or report.
5. **Sweep the weights.** Once the rules settle, run a one-at-a-time sweep on `tune`, then combine the best settings. Turn down any gain that comes mostly from one repertoire at the expense of others.
6. **Report on `test` once,** at the end of the round, and update the results above.

## Development

- `cargo test` runs the unit tests and an end-to-end test on `tests/fixtures/cadence.musicxml`.
- To check formatting, lints and tests on every commit, run `git config core.hooksPath .githooks`.
- CI (`.github/workflows/ci.yml`) runs the same checks, plus ruff on `tools/`.
