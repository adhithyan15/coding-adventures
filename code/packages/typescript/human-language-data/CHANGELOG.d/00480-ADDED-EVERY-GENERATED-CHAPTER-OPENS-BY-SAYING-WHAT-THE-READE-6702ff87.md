### Added — every generated chapter opens by saying what the reader will be able to do (HL-C49)

- **Russian chapter 3 is the one generated chapter with no opening**, because it has
  no HL05 capability entry at all. A chapter with no `canDo` gets no opening rather
  than an invented one; the gap is capability debt the gap report already counts, and
  the test names it so it shrinks visibly instead of hiding behind a number.
- **288 of 407 chapters opened on a bare title** — `\chapter{}`, `\label{}`, straight
  into the first lesson section. Nothing told the reader why they were there. All
  **302 generated chapters** now carry a short opening, and **all 302 had the data
  already**: every one has a `canDo` in its HL05 capability ledger.
- **Derived, never authored.** `book.ts` composes the opening from `canDo` and
  `payoff.summary`. 302 hand-written intros would be 302 places to drift from the
  lessons they describe, and the generated file says at the top that editing it is
  pointless. `canDo` is quoted verbatim, so the book and the ledger cannot disagree
  about the same sentence.
- **It must stand alone in English**, per HL09 §8 — the book is a standalone artifact
  and English is its only requirement. Naming a *source* language is not a violation
  and is the point of the book ("negro inherited from Latin", "trace *hermano* through
  *germānus*"); naming another **track of this course** is, because it dangles for a
  reader holding one PDF. One real violation was found and fixed at source: Telugu
  ch11's payoff said "the borrowed blue every language in this course now shares".
- The blurb that used to sit here explained how the chapter was *produced*. Removing
  it was right; leaving nothing was not.

