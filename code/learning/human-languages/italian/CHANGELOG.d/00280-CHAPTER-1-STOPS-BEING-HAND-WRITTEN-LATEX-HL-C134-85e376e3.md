## Chapter 1 stops being hand-written LaTeX (HL-C134)

`italian/book/chapters/ch01-greetings.tex` was the track's last hand-written
chapter. It was skipped by the generator, so nothing it said was reachable from
a lesson, and every lesson-level gate reported on it truthfully and
irrelevantly. It is now generated from the ten Chapter 1 lessons.

**What the flip needed.** `generate:books` refuses a schema-v1 lesson, and eight
of Chapter 1's ten were v1 — the only v1 lessons left in the track. They are
migrated here with **hand-authored knowledge atoms**, one per teaching section,
rather than the single per-lesson atom `migrate_schema_v2.py` assigns. That
script is honest about being an under-count, and an under-count would have made
the chapter look gentler than it is. The real figure is 20 atoms across 10
lessons: at most 3 in any one lesson, which is the per-lesson ramp budget, and
over the 12-atom chapter budget, which was previously invisible because the
chapter was measurement-blind. See the backlog entry for why that is progress
and not a regression.

Two section headings were renamed onto ones the renderer already classifies —
`## The phrase, assembled` and `## The greetings` both produced `unknown`
blocks, which schema v2 rejects. They are now `## The word, taken apart`
(`buongiorno` genuinely is taken apart, into *buon* + *giorno*) and
`## What you've built — the day of greetings`. No renderer feature was invented.

`IT-C01-practice` gained the warm-up block every lesson must open with, and is
now registered where `IT-C02-practice` already sits: a `consolidation`
extension node, `IT-EXT-006-CONSOLIDATION`, attached after `IT-PATH-006`. The
extensions ledger renumbered to keep it in path order.

**What the reader gains.** The hand-written chapter was nine sections of
exposition with no retrieval anywhere in it. The generated chapter is ten
lessons, each closing with its own *Your turn* and *Before you move on* box; a
chapter-opening statement of what the reader will be able to do; a grammar lens
on feminine agreement in `sera` and a register note in `grazie` that existed in
the lessons and had never reached the book.

**What was dropped, and why.** One thing: the `noctem` / `lactem` / `factum`
correspondence table in the `notte` section, comparing Latin, Italian, Spanish,
French and English. `IT-C01-notte` already replaced it, deliberately, with
*"Watch just one change today ... there is no list to memorize now."* Three
extra correspondences and two extra languages is exactly the load a pre-A1
lesson is not supposed to carry, so the table stays dropped rather than being
carried back in. The chapter's own opening paragraph is likewise gone, replaced
by the generated can-do opening built from `chapters.d/0001.json`.

`handwritten_parity.py --check italian` exits 0, and it did before this change
too — the prose was already carried. What it could not tell anyone is whether
the generated chapter teaches the same thing in the same order. It does.

