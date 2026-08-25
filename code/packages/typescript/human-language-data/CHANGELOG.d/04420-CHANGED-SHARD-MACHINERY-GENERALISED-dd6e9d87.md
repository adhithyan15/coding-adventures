### Changed - shard machinery generalised

- `ShardPlan` gains an optional `ordinalOf`, so a ledger that carries its own
  ordering number uses it. Appending chapter 346 writes one new file and renames
  nothing, where the index-derived stride would have renumbered every shard
  after the insertion point — a mass rename, and therefore the mass merge
  conflict this work exists to remove.
- `ShardPlan` gains an optional `idOf` (absent when identity *is* the number)
  and a **required** `monolith` disposition, `"generated" | "removed"`. That
  decision is what makes a migration worth doing, so it is not defaulted.
- `shardContents` now refuses two elements that collide on a *filename*, not
  only on an id. With no id, a duplicate chapter number silently overwrote a
  chapter, and `--check` would have agreed with itself about the truncated set
  ever after.
- `loadTrackChapters` reads either form. Its presence test is
  `existsSync(path) || isSharded(path)`; the second half is load-bearing,
  because the function treats a missing `chapters.json` as "not yet authored".
- `code/scripts/verify-human-languages.sh` gains `check:shards`. CI has run it
  since the spine landed, but the script people run before pushing had not —
  under a heading that promises "exactly what CI runs".

