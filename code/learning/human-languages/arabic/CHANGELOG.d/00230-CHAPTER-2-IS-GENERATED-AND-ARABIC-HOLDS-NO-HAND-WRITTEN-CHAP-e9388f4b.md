## Chapter 2 is generated, and Arabic holds no hand-written chapter (HL-C286)

`arabic/book/chapters/ch02-introductions.tex` is now generated from its thirteen
lessons. That was the last `handwritten` entry Arabic owned, so **the track is
fully generated**: `handwritten_parity.py --check arabic` now answers *"already
retired, nothing handwritten remains"* instead of blocking.

**The migration was the work; the flip was downstream of it.** Twelve of the
chapter's lessons were still **schema v1**, which declares no knowledge atoms at
all, and `book.ts` refuses to generate a chapter from a v1 lesson. So the flip
could not be attempted until every one of the twelve carried a v2 frontmatter, a
`hl-knowledge` directive on every body block, and a heading that classifies to a
real block type.

**Twelve headings had no stable block type.** `classifyBlock` sorts a `## `
heading into a block type by prefix, and schema v2 rejects anything that lands on
`unknown` — so twelve sections across six lessons were renamed onto conventions
the corpus already uses, by PREFIXING rather than rewriting so the prose bytes
are untouched:

- `The phrase, assembled` → `You'll want to know — The phrase, assembled` (×2)
- `The bowl family…`, `Draw them`, `Where you have seen them — أنت`,
  `ي (yāʾ) — two dots below`, `Assemble — first اسم, then اسمي`,
  `The ḥarakāt — three short-vowel marks`, `The catch in "uh-oh"` → `Script — …`
- `Its three jobs`, `The payoff — أنتَ vs أنتِ` → `You'll want to know — …`
- `The dialogue` → `The exchange`, which is the corpus's own name for a
  recap table and classifies as `input`.

**One block of new prose, and only one.** Schema v2 requires the first body block
to be a warm-up, and `AR-C02-practice` opened straight onto its dialogue table —
its opening lived in the hand-written LaTeX, not in the lesson. A warm-up was
written for it. Everything else in this chapter is prose that already existed.

**The reported parity gap of 5 was an environment mismatch, not missing work.**
`handwritten_parity.py` recognises a `sounds` block only from the heading
*"Sounds you'll need"*, while `parse.ts` maps *"The letters in this word"* — the
heading these five sections actually use — to `script`. The five `\begin{sounds}`
environments in the hand-written `.tex` were those five sections all along. The
generated chapter carries every one of them, and `cousinweb` (5) and
`grammarlens` (3) come across at identical counts. Nothing was lost; the chapter
grew from 222 lines to 512 because the generator publishes the guided-practice
and recall blocks the hand-written version had dropped.

**Curriculum placement.** `AR-C02-practice` was in no path segment, which schema
v2 will not allow for a lesson that declares a spine node. It joins `AR-PATH-007`
and a new `AR-EXT-007-CONSOLIDATION` extension, matching how `AR-C01-practice`
and `AR-C03-practice` are already placed.

**Counters re-measured, not derived.** `handwritten.d` holds **37** entries after
this change, counted from the directory. Arabic's lesson-content budget moves
90 → 102 — the twelve migrated lessons become visible to it — while idioms,
senses and culture claims are unchanged at 2 / 3 / 14, because the migration
declared atoms and renamed headings without authoring vocabulary. The
`grouped-shards` handwritten ratchet is deliberately left alone: it is a ceiling,
and its in-file directive says a retirement PR should simply pass under it.

**Verified.** The whole Arabic book compiles under XeLaTeX with zero overfull
boxes.

