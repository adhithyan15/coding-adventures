### Changed — Italian's etymology gap pins fall to 745 and 745

`ETYMOLOGY_GAP_PINS.italian` in `tests/root-ledger.test.ts` is lowered from
1126 word lessons without an `etymology_hook` and 1135 without visible etymology
to **745** and **745**. That is the HL24 §9 pilot for one track: the first 401
gap lessons in sequence order (IT-C01-ciao through IT-C110-pepe) now say where
their word comes from.

- **Source.** EtymDB 2.1 (Fourrier & Sagot 2020, LREC), CC BY-SA 4.0. It was
  read locally and checked lesson by lesson. No dataset file is vendored. Each
  statement is our own sentence and cites "(EtymDB 2.1)".
- **Shape.** A `## The word, taken apart` (or `## The phrase, taken apart`)
  block, which the parser types `etymology`, after the introducing block. A
  matching `etymology_hook` of 120 characters or fewer.
- **Honest answers.** 239 inherited, 43 borrowed, 34 built from parts, and 85
  "Its origin is not recorded in EtymDB 2.1". The last group includes two where
  EtymDB's origin could not be confirmed. 33 EtymDB rows were rejected, for
  wrong parents, loans labelled as inherited, or junk glosses.
- **Ledger side effects.** 208 Italian lessons gained a `roots:` slug, 78 of
  them reusing an existing slug. `check:root-slug-splits` reports no new split.
  The root ledger and cousin counts are floors and only grow. Nine lessons had
  `duration.max_seconds` raised to their computed length, all under 300 s; the
  Italian changelog lists them.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
