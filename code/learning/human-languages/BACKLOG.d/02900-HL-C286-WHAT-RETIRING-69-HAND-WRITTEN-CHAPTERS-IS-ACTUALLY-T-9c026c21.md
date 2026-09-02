## HL-C286 — what retiring 69 hand-written chapters is actually teaching us, from the four small tracks

Italian, Portuguese, Persian and Arabic are now generated from their lessons.
HL-C283, HL-C284 and HL-C285 record what each track cost. This entry records the
three things that generalise, because they will decide how the remaining
chapters go.

### 1. A green parity check is not permission to ship

`handwritten_parity.py --check` answers exactly one question: *is a prose BLOCK
about to disappear?* It cannot answer *will the generated chapter teach the same
thing in the same order?* Four tracks demonstrated four different ways for those
answers to come apart.

  * **Italian and Portuguese both reported a gap of ZERO, and both were hiding
    the same thing** — a Latin-to-Romance sound-correspondence table, invisible
    because `tabular` is classified as layout rather than prose. They resolved
    it OPPOSITELY and both were right. Portuguese kept it (the lesson already
    carried it as markdown). Italian dropped it, because `IT-C01-notte` had
    already replaced it with *"watch just one change today"*, and three extra
    correspondences across two extra languages is not pre-A1 load. **Same
    measurement, same zero, two correct and contradictory answers.**

  * **Persian reported 16 orphaned blocks and the number pointed the wrong
    way.** Its instruction reads "carry the missing prose into the lessons
    first". Four of the sixteen — the `scriptstep` boxes — were the chapter's
    central DEFECT, not prose to rescue: between them they named nine letters in
    the first chapter of a book that teaches one. Carrying them faithfully would
    have been the wrong answer.

  * **Arabic's remaining gap of five is not missing prose at all.** The
    hand-written chapters boxed letter-shape descriptions as `sounds`; the
    lessons file the same sentences under `## The letters in this word`, which
    the renderer classifies as `script`. The lessons have it right. Flipping
    chapter 2 will re-file that prose, not lose it — so **the gap of five is not
    a blocker and nobody should type five paragraphs to close it.**

The instrument is worth keeping and its output is worth reading as a question,
never as an instruction.

### 2. A schema-v1 lesson contributes ZERO to every atom budget — not "a little"

This is the one that will otherwise get a good change reverted.

A schema-v1 lesson declares no knowledge atoms. Not few — **none**. So it
contributes exactly zero to the atom ramp, to the per-chapter budget, and to
every gentleness measure in the corpus. A chapter built entirely from v1 lessons
is not measured as gentle; it is **not measured at all**, and an unmeasured
chapter reports as a pass.

The consequence is that retiring a hand-written chapter *reliably makes the
numbers look worse*:

    Italian     chapters above 12 atoms   27 -> 28      atom-blind  409 -> 401
    Portuguese  chapters above 12 atoms   +1            atom-blind  -8

Both movements come from the same lessons. Italian chapter 1 has always taught
20 atoms against a budget of 12; it was over budget the entire time and the
report could not see it. **27 -> 28 does not mean "this made Italian steeper".
It means "this made Italian measurable, and it was already steep."**

Two corollaries worth stating:

  * `migrate_schema_v2.py` assigns exactly ONE atom per lesson and its own
    docstring calls that a deliberate under-count. Running it unmodified keeps
    the chapter under budget and keeps the report wrong. All four tracks here
    were migrated with hand-authored atoms, one per teaching section, for that
    reason.
  * Expect every remaining flip to arrive with a finding attached. **A flip that
    reports nothing new is the result to double-check.**

### 3. Three claims in this wave were wrong, and were caught by checking them

Recorded because the standard matters more than the individual errors.

  * *"These writing lessons render only in the answer key."* True of Hindi, and
    the phrase came from `handwritten_parity.py`'s own docstring. **False of
    Arabic**: grepping the compile inputs and the committed `book/` tree found
    them nowhere, and `appendix-answer-key.tex` is 31 lines and does not mention
    them. They rendered nowhere in the book at all — while the generated
    narration still scripted them for audio. So the listener met that alphabet
    twice and the reader never met it once.
  * *"Persian sat at 0 lessons above 3 new glyphs."* Read off a corpus-wide
    report line. Measured per track, the budget flagged one Persian lesson,
    `FA-C01-salam`, for the four glyphs of سلام against a budget of three. The
    corrected version makes the point better than the wrong number did: the
    budget policed a four-glyph headword and had nothing to say about four
    `scriptstep` boxes naming nine letters in the same chapter.
  * *"Portuguese chapter 1 introduces 19 atoms."* Hand-counted. Re-counted from
    the corpus: **18**.

None of the three changed a conclusion. All three were in prose that would have
outlived the PR. Check the number you are about to write down, especially when
it is the one that makes your argument.

### What Arabic still owes

Chapter 2 is the last hand-written chapter in these four tracks. It needs twelve
schema-v1 lessons migrated and several headings the renderer classifies as
`unknown` re-homed (`## The bowl family — a truth-table`, `## Its three jobs`,
`## Draw them`, `## The catch in "uh-oh"`, `## The dialogue`). Its parity gap of
five is the re-filing described above and is not work.

One renderer limitation was observed and left alone: markdown ordered lists
(`1.` `2.` `3.`) in a lesson body flatten into a run-on paragraph in the
generated `.tex`. This is corpus-wide and pre-existing — Arabic chapters 3+ and
every other generated track show it — so fixing it belongs in a renderer PR that
regenerates all 23 books, not in a track flip.
