### Changed — Portuguese's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.portuguese` in `tests/root-ledger.test.ts` is lowered from
1142 word lessons without an `etymology_hook` and 1141 without visible etymology
to **0** and **0**. Every Portuguese lesson that introduces a word now says where
that word comes from (HL24 §9).

- **Sources.** EtymDB 2.1 (Fourrier & Sagot 2020, LREC), CC BY-SA 4.0. Where
  EtymDB has no row for a certain textbook Latin form, it is cited from Lewis &
  Short (Perseus `lexica`, CC BY-SA 4.0). Both were read locally and every row
  was checked by hand. No dataset file is vendored. Each statement is our own
  sentence, and its tag sits beside the etymon the source supports.
- **Shape.** A `## The word, taken apart` (or `## The phrase, taken apart`)
  block, which the parser types `etymology`, after the introducing block, and a
  matching `etymology_hook` of 120 characters or fewer. Twelve chapter 1–4
  lessons gained only the hook; eleven whose hook was not etymology gained a
  block and a rewritten hook.
- **Honest answers.** 469 inherited, 215 learned, 137 borrowed with the donor
  named, 271 built from parts. 61 say the origin is uncertain or disputed. 47
  words with no EtymDB ancestor row are traced through the rows for their
  sister words, and 136 lines cite Lewis & Short. 228 EtymDB rows or labels
  were rejected: loans and learned words labelled inherited, homographs
  (*colher* "spoon" ← *colligere*, *canto* "corner" ← *cantus*), and wrong
  parents.
- **Ledger side effects.** 940 Portuguese lessons gained a `roots:` slug, 576 of
  them reusing an existing slug; 289 slugs are new. `check:root-slug-splits`
  reports no new split. Six lessons had `duration.max_seconds` raised to their
  computed length, all under 300 s; the Portuguese changelog lists them.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology, and `measureContinuity` already
  skips etymology blocks, so the forward-reference pin (34) does not move.
