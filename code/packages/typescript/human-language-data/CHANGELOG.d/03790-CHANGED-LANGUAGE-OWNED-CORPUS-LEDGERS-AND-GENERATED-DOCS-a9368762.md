### Changed - language-owned corpus ledgers and generated docs

- Split modality output into `core/lesson-modality/<language>.json` shards and
  reconstruct the compatible corpus manifest at read time.
- Move exact continuity and modality regressions into 23 independently discovered
  `tests/corpus/<language>.test.ts` files backed by language-owned ledgers.
- Replace the generated top-level README table with `progress/<language>.md` cards,
  so curriculum PRs no longer serialize on shared test or documentation lines.

