## HL-C301 — Malayalam's chapters 1–5 are generated, and the split HL-C281 named is now costed

Malayalam has no hand-written book chapters left. All five moved from the
ledger's `handwritten` block to `targets` and are generated from lessons, so
`handwritten_parity.py --check malayalam` exits 0 and the corpus figure drops
from 69 hand-written chapters across 14 tracks to 64 across 13.

**The parity number was not the size of the job.** The script reported 33
blocks of prose at risk. Twenty-nine of those were the classification illusion
the Urdu agent found: prose that was already in the lessons, sitting under
`## Script` and `## Writing` headings, which classify as `script`/`writing`
rather than `pronunciation` and so never produce a `sounds` box to match the
`.tex`. The renderer emits them as `\subsection*`, and nothing was lost. Only
four blocks were genuinely missing, all `cognates` tables, and two of those
turned out to be already carried.

The real blocker was invisible to that script entirely: **a generated chapter
requires schema v2 for every lesson in it, and 29 of the 67 lessons behind
these five chapters were v1.** `book.ts` throws on the first one. Migrating
them is the tranche.

**HL-C281 is now larger and better evidenced, not smaller.** Declaring the
atoms those 29 lessons were always teaching took Malayalam's
`atomMeasurementBlindLessons` from **38 to 9** — and, in the same motion, its
`atomChapterSpikes` from **0 to 3**:

| chapter | new atoms | budget | what it teaches |
|---|---|---|---|
| 1 | 20 | 12 | namaskāram + an 8-lesson writing runway + nandi, athe, illa, śari |
| 2 | 22 | 12 | ten letters and eight words, in one chapter |
| 5 | 16 | 12 | three verbs, the `-unnu` present, and nine letters |

Not one atom was invented. HL-C281 predicted Chapter 1 would land at 15 if the
three closure letters were added; the true figure is 20 before they are added,
because the four v1 word lessons were contributing zero to a budget they were
in fact spending. **The split is now required by two independent measurements
rather than one**, and it extends to Chapter 2, which HL-C281 did not know
about.

The renumber HL-C281 costs out is unchanged and still wants its own PR: clear
the 46 cross-chapter prose references `tests/chapter-references.test.ts` pins,
then renumber, then split. What has changed is that the split now buys three
chapter-budget zeros as well as the last closure zero.

**Two things this tranche learned that generalise to the other 13 tracks.**

1. *Run the schema-v2 census before quoting a parity number.* `grep -L
   "schema_version: 2"` over the owning chapters' lessons is a better estimate
   of a retirement's size than the parity script, which measures a different
   thing. For Malayalam the parity number was 33 and the actual work was 29
   lesson migrations plus 15 block re-homings.
2. *A block counter will happily buy you a worse book.* Fourteen `## Sounds
   you'll need` blocks in these chapters were verbatim restatements of the
   `## The letters in this word` block directly above them, added in an earlier
   tranche to make the parity count come out. Generated, they print the same
   letter breakdown twice. Deleting them makes the parity number worse and the
   book better. Check for that pattern in any track a previous tranche has
   already touched.
