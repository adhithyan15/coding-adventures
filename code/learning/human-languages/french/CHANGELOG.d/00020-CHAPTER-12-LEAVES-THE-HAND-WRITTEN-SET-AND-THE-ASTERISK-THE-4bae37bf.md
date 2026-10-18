## Chapter 12 leaves the hand-written set, and the asterisk the book was eating

Chapter 12 (Family) was hand-written LaTeX over **two** schema-v1 lessons that
held **four headwords between them** — `le père, la mère` in one and
`le frère, la sœur` in the other. No word in the chapter owned a lesson, so no
word could be practised, reviewed or scheduled on its own.

Seven schema-v2 lessons replace them, one new thing each:

| lesson | the one new thing | atoms |
|---|---|---|
| `FR-C10-pere` | *le père*, and *pater* inside *paternal* | 2 |
| `FR-C10-mere` | *la mère*, and *māter* inside **matter** | 2 |
| `FR-C10-grimm` | **Grimm's law** — *p → f*, *t → th* | 1 |
| `FR-C10-frere` | *le frère*, and *friar* as a worn-down *frère* | 2 |
| `FR-C10-soeur` | *la sœur*, and why it broke the rhyme | 2 |
| `FR-C10-oe` | **œ**, one letter and not two | 1 |
| `FR-C10-practice` | the chapter payoff | 0 |

Ten atoms, under `maxNewAtomsPerChapter`'s 12, so this one does **not** split.

### The two things the `.tex` held that no lesson owned

**Grimm's law** lived in a `grammarlens` block. It is the single fact that turns
*père/father*, *mère/mother* and *frāter/brother* from three separate
memorisations into one, and it now has a lesson rather than a box.

**œ** is a *letter*, and it was a parenthesis inside a `sounds` block. It earns a
lesson on the corpus's own rule that a root must be cashed in at least three
times: **œ** returns in `l'œuf` and `l'œil` later in this book, and in *cœur*
everywhere outside it. The lesson says plainly which of those the book teaches
and which it does not, rather than promising four and delivering two.

### Sized four ways, because the gap said 2

`handwritten_parity.py` scored the chapter at **2** blocks. Counting the French
words the `.tex` teaches against the lessons that own them gives **4 headwords
against 2 lessons**; the honest denominator, `grep -l '^chapter: 12$'`, gives
**2**; and reading the tables finds three of them, all vocabulary rather than
paradigm — which is exactly why this chapter did not need splitting and
chapter 9 did.

### A defect only the printed page could show

The Grimm's law table lists reconstructed forms, and every reconstructed form is
written with a leading asterisk. The second row printed as *méh₂tēr* — **without
its asterisk** — because `\\` followed by `*` is LaTeX's *starred line break*,
so the row separator above swallowed it. The first row was fine; it follows
`\midrule`.

Nothing could catch this but looking. The `.tex` on disk contained the asterisk,
so `check:books`, the hash shards and every text assertion were green while the
printed page dropped the one mark that says a form is reconstructed. `book.ts`
now brace-protects a leading `*` in a table cell, and `book.test.ts` pins it
with a **two-row** fixture, because a one-row table cannot reproduce it.

Two further page-only defects, same method: a `[WRITE: …]` cue written without
the `YOU ` prefix printed its brackets verbatim (389 lessons write
`[YOU WRITE:`; this was the only one that did not), and two Guided Practice
sections ended their prose with a colon, so the renderer's own *"Say these
aloud:"* landed underneath as a stutter.

