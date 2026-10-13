### Changed — German's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.german` in `tests/root-ledger.test.ts` is lowered from
988 word lessons without an `etymology_hook` and 1071 without visible etymology
to **0** and **0**. Every German word lesson now says where its word comes from
(HL24 §9).

- **Source.** EtymDB 2.1 (Fourrier & Sagot 2020, LREC), CC BY-SA 4.0, read
  locally and checked lesson by lesson. No dataset file is vendored. Each
  statement is our own sentence, and "(EtymDB 2.1)" sits beside the facts a
  row actually holds. 98 blocks trace a word EtymDB has no row for through the
  rows for its English, Dutch or Old English sisters. 35 blocks carry no tag:
  builds from words already taught, and a few well-established accounts.
- **Shape.** A `## The word, taken apart` (or `phrase` / `letter`) block,
  which the parser types `etymology`, after the introducing block, and a
  matching `etymology_hook` of 120 characters or fewer.
- **What they say.** 355 inherited, 202 borrowed (Germanic words taken from Low German
  count as borrowed), 502 built from parts, 12 genuinely uncertain (each
  hedged), and 6 lessons that already told their
  story gained only a hook. Over 90 EtymDB rows or labels were rejected:
  homographs (*Salat* ← Arabic *ṣalāt* "prayer", *Hafen* ← *hafan* "pot"),
  Latin loans labelled inherited (*Fenster*, *Wein*, *Käse*), and junk parents.
- **Ledger side effects.** 775 German lessons gained a `roots:` slug; 227 of
  the additions reuse an existing slug, so German cousins now join Spanish and
  Italian ones (`fenestra-latin`, `gardaz-germanic`, `northa-germanic`).
  `check:root-slug-splits` reports no new split. 61 lessons had
  `duration.max_seconds` raised to their computed length, all under 300 s; the
  German changelog lists them.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
