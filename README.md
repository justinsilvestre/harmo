# harmo

Rules-based harmonic analysis of MusicXML scores that offers **several competing readings** instead of a single answer. Every reading comes with a trace of the rules behind it, each citing its source. The analysis also records how a reading's rank changed while the piece unfolded.

## How it works

The engine reads the score left to right, one **slice** at a time (a span in which no note starts or stops). At every slice, each hypothesis in the beam either extends its current chord segment or opens a new one. A new segment gets a freshly proposed **reading**: a chord, the key it is heard in, and its function there (diatonic, mixture, chromatic or applied). The beam after each slice stands for the interpretations a listener might hold at that moment, so no rule may use music more than two quarter notes past the slice it scores, the same lag allowed to offered readings below, and no rule may use statistics of the whole score. The reading reported for each moment is the best final hypothesis traced back, so later music can still decide between hypotheses that were each scored this way, as a listener revises an earlier chord hypothesis. [Search](#search) describes the search step by step, with its settings and where each value came from.

The engine represents the music and the readings (slices, chords, keys, functions and the bass), runs the search, and records traces and rankings. It makes no musical judgments of its own. Every judgment, from which readings to propose and allow to how well a reading fits and which competing readings count as different, is made by a **rule**, so that it shows in the traces and carries a citation. The search keeps two **layers**, in the order of Temperley's Roman numeral program, which gives the output of its harmonic analysis to its key analysis (2001, pp. 188–189):
- **The chord layer** holds chord hypotheses: where segments begin, each segment's chord and its bass. A chord hypothesis is scored by the chord rules alone, which never see a key.
- **The key layer** holds key hypotheses, each of which reads one chord hypothesis's latest segment in a key, with its function there. Its rules may read the chord heard at the same moment, as a later layer interprets an earlier one. Several hypotheses can read the same chord hypothesis.

Each hypothesis stores only its own layer's rule scores, and a complete interpretation's score is the sum of the two layers' scores along it, as Temperley sums the rules of interacting preference rule systems (2001, pp. 157–158). [docs/layered-search.md](docs/layered-search.md) records the decisions behind this model and what was measured.

Rules hook into the engine at fourteen points, six in the chord layer and eight in the key layer. A hook of the chord layer receives only the chord layer's state. Transition hooks of either layer can also read their own layer's earlier segments.

| hook | what it does | example |
|---|---|---|
| **chord layer** | | |
| `Chord(Propose)` | proposes chords for a segment opening on a slice | `chord.propose_templates` |
| `Chord(Slice)` | scores one slice against a chord | `chord.compatibility`, `chord.ornamental_dissonance` |
| `Chord(Transition)` | scores opening a new chord segment after the previous one | `chord.strong_beat`, `chord.implied_harmony` |
| `Chord(Close)` | scores a chord segment once its extent is known | `chord.seventh_evidence`, `chord.six_four` |
| `Chord(Bass)` | decides a segment's bass note, and so its inversion, as the segment hears each chord tone (with no bass rule, every chord is in root position) | `chord.bass_note` |
| `Chord(Rival)` | puts a chord segment in a group of rival chords for the same music, so that a new segment also opens after the best hypothesis whose chord hypothesis ends in the rival chord (see [Search](#search)) | `chord.seventh_rival` (off) |
| **key layer** | | |
| `Key(Propose)` | proposes keys for a segment opening on a slice | `key.propose_all` |
| `Key(Interpret)` | lists how a chord can function in a key (a hard constraint: no entry, no reading) | `function.vocabulary` |
| `Key(Slice)` | scores one slice against a reading | `key.context_profile`, `key.segment_profile` |
| `Key(Transition)` | scores opening a new segment after the previous one | `key.modulation`, `function.applied_resolution` |
| `Key(Departure)` | scores leaving the previous segment's key, from the hypothesis alone (scored once per hypothesis, not per new reading) | `key.area_duration`, `key.unconfirmed_departure` |
| `Key(Close)` | scores a segment once its extent is known | `key.phrase_close`, `function.picardy_third` |
| `Key(Distinguish)` | gives a segment an opaque value; pruning counts hypotheses whose recent segments get equal values as the same interpretation | `function.distinct_readings` |
| `Key(RivalCandidate)` | names a rival reading of a slice for a candidate reading of a segment opening there, which is then kept as a candidate whatever its fit | `function.cadential_rival` |

### What the engine represents

These choices define what a reading can be, not which reading is right, so they stay in the engine:
- **Spelled pitch.** Notes, chords and keys keep their spelling, so C♯ and D♭ are different pitches and G♯ major and A♭ major different keys. Temperley's harmonic model likewise takes spelled pitches as its input (2001, pp. 147, 187). A note is a chord member when it is spelled as one, which is what a segment records as the members heard and what gives its bass; rules that accept a respelled member, such as `chord.compatibility` and `chord.bass_change`, compare pitch classes themselves.
- **Slices.** The score is cut at every note onset, note release and barline, the smallest spans in which nothing changes. Any segmentation a rule could prefer is made of whole slices, so a chord can change only where some note starts or stops.
- **Metric levels.** Each position gets a level from its time signature: 4 for the downbeat, 3 for the middle of a bar with an even number of beats, four or more, 2 for other beats, 1 for beat subdivisions and 0 for anything weaker, with pickup bars measured from their barline. These are the levels of beats of a metrical structure (Temperley 2001, p. 24). Which levels count as strong is decided by each rule, usually through a parameter.
- **Chord qualities.** The thirteen templates (four triads, six seventh chords and three augmented sixths) are the vocabulary of what a chord can be (Kostka, Payne and Almén 2023, ch. 3, pp. 38–44; ch. 22, pp. 382–389). Augmented sixths are rooted as music21 roots them, so that labels compare with the corpora's. Which chords a slice suggests is decided by `chord.propose_templates`.
- **Keys.** A key is a spelled tonic from B𝄫 to B♯ in major or minor, 44 in all, a range that reaches the double-sharp and double-flat keys of modulation examples such as Reger's (commit 4e9ccb7). Which keys are considered is decided by `key.propose_all`.
- **Readings.** A reading is a chord, a key, the key it tonicizes if any, and a category (diatonic, mixture, chromatic, applied). Which combinations exist is decided by `function.vocabulary`. A segment changes key when its key differs from the previous segment's; a tonicization stays in the key. This is what departure hooks are told, and the rules decide what a change of key costs.
- **Labels.** Roman numerals are spelled in the RomanText and AugmentedNet conventions (for example `viio7` in minor implies the raised seventh, and the case of `/ii` follows the target's diatonic triad), so the bench can compare them with the corpora. The spelling decides nothing about the music.
- **The score as written.** The MusicXML reader takes notes at sounding pitch and pools every voice and part. It drops grace notes, which notation gives no duration, cue notes, which are not played, and unpitched notes. It merges tied notes into one, reads ornaments and repeats as written rather than realizing them (the ground truth is placed by measure number), takes key signatures from the first part without their often unreliable mode, and records where fermatas end; rules that read a fermata as a phrase ending, such as `key.phrase_close`, cite their source.
- **The search and its reports.** The beam, the number of candidate readings and the offered readings' count, margin and lag (see [Settings](#settings)) decide how much of the analysis is kept and shown, not how the music is heard. Every score that orders hypotheses comes from rules.

Which readings count as the same when the beam is pruned is decided by `Distinguish` rules, and which note is a segment's bass, and so its inversion, by `Bass` rules: the engine hands them each chord tone with whether it is its slice's lowest note, the slice's metric level and whether the slice opens the segment.

Rules are pure functions with a weight, a topic (`chord`, `key` or `function`, the prefix of the rule's id), a summary and citations. A rule's hook decides the layer of the search it belongs to; function rules belong to the key layer. A rule may also declare **parameters**: the thresholds, durations and ratios inside it, each with a default, a unit and its provenance (the source that gives the value, or the measurement that set it). The engine passes a scoring hook its rule's parameter values. Because rules are pure, the search stores only scores, and traces are rebuilt on demand by evaluating the rules again along a path. Run `harmo rules` to list the catalog.

### Layout

- `src/engine/` is the core: what every layer shares (`hypothesis.rs`, the hypothesis record and typed index; `layers.rs`, each layer's store; `layer_history.rs`, the history that transition hooks read), each layer's units (`model.rs`), the search (`search.rs`, which runs each slice's steps: `proposals.rs` proposes new readings, `exits.rs` scores leaving each hypothesis and finds the ones a new segment opens after, `continuations.rs` extends the beam by the slice, and `pruning.rs` chooses what the beam keeps), the rule interface (`rule.rs`, with each layer's hooks in `chord_hooks.rs` and `key_hooks.rs`, and what key hooks receive in `key_contexts.rs`, and a rule set's hooks by layer in `ruleset.rs`, `chord_rules.rs` and `key_rules.rs`), path reconstruction, traces (`trace.rs`, with each layer's tracer in `chord_trace.rs` and `key_trace.rs`) and rank histories.
- `src/rules/` is the rule catalog. Files are prefixed by topic (`chord_*`, `key_*`, `function_*`), and `sources.rs` holds the bibliography.
- `src/theory/` holds keys, chord templates and Roman numeral labels.
- `src/score/` holds the MusicXML reader, meter and slicing.
- `src/output/` builds the analysis document: readings, alternatives, `Determined`/`Interpretive` status, traces and rank histories.
- `src/bench/` loads the corpus, computes metrics, runs weight sweeps and writes reports.
- `tools/prepare_corpus.py` is the only Python. It runs once, offline, to download the corpora and convert the RomanText analyses to JSON with music21. `tools/prepare_corpus_anet.py` adds the AugmentedNet corpora that When in Rome has no MusicXML for, converting Humdrum scores to MusicXML.

The code avoids trait objects and macros: rules are plain structs holding function pointers, and each layer's hypotheses live in an index-linked arena of one generic record. The layers form an ordered sequence, lowest first, with one store and one step of the search per layer and the same machinery for each; [docs/layers-in-the-engine.md](docs/layers-in-the-engine.md) describes it and how a layer would be added. This keeps a TypeScript port straightforward.

## Search

The search follows the dynamic-programming method that Temperley uses for his preference-rule models: it reads the piece left to right and, at each segment, keeps the best analysis so far ending in each possible state, so that the best final analysis can be traced back and an earlier choice can be revised when later music favors another (Temperley 2001, pp. 15–19; for harmony and key, pp. 157–159 and 189; on revision, pp. 206–207). Temperley's states are few, such as one per key in his key model, so he keeps one analysis for each of them and finds the best analysis exactly. Here a state would be a whole reading together with the segments before it, far too many to keep, so the engine keeps only a fixed number of the best hypotheses. This is a beam search, which keeps several analyses at once and limits their number by pruning (Temperley 2001, p. 29). It may lose the best analysis if that analysis falls out of the beam early on.

### What happens at each slice

1. **Extend.** Every chord hypothesis that the beam interprets extends its current segment by the slice and gains the chord slice rules' score of its chord on the slice. Every hypothesis in the beam follows its chord hypothesis, and its own score gains the key layer's slice rules' score of its reading on the slice.
2. **Propose.** The proposers suggest chords and keys for the slice; by default every one of the 44 keys (tonics B𝄫 to B♯, major and minor) is proposed. Each chord is read in each proposed key in every way the interpreters allow. Each reading is scored by the slice rules as the opening slice of a segment, and the best 48 are kept. A rival-candidate rule can name a rival reading of the slice for a kept reading, such as the dominant for a cadential six-four read as I64; the rival is kept as well if it was proposed, whatever its fit.
3. **Open.** Each kept reading opens a new segment after the hypothesis in the beam that leads into it best. That hypothesis's score is added to the close rules' score of its last segment, then to the transition score of moving to the new reading (departure scores count only if the key changes), both layers' rules included, and the hypothesis with the highest total becomes the parent. The new segment's chord opens a new chord hypothesis after the parent's chord hypothesis, shared by every reading of that chord that opens after the same chord hypothesis. A reading opened on a slice therefore appears once, with its best history, which is the step that Temperley's method takes for every state. The exception is a parent whose chord hypothesis's last segment a rival rule puts in a group: the best hypothesis whose chord hypothesis ends in another member of the same group becomes a second parent, so that both chord hypotheses of an ambiguous chord, such as a major triad and the dominant seventh chord on its root, keep a continuation. No rival rule is on by default.
4. **Prune.** The extended and opened hypotheses are sorted by score, best first, and kept in that order until the beam holds 80. A hypothesis is skipped if it reads a chord hypothesis other than the 40 already read by better hypotheses, so each chord hypothesis is ranked by the best hypothesis that reads it. Chord hypotheses that no kept hypothesis reads are dropped. With a distinct depth above zero, a hypothesis is skipped if a better one already kept has that many last segments that the distinguish rules count as the same; at the default of zero, nothing is merged. The engine only compares the values those rules give each segment; `function.distinct_readings` gives equal values exactly to equal readings, so two hypotheses merge when they differ only in where those segments begin. Ties keep the order of creation: extended hypotheses first, in beam order, then opened ones, best fit first.

A hypothesis's score is the sum of every weighted rule score along its path, higher being better. The scores are not divided by length, because every hypothesis in the beam covers the same slices.

### Readings reported

- **The retrospective reading** comes from the end of the piece. Each final hypothesis's last segment is scored by the close rules, the hypotheses are ranked, and the best one is traced back through its segments' links to the segments before them.
- **The reading at the moment,** which the bench scores as `key now` and `rn now`, is the reading that the best hypothesis gives the slice in the beam two quarter notes later: the beam after the last slice that starts no more than two quarter notes after the slice in question starts.
- **Offered readings** come from that same beam: up to four distinct labels for the slice, taken from hypotheses within 4.0 of its best score, in rank order.
- **Readings offered by layer** come from that same beam too: up to four keys, and separately up to four chords, ranked by their shortfall, the best score in the beam minus the best score of a hypothesis holding the key or chord. Numerals are built from these keys and chords (see [Benchmarks](#benchmarks)). Only the bench uses them.
- **Alternatives in `analyze`** come from the eight next-best final hypotheses. A segment is marked interpretive (`?`) when one of them reads its first slice differently and scores within 4.0 of the best.
- **Traces** are not stored. Rules are pure, so `analyze --json` rebuilds each segment's trace by evaluating every rule again along the path.
- **Rank histories** follow a reading through the beam after every later slice, recording each slice where its rank changes. Each layer also has rank histories of its own part of a reading, its chord or its key and function, followed through that layer's ranking: its hypotheses stored at each slice, each ranked by the best hypothesis in the beam that reads it.

### Settings

The first eight settings can be changed on the command line of `bench`, `diagnose`, `results` and `sweep`; `analyze` and `inspect` use the defaults. Where a value was measured, the commit that set it gives the figures.
- **Beam width, 80** (`--beam`): the number of hypotheses kept after each slice. It was 48 from the first commit (4e0fbe2) until round 4, when commit 254ad4c raised it to 80 and also changed the candidate readings per slice and the distinct depth: fewer candidates made each slice cheaper, and the time saved paid for a wider beam. Against the round 4 baseline that combination gained 0.16 RN on `tune` and 4.2 points of both readings offered on `agreement-tune`, at the same CPU time. Rechecked in round 7: 48 and 64 lose RN or coverage, and 96 and 128 gain coverage only by offering more readings, at 10 to 35% more CPU time.
- **Chord hypotheses, 40** (`--chord-hypotheses`; 0 sets no limit): the most chord hypotheses that the hypotheses in the beam may read. Commit 46c5c6b set it, measured against the same search with no limit: on `tune` RN changed by -0.02 [-0.13,+0.09] and RN eq by +0.02; on `agreement-tune` keys offered by layer rose 1.4 points [+0.2,+3.0] with fewer keys offered, chords offered by layer rose about 0.5 at an equal number offered, and the search took 4% less CPU time. Widths from 32 to 56 gave the same within 0.2 RN. A fixed number of hypotheses per chord hypothesis cost RN at equal CPU time in the same commit (32 chord hypotheses by 4, RN -0.3), and only 64 by 4, at 1.7 times the CPU time, matched RN.
- **Candidate readings per slice, 48** (`--candidates`): how many new readings are proposed per slice, ranked by their fit to the opening slice. It was 96 from the first commit until 254ad4c. More candidates mostly added close variants of one chord in neighboring keys, which pushed older interpretations out of the beam. Rechecked in round 7: 24 and 32 lose RN, and 64 and 96 gain nothing outside the noise at 20 to 55% more CPU time. Cutting the candidates by layer instead, first the chords that best fit the slice and then each chord's best keys, cost 0.7 to 1.5 RN in commit 46c5c6b, so readings are cut whole, by the fit of chord and key together.
- **Distinct depth, 0** (`--distinct-depth`; 0 turns merging off): how many recent segments the distinguish rules compare between two hypotheses; both are kept only if the rules tell some of those segments apart. With no distinguish rule (for instance with `function.distinct_readings` weighted 0), nothing is merged. Which segments count as the same is the rules' decision; until round 7 the engine compared whole readings itself, and moving that comparison into `function.distinct_readings` changed no result. Commit 77533ec added it in round 2 with a depth of 3, after measuring that about 12 of the 48 beam places went to hypotheses that differed only in where their segments begin. 254ad4c raised it to 4. In round 7, depths 3, 5 and 6 measured within noise of 4. Round 8 set it to 0: without merging, both readings were offered more often where analysts disagree at an equal number offered (RN +0.85, chord +1.94 on `agreement-tune`), RN rose 0.17 and RN eq did not fall, and the search took about 9% less CPU time. It is a heuristic: close rules and some transition rules read segment lengths, which differ between the hypotheses it merges.
- **Offer count, 4** (`--offer-count`): the most readings offered for a moment. Commit 5c6be83 set it in round 2. Readings had been taken from the eight best hypotheses, which mostly shared one reading, so about 1.9 were offered although the beam held about four distinct readings. No count other than four has been measured. Coverage rises with the number offered, so the bench reports the mean number offered beside it.
- **Offer margin, 4.0** (`--offer-margin`): how far below the best score an offered reading's hypothesis may fall. Temperley treats a passage as ambiguous when its best analyses score about equally, and leaves open how close they must be (2001, pp. 219–220). The value 4.0 dates from the first commit (4e0fbe2) and was not chosen by measurement. In round 5, commit ccea563 found that it rarely limits what is offered (removing it raised the mean offered on `agreement-tune` only from 3.57 to 3.63), and step 4 now uses a smaller margin to compare coverage at an equal mean number offered.
- **Offer lag, 2 quarter notes** (`--offer-lag`): how long after a moment the beam is read for offered readings and the reading at the moment, as a listener who has heard a little further. It dates from the first commit and was not chosen by measurement. It also bounds how far ahead any rule may look.
- **Margins of readings offered by layer, 4.0** (`--key-offer-margin`, `--chord-offer-margin`, `--numeral-offer-margin`, each defaulting to `--offer-margin`): the largest shortfall of an offered key and of an offered chord, and the largest sum of the two for an offered numeral. They exist so that coverage can be compared at an equal number offered in each layer, and were not chosen by measurement.
- **Keys, all 44** (the rule `key.propose_all`): every slice considers every key from B𝄫 to B♯. Commit f8acf4e set this in round 4, so that the keys available never depend on music not yet heard; it cost 0.02 RN on `tune` against admitting only the keys a piece spells.
- **`analyze` alternatives, 8, and interpretive margin, 4.0**: fixed in the code since the first commit, and not measured.

## Usage

```sh
uv run tools/prepare_corpus.py            # once: fetch corpora into data/ (about 5 minutes on a fresh checkout)
cargo build --release

./target/release/harmo analyze score.mxl          # readings, with "?" marking interpretive spans
./target/release/harmo analyze score.mxl --json   # full document: traces, alternatives, rank histories
./target/release/harmo inspect k282-3             # ground truth beside the engine's reading
./target/release/harmo labelfit                   # check every analysis's labels against the notes
./target/release/harmo bench --set tune --label my-change
./target/release/harmo bench --set test --set agreement --label my-change
./target/release/harmo diagnose --set tune --set agreement-tune   # wrong frames by kind, ranked
./target/release/harmo sweep sweeps/key-area.json
./target/release/harmo results                    # regenerate RESULTS.md
```

In the `--json` document, each span's `rank_history` lists the rank (1 = best hypothesis) at which the beam held the span's reading after each slice where it changed, and each alternative has its own. `pruned_alternatives` lists, best first, readings of the span that the beam held but later dropped. Each of their histories ends with `rank: null` and a `left_beam` record: when (`slice`, `time`, `measure`), the rank it last stood at (`last_rank`), the rank of its best hypothesis among all that competed for the beam (`competing_rank`), how far that hypothesis fell below the best (`score_gap`), and why: `beam_width` when the beam was full of better hypotheses, `chord_hypothesis_width` when the beam already held as many chord hypotheses as it may, each read by a better hypothesis, and this hypothesis read another, or `distinct_depth` when a better hypothesis with the same readings of its last segments took its place (`merged_into_rank`, `merged_into_reading`). A reading that leaves the beam never returns, because every hypothesis descends from one in the previous beam. `rank_histories_by_layer` gives the same kind of history for each layer's own part of the span's reading, in that layer's ranking: under `chord`, its chord and bass as heard at its first slice, and under `key`, its key and function.

`bench` prints accuracy by set, the change against the previous run (`runs/latest.json`, or `--compare FILE`), and AugmentedNet's published numbers where they apply. Each run is saved to `runs/`. To try different weights, pass `--weights FILE` with a JSON object of rule ids and weights; a weight of 0 switches a rule off. To try different parameter values, pass `--params FILE` with a JSON object keyed by `<rule id>.<parameter name>`, such as `{"key.context_profile.half_life": 24}`; either file accepts either kind of key. A sweep's `grid` and `base` take parameter keys too, so `harmo sweep` varies parameters the same way it varies weights. `harmo rules` lists every parameter with its default and provenance.

`diagnose` runs the same evaluation and assigns every frame the top reading gets wrong to exactly one kind, ranked by frames lost, for each set and its subsets:
- **Where the analysts agree, or there is only one:** key errors by relation (relative, dominant, subdominant, parallel, tonicization against modulation split by which side tonicizes, other), each sustained or brief; then, in the right key, a root a third, fifth or step away, a function (applied or not), a seventh added or missing, or an inversion. Labels the engine cannot produce at all are counted apart, by reason, using the rule set's own proposers and vocabulary. A label that is right but shifted by up to a quarter note counts as timing. Each kind also gives the share of its frames that the equivalents measure counts as right, and each set the frames that each equivalence credits.
- **Where they disagree:** frames are classified by what the analysts differ on (cadential six-four as I64 or V, tonicization against modulation, seventh against triad, and so on). For each class it reports how often the top reading matches either analyst and how often both readings are offered, matching the `disagreements` lines of `bench`, and why a missing reading is missing: the engine cannot produce it, it was in the beam below the offer cut-off, or it was not in the beam at all. A reading not in the beam was either cut by the beam width, cut by the limit on chord hypotheses, merged away by the distinct-depth rule (with how long after the frame that happened), or never held by any hypothesis.
- For each set, it splits the frames into those with the key right and wrong, gives the quality, inversion, root and chord accuracy in each, and reports the share of chord-wrong frames whose key is right.
- It also prints chord changes per bar, how many distinct readings the beam holds at each slice, and the longest wrong spans with their measure numbers. `--json` writes everything, including the common label pairs of each kind.

## Benchmarks

**Sets**
- `test` and `dev` are the AugmentedNet test and validation splits, with one subset per AugmentedNet module:
  - `wir`, `abc_dcml` (Beethoven quartets), `mps` (Mozart sonatas) and `wirwtc` use the MusicXML scores in When in Rome.
  - `haydnsun` (Haydn's Op. 20 quartets) uses Humdrum scores converted to MusicXML, and `tavern` (Mozart and Beethoven variations) and `bps` (Beethoven sonata first movements) use MuseScore exports. Their analyses are the When in Rome versions that AugmentedNet pins.
- `tune` has every piece outside the test split: 975 pieces, against 45 in `dev`. Its subsets are `tune-<module>` for the training and validation pieces of each module, and `tune-<category>` for When in Rome categories outside AugmentedNet, such as `tune-early_choral`, `tune-openscore-liedercorpus` and `tune-textbooks`.
- `agreement` has the 150 When in Rome pieces with two or more expert analyses: 100 Bach chorales and the Mozart sonatas. `agreement-tune` leaves out its pieces in the test split. TAVERN's two analyses per piece are reported under `tune-tavern` but are not part of `agreement`.

**Ground truth** is placed on the score by measure number. Bars that a score splits around a repeat sign are joined, scores whose numbering skips or restarts are numbered by position, and first and second endings (`m12a`, `m12b`) are matched whether the score numbers both endings or leaves the second unnumbered. Spans known to be mislabeled are listed in `EXCLUDED_SPANS` in `tools/prepare_corpus.py` and skipped. Five test pieces have corrected labels (`KEY_CORRECTIONS` and `MEASURE_SHIFTS`), which adds about 1 point of RN to WiR and 3 to ABC; the corrections of Op. 74/iii and Dichterliebe 16 also change labels that AugmentedNet's published figures were scored against. Corrections elsewhere, found with `harmo labelfit`, fix analyses that name the wrong key or leave out a change of key, including ten of the BCMH chorale analyses.

**Accuracy** scores the engine against one analysis of each piece, as AugmentedNet does. Labels are compared at every thirty-second note (a frame), and each column gives the share of frames that are right:
- **Key, degree, quality, inversion, root:** AugmentedNet's five features. Degree includes the secondary degree, such as the "/V" of V7/V.
- **RN:** the full Roman numeral, right only when all five features match.
- **Chord:** root, quality and inversion all match, whatever the key.
- **RN|key:** RN accuracy over only the frames whose key is right.
- **Offered:** the share of frames whose full Roman numeral is among the readings the engine offers (defined below). The accuracy table prints the mean number offered beside it.
- **Key now, RN now:** the reading a listener would hold at the moment, which is the best hypothesis in the beam two quarter notes later. The other columns score the retrospective reading, the best final hypothesis traced back through the piece, as AugmentedNet's do. The gap between RN and RN now shows how much the engine relies on later music to revise earlier labels.
- **RN eq, RN eq now:** RN and RN now with notational equivalents of the expert's label also counted as right (see below).

**Equivalents RN** (`rn eq`) counts a frame as right when the engine's label is the expert's, or when it differs from the expert's label as given only by one of these conventions:
- **Seventh:** the expert labels a triad and then its own seventh chord on the same bass, or the reverse (V then V7), and the engine gives either label anywhere over the pair. Whether a seventh that enters partway makes a new chord is a matter of judgment (Kostka, Payne and Almén 2023, ch. 15). A seventh entering in the bass (V then V42) changes the bass and inversion, so it does not count.
- **Inversion:** the expert relabels one chord with another inversion as its bass moves (I6 then I), and the engine gives either label anywhere over the pair. Both name the same chord; Kostka, Payne and Almén (2023, p. 115) mark such a bass arpeggiation with figures alone or leave it unmarked.
- **Applied:** X/Y in a key against X in the key that Y stands for, with Y's mode from its case: V/V in C against V in G, viio7/ii in C against viio7 in d, or TAVERN's key-area labels such as I/V. Only the frame with the slash counts, not the rest of a key area. A chord applied to the tonic of the other mode (VI/i in a major key, a way of writing mixture) does not count, since that would credit the parallel key, and neither does one chord applied from two keys (V/V in C against V/IV in D).
- **Rootless:** V7 in any inversion against viio of the same key and tonicization, over the same bass (or with the V7 written in root position, which then names its silent root rather than a sounding bass), while the dominant's root is not sounding: they then name the same pitches (Burstein and Straus 2016, ch. 13; Kostka, Payne and Almén 2023, ch. 7). viio7 and viiø7 add the dominant's ninth, so they do not count.
- **Pedal:** where the score's lowest pitch class is held or restruck through two or more of the expert's labels, at least one of which leaves it out, the engine counts if it names the chord above the pedal in any inversion (where the expert's bass is not the pedal note), or the expert's chord at the pedal's start or end whose bass is the pedal note. Kostka, Payne and Almén (2023, ch. 13) leave inversions above a pedal unmarked and hear the pedal as prolonging its own harmony. A pedal six-four between two statements of one root-position chord (I–IV64–I) counts the same way, so the engine may give that chord throughout (ch. 9, pp. 139–140); V–I64–V does not, since its six-four may be cadential.
- **Timing:** the engine gives the expert's label, or one of the equivalents above, within an eighth note (`--timing-tolerance`, in quarter notes). No source gives a value, so it was measured on `tune`: the timing credit grows from 1.5% of frames at a sixteenth to 2.8% at an eighth and 4.4% at a quarter, so each further frame of reach credits less beyond an eighth (0.75, 0.65, then 0.40 points per frame), and a quarter note begins to reach whole neighboring chords.

Genuinely different readings never count: a cadential six-four read as V, a leading-tone seventh chord read as V7, an applied chord read from another key, relative or parallel keys, a tonicized area read as a modulation beyond the applied chord itself, or roots a third apart. `bench`, `diagnose` and RESULTS.md report how many frames each equivalence credits, in each corpus.

**Expert agreement** scores the engine against two analyses of the same piece, where a piece has them (`agreement`, `agreement-tune` and `tune-tavern`):
- **Where the analysts agree,** the target is for the top reading to match them at the square root of their pairwise agreement rate, about as often as a third expert would.
- **Where they disagree,** the measure is how often both readings are among those the engine offers ("both offered"). How often the top reading matches either analyst is reported beside it.
- **The rn eq column** counts two analysts whose labels are equivalents of each other as agreeing, and the engine as matching an analyst when its label is an equivalent of theirs.
- **Offered readings** are up to four distinct readings held by hypotheses within a score margin of the best, two quarter notes after the moment, as a listener who has heard a little further might entertain them. The bench reports the mean number offered, so that coverage is read against how much is offered.

**Coverage by layer** offers keys and chords on their own rather than as parts of whole readings, so that a rival reading in one key does not take an offered place from a reading in another. Every other figure is unchanged by it.
- A key's or chord's **shortfall** at a slice is the best score in the beam two quarter notes later minus the best score of a hypothesis there that holds the key or chord for the slice. A chord is its root, quality and inversion, as in the chord column.
- Up to four keys are offered, those whose shortfall is within the key margin, smallest first, and up to four chords the same way.
- Numerals pair each offered key with each offered chord, since a numeral follows from a chord and the key it is read in (Kostka, Payne and Almén 2023, pp. 54, 65). Up to four are offered, ranked by the sum of the two shortfalls and within the numeral margin. Each takes the tonicization of the best hypothesis holding its key and chord, or none if no hypothesis holds both.
- `bench` prints, for each set with two analyses, how often both analysts' keys, chords and numerals are offered where they differ in that layer, and the mean number offered in each layer. `bench --compare` gives a 95% interval for each change (the `by layer` lines).

**Coverage in hindsight** reads the same beams but ranks each reading by the best final score that a hypothesis holding it led to. A backward sweep over the hypotheses gives each one the best final score among its descendants, which one pass suffices for because every hypothesis is stored after its parent. A reading whose hypotheses all left the beam before the end has no such score and is never offered. It is reported beside coverage by layer in `harmo results` and with `bench --hindsight`, and no rule or other figure uses it. The final hypotheses descend from a single hypothesis everywhere except in the last bars of a piece, so it offers about one reading per frame.

**Offer delay** measures how soon each expert label's reading is first offered, for every analysis of each piece, in RN, key and chord. The beam after each slice is read with the offer count and margin, for the label's first slice and for the latest slice of the label heard so far, and the delay runs from the label's onset to the start of the first slice whose beam offers the reading. The search stops two bars after the onset or at the end of the label, whichever is later. Only `harmo results` reports it: the median delay, the shares offered within a beat, a bar and two bars, and the share not offered by the end of the search.

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

`harmo labelfit` checks the labels against the notes, independently of the engine. A label's **fit** is the share of the notes' sounding time, summed over voices, that belongs to its chord tones. The command reports the fit by set and lists, ranked by frames at stake times the gain in fit:
- **Moves in time:** runs of labels that fit markedly better a beat or one to eight bars earlier or later, as when an analysis numbers a pickup as m1 or leaves out an ending.
- **Other keys:** runs of labels whose figures fit markedly better read in another key, as when an analysis names the wrong key or leaves out a change of key.
- **Meters:** analyses that place labels past the end of their bar, as when the analysis and the score disagree on the meter.

`--piece TEXT` prints the fit of each key span and measure of the matching pieces, with each measure's fit when its labels move by up to two bars, and `--measures FIRST-LAST` prints each label there beside the pitch classes that sound. Many candidates are not errors: a unison passage, a pedal or a repeated phrase can fit better elsewhere by chance, so check each against the score before correcting it. To test a correction, write the ground truth into a copy of `data/` with `uv run tools/prepare_corpus.py --data DIR` and pass `--data DIR` to `labelfit` and `bench`.

### 4. Make one change at a time

A change can be a cited rule, a vocabulary entry or a weight, or a change to the search or the theory model, such as how the beam is pruned or what a key can be. Its size does not matter. Making one change at a time keeps each change's effect measurable, so a large change is tested as one unit. Before benching a change, write down which `diagnose` error kind it targets and about how many frames are at stake.

Keep a change only if it passes all five checks:

| check | passes when |
|---|---|
| **Accuracy** | RN on `tune` improves and its 95% interval excludes zero, or the interval of the layer measure the change targets does. A change aimed at coverage passes instead when coverage at an equal number offered improves with an interval that excludes zero, and neither RN nor RN eq falls |
| **No corpus harmed** | no corpus loses more than about 1 RN point |
| **Coverage** | how often both readings are offered where analysts disagree does not fall: keys and chords as offered by layer, and RN as offered in the joint list |
| **Mechanism** | `diagnose` shows most of the gain coming from the targeted error kind |
| **Not a convention** | where a change is kept for its RN gain, RN eq improves too and its 95% interval excludes zero |

Record rejected and masked ideas in the commit message or the round report.

**Intervals.** `bench --compare` resamples the pieces of each set with replacement, 1000 times and paired by piece, and prints a 95% interval for every change. This includes the change in coverage for each set with two analyses.

**Coverage.**
- Judge keys and chords by layer, and RN on the joint list. Keys and chords are offered on their own in coverage by layer (see [Benchmarks](#benchmarks)), so a rival reading in one layer takes no offered place from the other, and the key and chord lines of `by layer` decide the check. RN coverage is read from the joint list of offered readings. The key and chord columns of the joint list are still reported, but a fall in them alone does not fail a change.
- Compare each coverage at an equal mean number offered: RN at an equal mean of the joint list, and keys and chords at an equal mean offered in their own layer. On `agreement-tune`, coverage rises about 5 points for each extra reading offered, so otherwise a change could win or lose coverage only by offering more or fewer readings. When the means differ, rerun the side that offers more with a smaller margin (`--offer-margin` for the joint list, `--key-offer-margin` or `--chord-offer-margin` for a layer) until they agree within about 0.02, or interpolate between two such reruns.
- Check keys and chords on their own, because a change can keep RN coverage by offering more keys for the same chord while offering fewer chords, or by offering more chords in one key while offering fewer keys.
- Coverage counts as fallen only when the 95% interval of its change lies entirely below zero.
- A change that raises `offered` without raising the mean number offered also counts in its favor, as long as it passes the RN checks.

**Masked changes.** RN needs the key and the chord both right, so a chord-layer change shows only where the key is right, and a key-layer change only where the chord is right. Judge each change on its own layer's measure as well. A chord-layer change that clearly improves `chord` and `RN|key` but leaves RN flat is recorded as **masked** rather than rejected, and tried again once key accuracy improves.

**Conventions.** A change whose RN gain disappears under the equivalents measure was teaching the engine to match a notational convention, such as where a seventh starts or how a pedal's chords are inverted, rather than to hear the music better. Reject it, however large its RN gain. A change that raises RN eq without raising RN fixes readings the experts would accept, and counts in its favor.

**Side effects.** A gain that shows up mostly outside the targeted error kind is a side effect. Report it, but do not accept the change as a fix for that kind.

### 5. Sweep the weights

Once the rules settle, run a one-at-a-time sweep on `tune`, then combine the best settings. Turn down any gain that comes mostly from one repertoire at the expense of others.

### 6. Report on `test`

At the end of the round, regenerate `RESULTS.md` with `harmo results`.

## Development

- `cargo test` runs the unit tests and an end-to-end test on `tests/fixtures/cadence.musicxml`.
- To check formatting, lints and tests on every commit, run `git config core.hooksPath .githooks`.
- CI (`.github/workflows/ci.yml`) runs the same checks, plus ruff on `tools/`.
