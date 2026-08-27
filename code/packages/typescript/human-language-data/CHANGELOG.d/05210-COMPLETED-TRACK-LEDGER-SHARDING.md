### Changed - every track ledger is now sharded (#12695)

- Finish the HL21 migration by adding French, Japanese, and Marwadi chapter
  ledgers plus the Marwadi curriculum ledger to `SHARD_PLANS`. All 23 tracks now
  author chapters in `chapters.d/` and curricula in `curriculum.d/`; the
  browser-facing monoliths remain generated artifacts guarded by
  `check:shards`.
- Keep the three chapter-ledger formatting changes in a separate commit and
  prove their parsed structures unchanged before generating shards. A fresh
  audit found `marwadi/curriculum.json` already canonical, so it needed no
  normalization change. Folding the new shard sets back together reproduces the
  canonical monoliths byte for byte:

  | ledger | SHA-256 |
  | --- | --- |
  | `french/chapters.json` | `d9481bc16b7d901bd3ac37e35320cd4fd2d8388583e32f0ffb1d2e184813e17f` |
  | `japanese/chapters.json` | `b35347bbecb1fedf3143d492bd365fb727b939240e7f01603c52e471f376ba7f` |
  | `marwadi/chapters.json` | `b3c63aec6bcc920f8823b8b0767567873196fe7d525cac2398bd0a683055ee4d` |
  | `marwadi/curriculum.json` | `554f3517ea0d61477fb8bbf2c105353d765c7e6fc999cd5ae63a9c884f2e2c21` |
- Extend the real-corpus round-trip, shard-closure, ordering, loader, and drift
  tests from partial coverage to every track, and update HL21 plus the package
  authoring guide to record the completed migration.
