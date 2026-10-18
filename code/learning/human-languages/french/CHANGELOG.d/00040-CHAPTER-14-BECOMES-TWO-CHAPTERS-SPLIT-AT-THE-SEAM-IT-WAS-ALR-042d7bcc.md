## Chapter 14 becomes two chapters, split at the seam it was already about

Chapter 14 (Numbers Eleven to Twenty) was hand-written LaTeX over **two**
schema-v1 lessons that owned **ten numbers** between them, plus the rules for how
those numbers are built.

Ten numbers plus their formation cannot fit `maxNewAtomsPerChapter` at one atom
per word. This is the second French chapter split, and the second one where the
budget was not the interesting part.

### The split point is the chapter's own subject

The chapter exists to show that **at seventeen, French changes its mind**. Up to
*seize* the numbers are inherited from Latin already fused; from *dix-sept* they
are built in the open out of *dix* and a digit. That is the seam, and it is where
the chapter divides:

| chapter | what it holds | atoms |
|---|---|---|
| **14 — The Welded Teens** | *onze* to *seize*, the `-ze` that is *decem*, digit-first order | 8 |
| **15 — Where French Changes Its Mind** | *dix-sept* to *vingt*, the seam, *vīgintī* | 6 |

Sixteen lessons replace two, and every French chapter after the split renumbered
by **+1** (old 15–35 → 16–36). Lesson ids do not move: all sixteen are
`FR-C12-*`, including the seven that land in chapter 15.

### What one lesson per number made room for

The old lesson listed six words in a table and moved on. Given a lesson each,
several facts fit that had nowhere to live before:

- ***quinze* needs English to be legible.** *cinq* and *quinze* do not look
  related, because *cinq* travelled further from *quīnque* than *quinze* did.
  **quintet** stands between them, and the lesson says so.
- **The seam is two changes, not one.** Everyone notices *fused → built*. Almost
  nobody notices that the halves **swap order**: *seize* is six-ten, *dix-sept*
  is ten-seven. The seam now has its own lesson because it holds two facts.
- **Latin counted 18 and 19 downward** — *duodēvīgintī*, "two from twenty" — and
  every daughter language abandoned it. The plain *ten-eight* is a survivor, not
  a simplification.
- ***neuf* is two words** — nine from *novem*, new from *novus* — worn into the
  same four letters.
- ***vingt* is a beginning.** It explains *quatre-vingts* before the reader ever
  meets it, which is a better place to learn it than at eighty.

### The renumber's stale shard, and how it was found

A renumber leaves **stale book-hash shards**, and `check:books` says only
*"missing, stale, or malformed"* without naming one. `check:shards` named it —
`book-generation targets identity set differs: missing [french/0019]` — and a
by-hand diff of the hash-shard set against the ledger target set confirmed
exactly one: `generated-book-hashes/french.d/0019.json` still described
`ch19-head-and-hand.tex`, which is now chapter 20. The generator writes shards
but does not prune them, so a chapter number vacated by a renumber keeps its old
hash file until someone deletes it.

### Two false sight-cues, found by reading the rendered page

Chapter 14 first printed *"Hands-free start: **first 1** of 9 lessons"* — meaning
the second lesson broke the run. The modality classifier had marked `douze`
`sight`, and its reason was one phrase: *"this one lets you **see the** join."*
The same thing had happened in the seam lesson, on *"**Look at** which half comes
first."* Both were metaphors about hearing, both fired a literal page-pointing
cue, and both lessons are now genuinely hands-free — chapters 14 and 15 print
*"all 9 lessons"* and *"all 7 lessons."*

Nothing but the rendered page shows this. Every gate was green with a
voice-first chapter that told a listener to look at something.

