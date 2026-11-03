---
category: TypeScript / JavaScript
---

# A single JSON module that outgrows a chunk group's maxSize stays one chunk, so trim what the eager path carries rather than raising the gate

**What went wrong.** Chinese B1 added 126 source-verified characters to
`data/scripts/chinese.json`, which grew from 457 kB to 789 kB. language-ladder's
`script-data` chunk group has `maxSize: 250_000`, but a bundler can only split
BETWEEN modules, never inside one. So the whole file became a single
699,737-byte eager chunk, and CI's `check:bundle` (500 kB gate) failed on a PR
that changed no app code. The local pre-push checks did not catch it: the
language-ladder vitest suite does not build the bundle, and a full `vite build`
needs about 13.4 GB, so in this container it was OOM-killed three times.

**The fix.** I measured what the eager path actually reads. Of the 789 kB,
410 kB was `strokeOrderSource` citation and `variation` prose. That prose shows
only on the lazily loaded handwriting card, whose stroke module imports the
JSON file itself. A build-only Vite plugin
(`language-ladder/script-provenance-plugin.ts`) now gives `scriptdata.ts` a
virtual copy with each source cut to `{ url }`, and the chunk group's test
became a tested predicate that keeps the full file off the eager path. The
eager chunk dropped to 353 kB. The gate was not raised.

**Do differently.**
- When a curriculum PR grows a data file that the app imports (script
  inventories, ledgers), check the bundle before pushing.
- If a full build cannot run locally, build a minimal entry that imports the
  same modules with the same plugins and chunk groups. That harness reproduced
  CI's chunk byte for byte (`script-data-CAP5vG0m.js`, 699,737 B).
- When one module outgrows a size budget, look for weight the eager path never
  reads before reaching for a bigger budget.
- Before slimming a shared copy, check every reader of the field. Arabic's
  stroke module reads sources through the copy that gets slimmed, so Arabic is
  deliberately left out.
