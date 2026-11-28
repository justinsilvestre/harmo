# A layered model of the listener

This note states a problem in harmo's model and the direction a solution should take. It leaves the design and its implementation to the work that takes it up.

## The problem: a reading is one indivisible whole

Each hypothesis holds a reading made of a key, a chord and a function together. This has two consequences.

**Readings of the same notes compete as rivals.**
- **The effect:** V and V7 over the same notes are two whole readings, and so are one chord over two different basses. They take separate places among the offered readings and in the beam, and push out readings that differ in substance, such as a reading in another key.
- **The evidence:**
  - **A rival V beside V7** (`chord.seventh_rival`) raised how often both analysts' readings are offered by about 3 points, but lowered the key column by 0.8.
  - **Rival basses** raised the inversion class from 11% to 21% both offered, but lowered key coverage by about 6 points. About half of that loss happened in the beam, not among the offered readings.

**The model cannot show how one belief leads to another.**
- **The goal:** harmo models a listener whose understanding develops as the piece unfolds. A listener's sense of the key shapes how the chords are heard, and the chords and cadences in turn shape the sense of key.
- **The gap:** a joint reading carries a single score, so its trace cannot say which layer's belief informed which other layer's, or when.

## What is known

- **Temperley's treatment.** Temperley treats two interacting preference rule systems as one combined system. Each system has its own rules, and further rules score how well the systems' structures fit together (2001, pp. 157–158). harmo's joint search follows this treatment.
- **The rules already have a direction.** A survey of the 31 rules that score found:
  - **Chord rules:** they read only the notes, the chord's own past and the bass, never the key.
  - **Key rules:** they read the chord.
  - **Function:** it is nearly determined by the key and the chord.
  - **The interaction rules:** 11 rules judge several layers at the same moment. Most of them concern cadences and progressions.
  - **Look-ahead:** where a rule looks ahead, it reads only the notes.
- **The engine imposes the joint reading, not the rules.** The engine:
  - proposes whole key, chord and function readings;
  - ranks those readings together when it cuts the candidates;
  - lets the key change only where a chord segment begins.

## The direction

- **Layers as beliefs.** Each layer (chord, key and function, and later cadence and phrase) holds its own evolving belief about the music, with its own alternatives.
- **Layers interpret one another.** A later layer interprets what an earlier layer holds. Two readings of the same notes are then alternatives within the chord layer, not rival wholes.
- **Loops unfold in time.** Where layers inform each other, the influence runs from what was believed before to what is heard now. No step is circular, and each step can be traced.
- **Revision stays possible.** Later music can still promote a reading that was held earlier.

## Open questions

- What the chord layer's unit is, given that chord boundaries and non-chord tones are partly interpretive.
- Whether a tonicized key belongs to the key layer or to function.
- How the interaction rules fit: as scores on several layers jointly, or as influences that take effect at the next moment.
- How the layers' scores combine to rank complete interpretations.
- How alternatives are counted when coverage is measured.

## Constraints

- **The listener model:** no rule may use music more than two quarter notes ahead.
- **Rules:** every judgment is made by a cited rule, and every reading keeps its trace.
- **Measurement:** a change of this kind is judged on the full benchmark suite and on runtime. A Roman numeral is right only when its chord and its key are both right, and the joint search judges the two together, so separating the layers could cost accuracy on full Roman numerals. RN and RN eq are therefore watched closely, and any fall is measured and weighed against what the change gains.
