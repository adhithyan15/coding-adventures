## 2026-09-01 — The opening five chapters are generated from lessons

Malayalam had five hand-written LaTeX chapters. A hand-written chapter is not
built from lessons, so every lesson-level gate reported on it by not reporting
on it at all: "0 lessons over 300 effective seconds" was true and said nothing
about Chapters 1–5, because nothing in those chapters was a lesson the gate
could see. Opening the book and finding an alphabet and several headwords
arriving together is what that silence looked like from the reader's side.

All five are now generated. **Malayalam has no hand-written book chapters left**,
and `handwritten_parity.py --check malayalam` exits 0 because there is nothing
handwritten to check.

- **Twenty-nine lessons were carried from schema v1 to schema v2.** This was
  the real blocker, and it is why the parity script's 33 blocks understated
  the job: a generated chapter requires schema v2 for *every* lesson in it,
  and 29 of the 67 lessons behind these five chapters were v1. They declared
  no knowledge atoms, no duration, no skills, no block boundaries — so they
  were invisible to the ramp, the budget, and the reinforcement windows alike.
- **Lessons the ramp cannot measure at all: 38 → 9** (87% → 97% measurable).
  Malayalam is now a version-2 track with no legacy lessons anywhere.
- **Fifteen prose blocks had no stable block type** and would have been
  rejected outright by the generator. Each was re-homed to a heading the
  renderer emits rather than being cut: two "Across the family" cousin tables
  moved into `The word, taken apart`; four "The phrase, assembled" and "The
  reply" sections became `The phrase, taken apart` and `The exchange`; three
  "The atoms" / "The engine" runs of production prompts became
  `Guided Practice`; three "Roots you now carry" recaps became
  `The roots, taken apart`. `ML-C02-practice` opened on a dialogue table with
  no warm-up and gained the one-line opening its four siblings already had.
- **Fourteen duplicate `Sounds you'll need` blocks were deleted.** Each was a
  verbatim restatement of the `The letters in this word` block directly above
  it, added to satisfy a block *counter*; generated, they would have printed
  the same letter breakdown twice on one page. One of them was also leaking
  raw LaTeX (`n\=\i`) into markdown. Deleting them raises the parity
  script's block count and lowers the reader's page count, and the reader wins.
- **Two cousin tables the `.tex` carried and the lessons did not were carried
  across.** `cognates` has no generated equivalent, so both folded into the
  etymology block, which is what they always were: the five-language "hello"
  table in `ML-C01-namaskaram` (romanisation only — the book's first lesson is
  listening-first and shows no Malayalam script at all), and the Bengali row
  and closing line of the farewell table in `ML-C04-poyi-varaam`.
- **One block was deliberately not carried.** Chapter 1's hand-written closing
  `culture` box previewed the farewell **പോയി വരാം** with its Tamil and
  Kannada cousins. `ML-C01-practice` replaced it with a promise for later
  during the gentle-ramp pass, and Chapter 4 now teaches that farewell one
  piece at a time. Re-adding it would forward-reference four untaught words in
  the chapter that is meant to be gentlest.
- **The chapters roughly doubled in depth without gaining a section.** All
  five keep exactly the section count they had (15, 20, 10, 8, 14); what they
  gained is the rest of each lesson — warm-up, letters, cousin web, grammar
  lens, practice and recall — which the hand-written versions had compressed
  away. The five hand-written sources were 1,101 lines of LaTeX; the
  generated five are 2,916, and they occupy the book's first 78 pages.

**What this cost, stated plainly.** Chapters over the twelve-atom budget went
**0 → 3**: Chapter 1 at 20, Chapter 2 at 22, Chapter 5 at 16. Not one new atom
was invented to get there — these are the atoms the v1 lessons were always
teaching and never declaring, and the budget can only see them now that they
are declared. Chapter 1 genuinely teaches a greeting, its eight-step writing
runway, and four more words; Chapter 2 genuinely teaches ten letters and eight
words. The measurement is right and the chapters are too long. The fix is the
split that `HL-C281` already specifies, now extended to Chapter 2 and costed in
this tranche's backlog entry; compressing them back down to hit a number would
undo the thing this change was for.

Held: script-closure violations **1** (unchanged — `ML-C01-practice`, still
waiting on that same split), characters shown but never taught **0**, headwords
without a romanization **0**, lessons over three new atoms **1** (unchanged,
`ML-C08-dayavayi`), lessons over five minutes **0**. The book compiles under
XeLaTeX at 459 pages with zero overfull boxes and zero missing characters, and
the split o-sign **ൊ** and pre-posed **േ**/**െ** were confirmed by rasterizing
the pages rather than by counting warnings.

