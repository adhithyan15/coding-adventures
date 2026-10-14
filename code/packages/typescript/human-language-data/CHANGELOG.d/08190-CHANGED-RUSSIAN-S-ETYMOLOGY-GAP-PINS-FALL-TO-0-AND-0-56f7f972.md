### Changed — Russian's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.russian` in `tests/root-ledger.test.ts` is lowered from
1135 word lessons without an `etymology_hook` and 1148 without visible
etymology to **0** and **0**. Every Russian word lesson now says where its word
comes from (HL24 §9).

- **Source.** Max Vasmer's *Russisches etymologisches Wörterbuch* (Russian
  edition, with Trubachev's additions), read locally and checked lesson by
  lesson; no dataset file is vendored and no article text is copied. Each fact
  from Vasmer carries a "(Vasmer)" citation beside it, his hedges are kept, and his
  older reconstruction notation is restated as standard starred forms. Old
  forms that need a letter the track never teaches (*ъ*, *ѣ*) are
  transliterated, which keeps `script-closure` at 0. A
  cousin is named only for an inherited relative; a loan names its donor and
  route. 935 lessons cite Vasmer (1106 citations). The Latin, Greek and
  Germanic forms quoted were checked against the article text of the word.
- **Shape.** A `## The word, taken apart` block, which the parser types
  `etymology`, after the introducing block, and a matching `etymology_hook` of
  120 characters or fewer, in all 1,135 generated word lessons. Thirteen
  earlier hand-written lessons gained the half they lacked: six as a block,
  seven as one origin sentence in an existing section, paid for by trimming.
- **What they say.** Of the 1,135 generated blocks: 457 words built from parts
  the learner can see, 257 inherited Slavic words and 63 more whose deeper
  origin is unclear, 40 Church Slavonic shapes, 90 Germanic, 52 French, 28
  Greek, 26 English and 15 Turkic loans, 84 from other donors (Latin, Polish,
  Italian, Persian, Arabic and others), and 23 Vasmer calls uncertain or
  disputed. Rejected readings include homographs (*лук* "onion" against "bow",
  *мука* "flour" against "torment", *ключ* "key" against "spring", *марка*
  "stamp" against the old coin) and cousins that are really loans (English
  *garden* for *город*).
- **Ledger side effects.** 1143 Russian lessons gained `roots:` slugs; 332 of
  the 1167 additions reuse a slug the corpus already had.
  `check:root-slug-splits` reports no new split. One lesson,
  RU-C02-menya-zovut, already over its declared length before this change, had
  `duration.max_seconds` raised to its computed 275 s.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
