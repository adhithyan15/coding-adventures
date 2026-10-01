---
category: Testing & coverage
---

# A track's targeted test set must include integration.test.ts, which pins some books' full chapter lists

**What happened:** the Russian A2 spine chapters (136-139) were validated with a
targeted set: the Russian corpus, level-gate, chapters, chapter-references,
banned-words, membership-shards, title-stubs and doc-shard tests. All passed.
`human-language-data/tests/integration.test.ts` was not in that set, and its
"preserves every existing LaTeX book" test pins the exact list of book chapters
for a few tracks: `Array.from({ length: 135 }, ...)` for Russian, and an
explicit 1-143 list for Urdu. Adding chapters 136-139 broke the pin, and the
PR would have gone red in CI. A full local run caught it after the push.

**The fix:** each commit that adds chapters to a pinned track extends that
track's list in `integration.test.ts`, with a `// old -> new:` history comment
like the ones already there.

**What to do differently:**

1. When a commit adds chapters to a track, grep
   `tests/integration.test.ts` for `book.language === "<track>"`. If the
   track is pinned, extend the pin in the same commit.
2. Before the first push of a new track's tranche series, run the whole
   `human-language-data` suite once (`npx vitest run`), not only the targeted
   set. The targeted set is fine for later tranches of the same track once
   the full run has shown which pins that track touches.
