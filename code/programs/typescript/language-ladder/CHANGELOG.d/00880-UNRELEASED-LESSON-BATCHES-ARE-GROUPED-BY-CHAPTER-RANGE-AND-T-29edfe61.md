## Unreleased — lesson batches are grouped by chapter range, and the request budget is derived

- Group lazy lesson batches by a **chapter range** — five chapters of one
  track's lesson series — instead of by track-then-size (#12918). A band gains a
  batch only when a track passes a chapter multiple, so the emitted count now
  follows **chapters** rather than corpus bytes.
- Replace the hardcoded 353-request ceiling in `scripts/check-bundle.mjs` with a
  budget **derived from the corpus**: `bands + BAND_SPLIT_SLACK`, where the band
  count is computed by scanning the curriculum tree and the slack (1) counts
  bands dense enough that the size backstop must split them. Adding lessons
  inside a band moves neither side; adding chapters moves both together; a
  grouping regression moves only the emitted count and fails.
- Demote `maxSize` from splitter to **backstop**, 56 kB → 256 kB. It now binds on
  exactly one band in the whole corpus, which is what keeps the emitted count
  equal to the band count plus one.
- Measured on the corpus at the time of the change: **353 → 281 batches**, median
  batch 40,529 B → 40,731 B (within 500 bytes), p90 52,598 B → 90,225 B, largest
  54,688 B → 200,124 B. The eager chunk that governs first paint is untouched at
  317,430 B against its 500 kB limit — lesson batches are lazy and never entered
  that budget.
- Five-chapter bands rather than ten, deliberately: ten halves the count again
  but nearly doubles the median batch, and the number that costs a reader
  anything is the payload of the batch their next lesson lands in, not how many
  batches exist.
- Include the **lesson series letter** in the group key. 599 of the corpus's
  4,154 lesson files are not `XX-C<digits>` — writing lessons (`AR-W00-…`) and
  review lessons (`ES-R02-…`) — and a chapter pattern that ignores them drops
  them out of the group and undercounts the budget on both sides.
- Report the derived budget on success as well as failure, and fail with a
  diagnosis rather than an unhandled `ENOENT` when the curriculum tree cannot be
  read.
- Define the grouping ONCE, in `lesson-bands.mjs`, imported by both the bundler
  config and the gate. The gate previously recovered the band width by
  regex-ing `vite.config.ts`, and `exec` takes the first match anywhere in the
  file including comments — so a comment quoting the old value handed the
  checker a band width of 1 while the bundler used 5, inflating the budget from
  281 to 1,158 and letting a full byte-linear regression pass. An import cannot
  be shadowed by a comment.
- Compare the emitted chunks against the derived bands as SETS rather than
  comparing counts, so a regression that reshapes the grouping without changing
  the total is caught, and the one legitimate backstop split is named
  (`lessons-spanish-C5-`) instead of hiding inside a bare slack number.
- Refuse symlinks in the corpus walk (`lstat`, and `isFile()` on entries): a
  symlinked `lessons/` directory would enumerate a tree outside the repository
  and mint bands for it, raising the budget while the bundler — whose glob
  resolves symlinks — emitted nothing to match. Also switch the pre-existing
  staleness walk from `stat` to `lstat`, since it recurses unbounded and a
  symlink cycle would exhaust the stack.
- Bound the chapter digit run, so a long enough one cannot reach `Infinity` and
  mint a `lessons-<track>-CInfinity` band, and whitelist track directory names
  rather than merely excluding path separators — Rollup's filename sanitiser
  leaves `#` and `%`, which produce assets a browser truncates at the fragment.

