## Chapter 1 stops being hand-written LaTeX (HL-C134)

`portuguese/book/chapters/ch01-greetings.tex` was the track's last hand-written
chapter. The generator skipped it, so nothing it said was reachable from a
lesson and every lesson-level gate reported on it truthfully and irrelevantly.
It is now generated from the ten Chapter 1 lessons.

**Nothing was lost.** Every environment in the hand-written chapter has a
counterpart in the generated one, the five-language `noctem`/`factum`
correspondence table included — `PT-C01-noite` already carried it as a markdown
table, so it renders straight through. The generated chapter additionally
carries a `sounds` block for `obrigado` that lived in the lesson and had never
reached the book.

**What the flip needed.** `generate:books` refuses a schema-v1 lesson, and eight
of Chapter 1's ten were v1 — the only v1 lessons left in the track. They are
migrated with **hand-authored knowledge atoms**, one per teaching section,
rather than the single per-lesson atom `migrate_schema_v2.py` assigns; that
script is explicit about being an under-count, and an under-count would make
this chapter look gentler than it is. The real figure is **18 atoms across 10
lessons** — counted from the corpus, after an earlier hand-count of 19 was
checked and found wrong — at most 3 in any one lesson, which is the per-lesson
ramp budget, and over the 12-atom chapter budget, which was previously
invisible because the chapter was measurement-blind rather than gentle.

Two headings were renamed onto ones the renderer already classifies, because
`## The phrase, assembled` and `## The greetings` both produced `unknown`
blocks and schema v2 rejects those. They are now `## The word, taken apart`
(`bom dia` genuinely is taken apart, into *bom* + *dia*) and `## What you've
built — the day of greetings`. No renderer feature was invented.
`PT-C01-practice` gained the warm-up block every lesson must open with, and now
sits in a `consolidation` extension node, `PT-EXT-007-CONSOLIDATION`, attached
after `PT-PATH-007`.

**What the reader gains.** The hand-written chapter was nine sections of
exposition with no retrieval anywhere in it. The generated chapter is ten
lessons, each closing with its own *Your turn* and *Before you move on* box,
opening with a stated can-do.

**The `noite` table is now two spoken lines, and that was not optional.** As a
full-width `tabular` in a `center` block it fit; generated into the narrower
`cousinweb` box as a five-column `tabularx`, the header *Portuguese* overflowed
by 2.78pt. This was first written off here as a sub-millimetre cosmetic
difference — wrongly. `core/latex-warning-baseline.json` pins Portuguese at
**zero** overfull boxes and the CI books gate failed on it, which is the gate
doing its job: the same table was also being **refused by the narrator** for
being too wide, so one structure was failing two different consumers.

All ten forms survive as a list, with the two *-it-* languages now adjacent so
the shared change reads at a glance:

- Latin *noctem* → Portuguese **noite**, French *nuit*; Spanish *noche*, Italian *notte*
- Latin *factum* → Portuguese **feito**, French *fait*; Spanish *hecho*, Italian *fatto*

Overfull boxes: 0. Narration refusals: 58 → 57, because there is no longer a
table to refuse.

