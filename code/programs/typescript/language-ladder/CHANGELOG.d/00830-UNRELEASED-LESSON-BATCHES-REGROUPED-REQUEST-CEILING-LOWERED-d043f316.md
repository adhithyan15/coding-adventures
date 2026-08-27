## Unreleased -- lesson batches regrouped, request ceiling lowered 399 -> 353

- Raise the lesson-group `maxSize` in `vite.config.ts` from **49 kB to 56 kB**.
  This is a bundler GROUPING parameter, not a budget: on the corpus after the
  Spanish A1 vocabulary tranche it takes the emitted batch count from **401 to
  353** while the corpus itself grows by 35 lessons.
- **Lower** the request-count ceiling in `scripts/check-bundle.mjs` from 399 to
  the measured **353**, in the same commit. A ceiling that may fall should fall
  when it falls; leaving it at 399 would have banked 46 batches of slack for the
  next regression to hide in. The count moved down because grouping changed, not
  because content shrank.
- Raise the mirrored largest-batch limit 49 kB -> 56 kB to match the grouping
  parameter. Largest emitted batch is 54,688 B, about **11%** of the 500 kB
  eager-chunk budget, which is the limit that actually protects the browser and
  which did not move.
- Second occurrence of this recurrence (the first was 32 kB -> 49 kB). Both
  files now say a third should not happen, and point at the structural fix:
  group batches by a chapter range rather than track-then-size, so the count
  grows sublinearly in corpus bytes.
