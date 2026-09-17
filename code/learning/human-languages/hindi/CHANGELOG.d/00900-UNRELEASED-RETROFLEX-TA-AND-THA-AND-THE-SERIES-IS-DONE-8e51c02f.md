## Unreleased — retroflex ṭa and ṭha, and the consonant series is finished

**`HI-A1-SCR-15` closes — the full consonant series.** It was the largest open
script point in the file, and **every number in its note was wrong by the time
it closed.**

| metric | before → after |
|---|---|
| exam-point coverage | 234/282 → **235/282 (83%)** |
| **script-closure violations** | **24 → 21** |
| atoms taught | 524 → 526 |
| measurable lessons | 469 → 471 |
| writing-practice lessons | 115 → 117 |
| never-taught glyphs | unchanged (2) |
| `forward-language` | unchanged (22) |
| `atomsNeverRevisited` | unchanged (33) |
| `durationViolations` | unchanged (0) |

**The metric moved for the first time in four script changes.** ग, फ and the
nuqta each left `scriptClosureViolations` sitting at 24, because
`measureScriptClosure` had already credited them to lessons whose bodies
happened to print them. ट and ठ were not credited that way, so drawing them
cleared three real violations. The defect in `HL-C383` is still there; this is
a case it does not bite.

### What the note claimed, and what counting found

It named **fifteen** missing consonants — *ga gha cha ja jha ṭa ṭha ḍha ṇa tha
pha ba va śa* — and called them a hard ceiling on a reading paper that forbids
romanization. Walking the five vargas plus the semivowels, the sibilants and ह
against the corpus now:

| varga | state |
|---|---|
| ka-varga | क ख ग घ · **ङ undrawn** |
| cha-varga | च छ ज झ · **ञ undrawn** |
| ṭa-varga | **complete** — ट ठ ड ढ ण |
| ta-varga | complete |
| pa-varga | complete |
| semivowels, sibilants, ह | complete |

**Two left, and neither is debt.**

- **ञ** appears **three times in the whole corpus and never standalone.** Every
  occurrence is inside the conjunct **ज्ञ**, which `HI-W05-conjuncts` already
  teaches as one of its three special shapes. A letter the corpus never presents
  alone is not a letter the reader is failing to read.
- **ङ** appears **zero times**, and is not in `data/scripts/devanagari.json` at
  all — 44 letters, including the Marathi **ळ**, and not this one. Logged as
  `HL-C385`. Its sound is written with the anusvāra in modern Hindi (अंक, रंग),
  so a reader meets the sound constantly and the letter never.

If that reasoning is rejected, **the probe to remove is SCR-15's**.

### The two lessons

| lesson | glyph | chapter | first used | pen lifts |
|---|---|---|---|---|
| `HI-S142-letter-ta-retroflex` | ट | 27 | the first writing lesson | **1** |
| `HI-S143-letter-tha-retroflex` | ठ | 28 | *ṭhīk*, third chapter | 2 |

**ट is the second one-lift character in the book**, after ढ: the stem, shoulder
and open round body are a single unbroken run, then the headline.

The pair is taught on the thing that actually separates them — **the round body
closes**, and the stem stops being part of the body's run. That is one
difference you can hear and one you can only see, and it is the whole of it.

**ठ completes the fifth breath pair**, after त/थ, च/छ, ज/झ and प/फ. The puff of
breath was learned once; every pair since has cost only a shape.

### Both illustrate themselves from words already taught

**रोटी** and **घंटा** for ट; **ठीक**, **ठंड** and **आठ** for ठ. `forward-language`
did not move.

### A data-file gap found by a count that came up short

`HL-C385` exists because a script walking the five vargas expected twenty-five
stops and the ka-varga returned four. A check that had trusted the file would
have reported the series complete. Three tracks read that file, and Sanskrit
does use ङ standalone.

### Payoffs

| atom | payoff |
|---|---|
| `HI-SCRIPT-RECOG-142` | `HI-C38-pet` — **पेट** |
| `HI-SCRIPT-RECOG-143` | `HI-C42-sit` — **बैठना** |

Both chosen for a headword whose spelling turns on the letter. Neither needed
new prose, so no duration moved.

By the corrected headword-based check, never-drawn goes **4 → 2**.
