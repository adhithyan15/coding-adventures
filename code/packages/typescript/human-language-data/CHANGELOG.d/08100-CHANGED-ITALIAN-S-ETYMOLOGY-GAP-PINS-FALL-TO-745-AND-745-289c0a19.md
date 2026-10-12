### Changed — Italian's etymology gap pins fall to 745 and 745

`ETYMOLOGY_GAP_PINS.italian` in `tests/root-ledger.test.ts` is lowered from
1126 word lessons without an `etymology_hook` and 1135 without visible etymology
to **745** and **745**. That is the HL24 §9 pilot for one track: the first 401
gap lessons in sequence order (IT-C01-ciao through IT-C110-pepe) now say where
their word comes from.

- **Sources.** EtymDB 2.1 (Fourrier & Sagot 2020, LREC), CC BY-SA 4.0. Where
  EtymDB has no row for a textbook word, the Latin form is cited from Lewis &
  Short (Perseus `lexica`, CC BY-SA 4.0), and only when the derivation is
  certain. Both were read locally and checked lesson by lesson. No dataset
  file is vendored. Each statement is our own sentence and names its source.
- **Shape.** A `## The word, taken apart` (or `## The phrase, taken apart`)
  block, which the parser types `etymology`, after the introducing block. A
  matching `etymology_hook` of 120 characters or fewer.
- **Honest answers.** 271 inherited, 60 borrowed and 58 built from parts.
  12 say "Its origin is not recorded in EtymDB 2.1", and all 12 are genuinely
  uncertain origins. 19 words with no EtymDB row are traced through the sister
  words EtymDB derives from the same Latin etymon. 26 cite Lewis & Short. Four
  well-established origins that neither source confirms carry no source tag.
  41 EtymDB rows or labels were rejected, for wrong parents, loans labelled as
  inherited, or junk glosses.
- **Ledger side effects.** 236 Italian lessons gained a `roots:` slug, 87 of
  them reusing an existing slug. `check:root-slug-splits` reports no new split.
  The root ledger and cousin counts are floors and only grow. Nine lessons had
  `duration.max_seconds` raised to their computed length, all under 300 s; the
  Italian changelog lists them.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
