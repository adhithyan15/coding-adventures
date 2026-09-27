## HL-C194 — the ten Kannada digits are the last untaught glyphs

Closing the eight letters ಉ ಊ ಝ ಥ ಈ ೃ ಃ ಞ took Kannada's never-taught glyph
count from **18 to 10**, and the ten that remain are exactly the digits:

> ೦ ೧ ೨ ೩ ೪ ೫ ೬ ೭ ೮ ೯

They are not shown in passing. **KA-C07-numbers-1-5** prints them in a table and
then drills them — `[YOU SAY: read the digits — ೧ ೨ ೩ ೪ ೫]` — and
**KA-C07-numbers-6-10** does the same for the rest plus **೧೦**. That is a
load-bearing decode of ten shapes no lesson has taught, which is the exact
failure the gloss-first ramp exists to prevent.

**Why this tranche did not fix it.** Closure is measured in reading order, so a
digit lesson only helps the lessons that follow it, and both consumers sit in
chapter 7. Ten digit lessons would therefore all have to land in chapters 6–7 —
either bunched at the end of chapter 6, or wedged between the two number lessons.
Both bunch ten writing lessons into one stretch, which is the batching the
interleaving rule forbids, and neither has glossed vocabulary between them to
space the glyphs out. The eight letters closed here had natural homes across
chapters 21–47 with word lessons already sitting between them; the digits do not.

**What the fix probably looks like.** The digits differ from letters in a way
that should be exploited rather than fought: the learner already owns the
*meaning* of every one of them, because they can read 1–9 in Arabic numerals.
There is no gloss-first debt to pay, only a shape to attach to a known concept.
That suggests pairing each digit with the number word it belongs to, one per
lesson, so chapter 7 becomes ten short word-plus-digit lessons instead of two
lessons carrying five digits each — which also gives **ondu … hattu** the
one-headword-per-lesson treatment the rest of the track gets and chapter 7
currently does not.

Note that splitting chapter 7 this way ripples into `language-ladder`, which
hardcodes chapter and lesson counts, and into `chapters.d/0007.json`, whose
payoff currently points at **KA-C07-numbers-6-10**.

The remaining Kannada closure debt after that would be the 30 violations that
are not about untaught letters at all, but about **ordering**: chapters 1–17
show letters that the ಚ, ಪ and virama script lessons do not teach until chapters
18–20. That is a resequencing problem, and a separate one.
