## Unreleased — four glyphs, and the consonant series closes for real

**`HI-A1-SCR-15` closes on the second attempt.** The first attempt is the more
useful record: it was probed, and the probe was **withdrawn before merge**
because the helper that said the series was complete was wrong (`HL-C386`).

| metric | before → after |
|---|---|
| exam-point coverage | 234/282 → **235/282 (83%)** |
| atoms taught | 526 → 530 |
| measurable lessons | 471 → 475 |
| writing-practice lessons | 117 → 121 |
| script-closure violations | 21 → 20 |
| `forward-language` | unchanged (22) |
| `atomsNeverRevisited` | unchanged (33) |
| `durationViolations` | unchanged (0) |

### Four glyphs

| lesson | glyph | chapter | pen lifts | why it waited |
|---|---|---|---|---|
| `HI-S144-vowel-sign-vocalic-r` | ◌ृ | 7 | copy-only | **it did not** — see below |
| `HI-S145-letter-sha` | श | 32 | 2 | first used in the first chapter |
| `HI-S146-letter-va` | व | 35 | 2 | first used in the first chapter |
| `HI-S147-letter-na-retroflex` | ण | 36 | 2 | first used in the second chapter |

**व has 39 headwords and श has 29** — by usage, the two largest script debts in
the corpus, ahead of ट (22) and ठ (13). Both were credited as taught by the old
helper and neither had ever been a lesson subject.

### The one that arrives on time

**`HI-S144` is the first glyph in this campaign to be drawn before the word that
needs it.** The sign sits at chapter 7 sequence 507; its carrier **क** is drawn
at 506, and **कृपया** — its only headword in the whole corpus — arrives at 510.

`HL-C384` listed the vocalic-ṛ sign among eight glyphs that *can never arrive on
time*, citing a chapter-1 first use. **That first use is a Sanskrit root quoted
in an etymology note** — √कृ, संस्कृत, कीदृश — not a Hindi word the reader is
asked to read. The same "counted from raw text rather than from what a lesson
teaches" error as `HI-C16-mahine`. `HL-C384` is corrected.

The lesson teaches the thing that actually trips a reader: **the mark is written
as one vowel and said as another.** *ṛ* on the page, *ri* in the mouth.

### Each of the other three completes a row

| lesson | row completed |
|---|---|
| श | the sibilants — स, ष, श |
| व | the semivowels — य, र, ल, व |
| ण | the retroflex varga — ट, ठ, ड, ढ, ण |

**Every varga is now complete.** Two glyphs remain and neither is debt: **ञ**
appears three times in the corpus and never standalone, always inside the
conjunct **ज्ञ** that `HI-W05-conjuncts` teaches as one of its three special
shapes; **ङ** appears zero times and is absent from `devanagari.json` entirely
(`HL-C385`).

If that reasoning is rejected, the probe to remove is SCR-15's — **and it has
been removed once already.**

### A forward reference caught before commit

`HI-S147` first illustrated itself with **प्रणाम**, which is taught 67 lessons
later. `forward-language` went 22 → 23. Cut, and the lesson now names the single
place the reader has actually met ण — **पूर्ण विराम**, the book's own term for
the full stop. One sighting in the whole book is a fair measure of how rare the
letter is, and the lesson says so rather than reaching forward for a better
example.

### Payoffs

| atom | payoff |
|---|---|
| `HI-SCRIPT-RECOG-144` | `HI-C08-kripaya` — **कृपया** |
| `HI-SCRIPT-RECOG-145` | `HI-C33-shubh-dopahar` — **शुभ दोपहर** |
| `HI-SCRIPT-RECOG-146` | `HI-C36-darvaaza` — **दरवाज़ा** |
| `HI-SCRIPT-RECOG-147` | `HI-C50-salutation` — **प्रणाम** |

None needed new prose, so no duration moved.

### Remaining Devanagari debt, by the corrected rule

**Four:** ं, ञ, ◌ू and ◌ो. The anusvāra and the two mātrās are real and are what
`HI-A1-SCR-14` actually needs.
