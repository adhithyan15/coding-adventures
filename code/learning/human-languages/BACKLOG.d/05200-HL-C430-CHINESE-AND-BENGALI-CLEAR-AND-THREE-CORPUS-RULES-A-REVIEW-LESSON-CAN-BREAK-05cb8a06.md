## HL-C430-05cb8a06 — Chinese and Bengali clear, and three corpus rules a review lesson can break that a content lesson cannot

**Status: CLOSED (2026-09-23) — implemented.** The third tranche of HL-C426's
second-pass template, and the first where the gates caught defects that only a
*review* lesson can produce.

### What shipped

Six `review` lessons, no new atoms, no new headwords:

| track | lessons | |
|---|---|---|
| chinese | 3 | `chinese: reinforcement 16 -> 0` |
| bengali | 3 | `bengali: reinforcement 14 -> 0` |

Chinese needed **three** rather than two because six of its sixteen thin atoms
had **no later revisit at all** and wanted two passes each: the r=0 six sit in
chapters 1-6, so one lesson at sequence 680 gives them their first revisit and a
second at 1975 gives them the other. The late one is also the better lesson —
retrieval eight chapters downstream is a test rather than a rehearsal, and the
lesson says so in its own prose.

Eleven of twenty-three tracks now carry no pre-A1 reinforcement debt.

### THE PART WORTH KEEPING: three rules a review lesson trips and a word lesson does not

All three passed `validate` and were caught only by the full suite.

**1. `standalone-book` — "the course" is not a thing the reader holds.**
Twelve findings, every one mine:

```
chinese/lessons/ZH-C11-second-pass-eight-exchanges.md: the course
chinese/lessons/ZH-C11-second-pass-character-and-word.md: The course
chinese/lessons/ZH-C19-second-pass-from-cold.md: the whole course
```

A book volume ships standalone, so prose may not tell its reader they already
learned something that lives in another volume. **A review lesson reaches for
exactly that phrasing** — "the course has warned you", "the opening exchange of
the whole course" — because looking back across the whole book is its job. A
word lesson almost never does. Write **this book**, **these chapters**, or name
the thing.

**2. `glyph-coverage` — the Chinese book cannot render `·` (U+00B7).**
Four findings, all from one habit: using a middle dot to list characters
compactly (`女 · 子`, `你好 · 再见`). The same character renders in the Telugu,
Persian and Bengali books and was used there without complaint. **Per-book font
coverage is per book**, and a review lesson lists more items side by side than
anything else, so it meets the limit first.

**3. `script-closure` — a headword still needs a romanization.**
`BN-R30` opened with `headword: ধীরে` and no `romanization:`. Bengali's debt
ceiling for unromanised headwords is **zero**, so one lesson broke it. Easy to
forget on a review lesson, whose headword is often a word already taught and
therefore feels like it does not need re-glossing.

### And the `pathOrder` trap, caught by its own lesson

`BN-R08` was filed on `BN-PATH-004B` because that is where `BN-C04-practice`
sits — the **last of its six prerequisites by sequence**. Two of the other five
sit on `BN-PATH-002A` and `BN-PATH-003A`, which rank **0250** and **0310**
against 004B's **0110**:

```
bengali: BN-C02-practice must precede BN-R08-second-pass-four-exchanges
bengali: BN-C03-practice must precede BN-R08-second-pass-four-exchanges
```

The `lessons.d` entry written after HL-C428 says to pick the segment from where
the prerequisites sit, and it was followed **for one prerequisite instead of
all six**. Checking the latest of them, not the latest by sequence, is the rule.

### Method

The pre-flight this time was the per-track corpus test, read **before**
authoring rather than after — which is why gujarati and marathi were deliberately
left out of this tranche. Both pin reinforcement-window POSITIONS
("closes Gujarati doorway R4 at position 134"), and `measureContinuity` judges a
window in lesson INDEXES, so inserting a review lesson between an atom and its
revisit can push that revisit out of its window. Those two want their pins
checked as the first step, not the last.

Remaining: gujarati 13, russian 13, french 14, marathi 14, then tamil 73 and
arabic 78. Arabic still wants its own entry — 68 of its 78 have no later revisit
at all, which is a missing review layer rather than debt.
