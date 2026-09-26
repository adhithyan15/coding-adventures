---
category: Repo policy / workflow reminders
---

# A curriculum.d shard inserted mid-list must renumber every later shard; re-sharding numbers them sequentially

**What went wrong (HL-C443 loop, French A1).** To give chapter 6 its own path
segment, I added `curriculum.d/path/0145-FR-PATH-015-COUNT.json` and
`extensions/0025-...` between two existing shards. Every other check passed,
but `curriculum-shards.test.ts` failed. The test requires the shards to be
exactly what re-sharding the rebuilt ledger would produce, and re-sharding
numbers the files 0010, 0020, 0030 ... in order. There are no gaps and no
in-between ranks.

**Fix.** Keep the new file in the right place in the sort order, then renumber
every shard in that directory as `(index+1)*10`. Use a two-phase `git mv` (to
`.tmp`, then to the final name) so renames never collide. Appending at the end,
as the vocabulary generator does, needs no renumbering.

**Do differently.** After adding any `curriculum.d` shard, run
`npx vitest run tests/curriculum-shards.test.ts` before the full suite.
