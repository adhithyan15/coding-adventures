## Unreleased — retroflex ṭa and ṭha, and a measurement bug caught mid-PR

**No inventory point closes.** `HI-A1-SCR-15` was probed as covered in the first
draft of this change and **the probe was withdrawn before merge**, because the
audit helper that said the consonant series was complete was wrong.

| metric | before → after |
|---|---|
| exam-point coverage | unchanged (234/282, 83%) |
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

**That table said two were left. Five are.**

The helper building it credited a glyph to any script lesson whose **headword**
contained it — and **a headword can be a whole word**:

| lesson | headword | what it silently taught |
|---|---|---|
| `HI-W01-shirorekha-na-ma` | शिरोरेखा | **श** and **ो** |
| `HI-W06-name-sentence-stop` | । / पूर्ण विराम | **व** and **ू** |
| `HI-W05-virama-namaste` | नमस्ते | **ं** |
| `HI-A1F01-name-delayed` | B → अरुण | **ण** |

None of those four teaches the glyph credited to it. They are lessons about the
head-line, the danda, the virama, and copying a name into a form.

**This is `HL-C383`'s bug one level up** — that one credited any lesson *body*
containing the glyph, this one any *headword* containing it — and it is logged
as `HL-C386`. Corrected, the series leaves **ङ, ञ, ण, व and श**.

**ण, व and श are real debt.** *vah*, *shukriyā*, *vinatī* and *shām* are all
taught vocabulary. ङ needs no Hindi lesson (`HL-C385`); ञ appears only inside
**ज्ञ**, which `HI-W05-conjuncts` teaches. **व and श are the next letters to
draw**, and SCR-15 should not be reconsidered until they are.

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

By the **stricter** check — a glyph is taught when the headword is a glyph
inventory, every token a base plus at most one combining mark — never-drawn
stands at **7**: ं ञ ण व श ू ो. Every earlier "undrawn goes N → M" figure in
this changelog was too optimistic for the same reason.
