## Unreleased — chapters 3 and 5 are generated, and stop being hand-written LaTeX

`ch03-responding.tex` joins `ch05-first-verbs.tex` in the generator. Both were
`handwritten` ledger entries; Kannada owned five and now owns **three** —
chapters 1, 2 and 4, each of which still holds `cognates` prose no lesson has.

Eleven `KA-C0[35]-*` lessons moved from schema v1 to v2 (v1 declares no
knowledge atoms, and `book.ts` refuses to generate from a v1 lesson), gaining 19
atoms between them. `KA-C03-practice` sat in no curriculum path segment, which
v2 forbids for a lesson declaring a spine node; it now sits in a new
`KA-PATH-008-RECAP` under `SPINE-CHECK-WELLBEING`, placed after
`KA-PATH-008` so it still follows `KA-C03-paravaagilla`, with a matching
`KA-EXT-007-CONSOLIDATION`.

### Two defects that only the rendered page showed

**The same syllable breakdown printed twice, under the same title.** Fourteen
chapter-1-to-5 lessons carry both a `## The letters in this word` block and a
`## Sounds you'll need` block. The hand-written LaTeX printed only one of them.
Generated, both print — and because Kannada's preamble titles the `sounds` box
*"The letters in this word"*, the reader met that heading twice on one page with
near-identical text under each. No already-generated Kannada chapter carries
both blocks (0 of 229), so the duplicate was removed from the four lessons in
these two chapters — but only after proving mechanically that the removed block
was a strict subset: every non-ASCII character and every token of it already
appears in the block that stays.

**Four lessons were not NFC-normalized.** The proof above failed at first on
`hē` not equalling `hē` — one precomposed U+0113, one `e` plus a combining
macron U+0304. Seven of Kannada's 268 lessons carry decomposed romanization and
all seven are in the hand-written chapters. The four in chapters 3 and 5 are now
NFC; the remaining three (chapters 2 and 4) go with their own chapters.

### Nothing was dropped, and that was measured rather than asserted

`handwritten_parity.py` reported both chapters clean. That agreed with the
artifacts, but its block-gap number is not trustworthy alone in either
direction — it counts LaTeX environments, so a word taught in a paragraph inside
a surviving box costs zero blocks while costing a whole lesson. So both chapters
were sized a second way, by counting the words the `.tex` teaches against the
lessons that own them, and each flip was gated on token loss against the
pre-migration tree:

| chapter | Kannada tokens | lost | tokens owned by no lesson |
|---|---|---|---|
| 3 | 28 → 33 | **0** | 0 |
| 5 | 22 → 34 | **0** | 0 |

`cousinweb` goes 4 → 6 and 3 → 5, `grammarlens` 2 → 3 and 4 → 4, `sounds` 3 → 3
and 1 → 1. The chapters grow because the generator publishes the warm-up and
wrap-up-recall blocks the hand-written versions dropped.

One genuine discrepancy surfaced: the hand-written book romanised Tamil
எப்படி as *eppaṭi*, the lessons as *eppaḍi*. The book's form is restored, in
three places — a deliberate three-character change the census reports exactly.

### A ramp number moved, and it is coverage arriving rather than a regression

Chapter 3 became the track's first `atomChapterSpikes` entry (0 → 1), and
`atomsNeverRevisited` went 9 → 11. Nothing got steeper: the chapter was schema
v1, which declares no atoms at all, so it was invisible to every ramp
measurement until now. Fifteen atoms across eight lessons is **1.9 per lesson**,
under the corpus mean of 2.31, and the chapter introduces five new words across
six content lessons — one new word per lesson. It trips
`maxNewAtomsPerChapter: 12` because it is long, not because any step is large,
and `maxNewAtomsPerLesson: 3` — the budget that defines gentle — is met exactly.
The atoms were deliberately NOT merged to get under the number; the honest
remedy is splitting a chapter that already spans two spine nodes, which is
recorded in `BACKLOG.d` rather than done inside a retirement PR.

Counted against the merged tree after merging it, never derived: `handwritten.d`
holds **35** entries on `origin/main` (sibling PRs retired a French and a German
chapter while this was in flight) and **33** on this branch.

Verified: human-language-data 124 test files / 1730 tests, all eleven `check:*`
gates, language-ladder 39 files / 442 tests, and the whole Kannada book compiled
under XeLaTeX — 421 pages, zero overfull and zero underfull boxes — with the
changed pages read on the page rather than in the source.

