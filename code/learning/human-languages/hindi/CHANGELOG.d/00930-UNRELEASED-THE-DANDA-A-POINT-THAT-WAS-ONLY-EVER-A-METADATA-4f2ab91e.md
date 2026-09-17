## Unreleased — the danda, a point that was only ever a metadata gap

**`HI-A1-S-04` closes with no new teaching and no new prose.**

| metric | before → after |
|---|---|
| exam-point coverage | 236/282 → **237/282 (84%)** |
| atoms taught | 533 → 534 |
| measurable lessons | unchanged (478) |
| measurement-blind lessons | unchanged (17) |
| `forward-language` | unchanged (22) |
| `atomsNeverRevisited` | unchanged (33) |
| `durationViolations` | unchanged (0) |

### The note called its own cause exactly

> `HI-W06-name-sentence-stop` teaches the danda and declares an EMPTY
> `introduces` list, so it contributes no atom and cannot be probed. **The gap is
> in the lesson's knowledge contract, not in the teaching.**

That is precisely right, and rare enough to be worth recording. The lesson shows
**।**, says it is not part of the **है** before it, and grades a written activity
on producing it — and declared nothing.

It now introduces `HI-SCRIPT-DANDA-01`. `HI-W06-two-sentence-no-model` pays it
off: two sentences, **no model on the page**, so the reader has to produce the
mark unaided.

### A regression that every gate passed

Wiring that payoff by appending the atom to the lesson's **first** `hl-knowledge`
comment cost a measurement-blind regression. A writing lesson with an explicit
zero-new contract stays measurable only while its blocks carrying an
`hl-writing-stage` directive assess **exactly** its practised set — and the first
block in the file is the **Warm-up**, which carries no directive. The practised
set grew, the staged set did not, the contract broke, and the lesson fell to
measurement-blind.

`validate` reported zero errors. All twelve `check:*` gates passed. **The only
symptom was two lines in the snapshot diff** — `atomMeasurableLessons` down one,
`atomMeasurementBlindLessons` up one. Fixed by putting the atom in the
`connected-composition` block where it belongs, and recorded in `lessons.d`.

### Two backlog corrections found while ranking

- **`HL-C382`** proposed grouping कल, परसों, पहले and बाद to close `HI-A1-T-09`.
  **`HI-A1-T-09` closed at chapter 98** on पहले and के बाद alone. The entry's
  stated payoff is gone; what remains is that कल and परसों each carry two senses
  and the corpus teaches one apiece — real, but closing no point.
- **`HL-C387`** is new, for the Devanagari digits. Three things block
  `HI-A1-NUM-05` and the third is the one that stopped the lesson being written
  now: the corpus contains **no digit anywhere**, `devanagari.json` has **no
  digit data** (the same gap as `HL-C385`), and **there is nowhere to pay the
  atom off**, because no lesson ever writes a number in digits. Wiring it into
  `HI-C72-rupee`, which teaches the word **रुपया** and never shows a price, would
  be the overstatement this campaign keeps correcting. The entry asks for two
  lessons — the shapes, then a sign that uses them — and says not to write the
  first without the second.
