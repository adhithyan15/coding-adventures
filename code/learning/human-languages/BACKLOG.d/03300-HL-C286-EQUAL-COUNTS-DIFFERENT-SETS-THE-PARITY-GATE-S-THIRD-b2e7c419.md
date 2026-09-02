## HL-C286 — equal counts, different sets: the parity gate's third failure mode

Two failure modes of `handwritten_parity.py` are already recorded: HL-C134 (an
allowlist of what the TARGET can emit undercounts what the SOURCE loses) and the
Arabic entry (an allowlist of HEADINGS invents debt that is not there). Kannada
chapter 1 found a third, and it is the quiet one.

**The gate compares counts per environment. Counts can match while the SETS do
not.**

Chapter 1's `.tex` holds two `culture` blocks. Its lessons hold two
"Why it's said this way" sections. The gate subtracts 2 from 2, reports nothing
at risk for that environment, and is wrong twice over:

* the `.tex` block on the "go and come back" farewell had **no lesson at all**;
* `KA-C01-dhanyavada`'s culture block on *dhanyavāda* being the formal,
  written thank-you is **not in the `.tex**` — it is lesson-only prose.

One block would have been dropped and a different one gained, and every count in
the report would have stayed at zero. No amount of staring at the gate's output
finds this; it is invisible by construction.

### What found it

Matching blocks by CONTENT rather than by count — Jaccard similarity over
lowercased word tokens with LaTeX markup stripped. That tolerance matters: the
prose was rewrapped and re-punctuated on its way into LaTeX, which is exactly
why the gate's own docstring rejects a text diff. A similarity floor of 0.35
separated real matches (0.44–0.86) from orphans (0.11–0.33) cleanly, with no
ambiguous middle on this corpus.

Run across the three remaining chapters it produced a list that overlapped the
gate's by **half**:

    parity said            content matching said
    ch1 sounds x1          false  - the script block already covers it
    ch1 cognates x3        1      - two were already authored as lessons
    (not reported)         ch1 culture x1
    ch2 cognates x1        1
    (not reported)         ch2 cousinweb clause x1
    ch4 cognates x1        1

Six blocks either way. Three of the six the gate named were not at risk, and two
blocks it never mentioned were.

### The second half of the lesson: matching per chapter over-reports too

The chapter-1 `culture` orphan was scored against chapter 1's lessons only,
because that is the chapter being retired. Corpus-wide it was not orphaned at
all: `KA-C04-hoogi-baruttene`, three chapters later, already teaches every fact
in it. Carrying it across on the strength of the per-chapter score put the same
idea in three boxes on one spread — etymology, culture, and cognate table — and
**only compiling the book and looking at the page showed it**. Measured
afterwards at 64% token overlap with zero unique facts, and removed.

So the instrument wants both halves: match by content, and match against the
WHOLE corpus, not the chapter under the knife. A block with no owner in its own
chapter may be perfectly well owned somewhere the reader will actually reach.

### Standing advice for the remaining 28

Neither number is trustworthy alone. Size a chapter three ways before planning
it, and expect them to disagree:

1. `handwritten_parity.py --check <track>` — cheap, and wrong in both
   directions.
2. Words the `.tex` teaches vs lessons that own them — catches the German
   failure, where teaching hides in prose inside a surviving box.
3. `grep -l '^chapter: N$' lessons/*.md` — catches separately-staged writing
   lessons that appear in no `.tex` at all.

And then compile it and look at the pages. Every defect in this tranche that
mattered to a reader — a duplicated syllable box, a triplicated farewell, a
four-column table the narrator refused — was invisible to all three.
