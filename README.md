# harmo

Rules-based harmonic analysis of MusicXML scores that offers **several competing readings** instead of a single answer. Every reading comes with a trace of the rules behind it, each citing its source. The analysis also records how a reading's rank changed while the piece unfolded.

## How it works

The engine reads the score left to right, one **slice** at a time (a span in which no note starts or stops). At every slice, each hypothesis in the beam either extends its current chord segment or opens a new one. A new segment gets a freshly proposed **reading**: a chord, the key it is heard in, and its function there (diatonic, mixture, chromatic or applied). The beam after each slice stands for the interpretations a listener might hold at that moment, so no rule may use music more than two quarter notes past the slice it scores, the same lag allowed to offered readings below, and no rule may use statistics of the whole score. The reading reported for each moment is the best final hypothesis traced back, so later music can still decide between hypotheses that were each scored this way, as a listener revises an earlier hearing. [Search](#search) describes the search step by step, with its settings and where each value came from.

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

## Search

The search follows the dynamic-programming method that Temperley uses for his preference-rule models: it reads the piece left to right and, at each segment, keeps the best analysis so far ending in each possible state, so that the best final analysis can be traced back and an earlier choice can be revised when later music favors another (Temperley 2001, pp. 15–19; for harmony and key, pp. 157–159 and 189; on revision, pp. 206–207). Temperley's states are few, such as one per key in his key model, so he keeps one analysis for each of them and finds the best analysis exactly. Here a state would be a whole reading together with the segments before it, far too many to keep, so the engine keeps only a fixed number of the best hypotheses. This is a beam search, which keeps several analyses at once and limits their number by pruning (Temperley 2001, p. 29). It may lose the best analysis if that analysis falls out of the beam early on.

### What happens at each slice

1. **Extend.** Every hypothesis in the beam extends its current segment by the slice, and its score gains the slice rules' score of its reading on the slice.
2. **Propose.** The proposers suggest chords for the slice. Each chord is read in each of the 44 keys (tonics B𝄫 to B♯, major and minor) in every way the interpreters allow. Each reading is scored by the slice rules as the opening slice of a segment, and the best 48 are kept.
3. **Open.** Each kept reading opens a new segment after the hypothesis in the beam that leads into it best. That hypothesis's score is added to the close rules' score of its last segment, then to the transition score of moving to the new reading (departure scores count only if the key changes), and the hypothesis with the highest total becomes the parent. Admit rules can rule a parent out. A reading opened on a slice therefore appears once, with its best history, which is the step that Temperley's method takes for every state.
4. **Prune.** The extended and opened hypotheses are sorted by score, best first, and kept in that order until the beam holds 80. A hypothesis is skipped if a better one already kept gives its last four segments the same readings, differing only in where those segments begin. Ties keep the order of creation: extended hypotheses first, in beam order, then opened ones, best fit first.

A hypothesis's score is the sum of every weighted rule score along its path, higher being better. The scores are not divided by length, because every hypothesis in the beam covers the same slices.

### Readings reported

- **The retrospective reading** comes from the end of the piece. Each final hypothesis's last segment is scored by the close rules, the hypotheses are ranked, and the best one is traced back through its segments' links to the segments before them.
- **The reading at the moment,** which the bench scores as `key now` and `rn now`, is the reading that the best hypothesis gives the slice in the beam two quarter notes later: the beam after the last slice that starts no more than two quarter notes after the slice in question starts.
- **Offered readings** come from that same beam: up to four distinct labels for the slice, taken from hypotheses within 4.0 of its best score, in rank order.
- **Alternatives in `analyze`** come from the eight next-best final hypotheses. A segment is marked interpretive (`?`) when one of them reads its first slice differently and scores within 4.0 of the best.
- **Traces** are not stored. Rules are pure, so `analyze --json` rebuilds each segment's trace by evaluating every rule again along the path.
- **Rank histories** follow a reading through the beam after every later slice, recording each slice where its rank changes.

### Settings

The first six settings can be changed on the command line of `bench`, `diagnose`, `results` and `sweep`; `analyze` and `inspect` use the defaults. Where a value was measured, the commit that set it gives the figures.
- **Beam width, 80** (`--beam`): the number of hypotheses kept after each slice. It was 48 from the first commit (4e0fbe2) until round 4, when commit 254ad4c raised it to 80 along with the next two settings: fewer candidates made each slice cheaper, and the time saved paid for a wider beam. Against the round 4 baseline that combination gained 0.16 RN on `tune` and 4.2 points of both readings offered on `agreement-tune`, at the same CPU time.
- **Candidate readings per slice, 48** (`--candidates`): how many new readings are proposed per slice, ranked by their fit to the opening slice. It was 96 from the first commit until 254ad4c. More candidates mostly added close variants of one chord in neighboring keys, which pushed older interpretations out of the beam.
- **Distinct depth, 4** (`--distinct-depth`, 0 to turn it off): how many recent segments' readings must differ between two hypotheses for both to be kept. Commit 77533ec added it in round 2 with a depth of 3, after measuring that about 12 of the 48 beam places went to hypotheses that differed only in where their segments begin. 254ad4c raised it to 4. It is a heuristic: close rules and some transition rules read segment lengths, which differ between the hypotheses it merges.
- **Offer count, 4** (`--offer-count`): the most readings offered for a moment. Commit 5c6be83 set it in round 2. Readings had been taken from the eight best hypotheses, which mostly shared one reading, so about 1.9 were offered although the beam held about four distinct readings. No count other than four has been measured. Coverage rises with the number offered, so the bench reports the mean number offered beside it.
- **Offer margin, 4.0** (`--offer-margin`): how far below the best score an offered reading's hypothesis may fall. Temperley treats a passage as ambiguous when its best analyses score about equally, and leaves open how close they must be (2001, pp. 219–220). The value 4.0 dates from the first commit (4e0fbe2) and was not chosen by measurement. In round 5, commit ccea563 found that it rarely limits what is offered (removing it raised the mean offered on `agreement-tune` only from 3.57 to 3.63), and step 4 now uses a smaller margin to compare coverage at an equal mean number offered.
- **Offer lag, 2 quarter notes** (`--offer-lag`): how long after a moment the beam is read for offered readings and the reading at the moment, as a listener who has heard a little further. It dates from the first commit and was not chosen by measurement. It also bounds how far ahead any rule may look.
- **Keys, all 44**: every slice considers every key from B𝄫 to B♯. Commit f8acf4e set this in round 4, so that the keys available never depend on music not yet heard; it cost 0.02 RN on `tune` against admitting only the keys a piece spells.
- **`analyze` alternatives, 8, and interpretive margin, 4.0**: fixed in the code since the first commit, and not measured.

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
./target/release/harmo results                    # regenerate RESULTS.md
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

**Ground truth** is placed on the score by measure number. Bars that a score splits around a repeat sign are joined, scores whose numbering skips or restarts are numbered by position, and first and second endings (`m12a`, `m12b`) are matched whether the score numbers both endings or leaves the second unnumbered. Spans known to be mislabeled are listed in `EXCLUDED_SPANS` in `tools/prepare_corpus.py` and skipped. Four test pieces have corrected labels (`KEY_CORRECTIONS` and `MEASURE_SHIFTS`), which adds about 1 point of RN to WiR and 2 to ABC; two of the corrections change labels that AugmentedNet's published figures were scored against.

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

**[`RESULTS.md`](RESULTS.md)** has every figure that `bench` and `diagnose` compute on the standard sets, with notes on how to read them. Regenerate it with `harmo results` rather than editing it by hand.

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

A change can be a cited rule, a vocabulary entry or a weight, or a change to the search or the theory model, such as how the beam is pruned or what a key can be. Its size does not matter. Making one change at a time keeps each change's effect measurable, so a large change is tested as one unit. Before benching a change, write down which `diagnose` error kind it targets and about how many frames are at stake.

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

At the end of the round, regenerate `RESULTS.md` with `harmo results`.

## Development

- `cargo test` runs the unit tests and an end-to-end test on `tests/fixtures/cadence.musicxml`.
- To check formatting, lints and tests on every commit, run `git config core.hooksPath .githooks`.
- CI (`.github/workflows/ci.yml`) runs the same checks, plus ruff on `tools/`.
