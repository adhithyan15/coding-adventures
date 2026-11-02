---
category: Testing & coverage
---

# A memoised closure that is handed back as a fresh Set per query is still quadratic, so answer membership from the index instead

`human-language-data`'s "has a valid shared spine covering every registered
language" integration case calls `validateCurriculum` on the real corpus
(60,202 lessons). It had already been sped up once: the prerequisite-knowledge
closure was memoised as one bitset per lesson, which removed the quadratic
re-walk of every track's prerequisite chain. Even so, the case took 34-45 s
against Vitest's 30 s budget, even alone on a quiet machine.

The memo had made the closure cheap to *compute*, but not to *return*.
`prerequisiteKnowledge` peeled every set bit back into a new `Set<string>`
for each lesson. The caller then copied that Set twice more, as
`new Set([...known, ...introduces])` and as `new Set(known)` for the block
frontier. Late in a track a learner knows thousands of atoms, so each lesson
built three Sets of thousands of strings to answer a handful of `has`
questions. That is lessons x atoms work again, and the garbage collector
(12.7 s of a 46 s profile) was cleaning up after it. A second hot spot was
the relocation ledger: a filter over the whole corpus inside the
per-curriculum, per-spine-node loop (23 x 41 x 60k lesson visits).

The fix changes the shape of the answer, not the algorithm. The closure now
returns `{ has(atom) }`, which is one Map lookup (atom -> bit index) plus a bit
test against each prerequisite's memoised closure. The two copied Sets became
`known.has(a) || lessonIntroduces.has(a)` and
`known.has(a) || introducedInBlocks.has(a)`. The relocation filter became a
per-curriculum concept index, merged back into corpus order, because the
ledger is compared as JSON and its key order matters. The case went from a
timeout to 7.4 s. Equivalence was checked with a differential harness that
ran the old and new compiled validators on randomly mutated copies of the
real corpus and compared the Issue arrays with `isDeepStrictEqual`.

What to do differently:

- When a memo does not help as much as it should, profile what the memoised
  function *returns*. If it hands back a materialised collection, the
  consumers' copies can cost more than the memo saved. A
  `(garbage collector)` row near the top of a CPU profile is the tell.
- Return the narrowest interface the callers use. Here that was `has`. A
  `Set` already satisfies that interface, so the slow fallback path did not
  change.
- When you swap a filter for an index, check whether anything downstream
  depends on order (`JSON.stringify`, `Object.fromEntries` overwrite
  semantics, the order issues are emitted in). Keep positions so the merged
  result can be sorted back into the original order.
- Prove the rewrite with a differential run against the old build on mutated
  real data. A clean corpus produces zero issues, which proves almost nothing
  about error paths.
