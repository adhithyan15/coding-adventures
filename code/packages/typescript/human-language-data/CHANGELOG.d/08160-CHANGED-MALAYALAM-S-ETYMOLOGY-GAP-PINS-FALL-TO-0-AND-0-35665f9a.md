### Changed — Malayalam's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.malayalam` in `tests/root-ledger.test.ts` is lowered from
1367 word lessons without an `etymology_hook` and 1275 without visible
etymology to **0** and **0**. Every Malayalam word lesson now says where its
word comes from (HL24 §9).

- **Sources.** Native words cite the Dravidian Etymological Dictionary (DEDR,
  Burrow & Emeneau 1984) by entry number; Sanskrit loans cite Turner's
  Comparative Dictionary of the Indo-Aryan Languages (CDIAL) where it has the
  form. Both were read locally from the moli-mandala data and checked lesson by
  lesson; no dataset file is vendored and no entry text is copied. A cognate is
  named only when the full DEDR article lists it under that language (the
  flattened `dedr.csv` mislabels some languages), and DEDR's own "Cf. Skt." and
  "?" caveats are carried into the lesson.
- **Shape.** A `## The word, taken apart` (or `phrase`) block, which the parser
  types `etymology`, after the introducing block, and a matching
  `etymology_hook` of 120 characters or fewer. 56 lessons that already had a
  "taken apart" block gained a hook and a sentence in that block; 3 lessons
  near the 300 s cap state the origin in one sentence of an existing section.
- **What they say.** About 816 native Dravidian, 332 Sanskrit (193 whole
  loans, tatsama or tadbhava), 70 Sanskrit-and-native hybrids, 124 English,
  about 28 Arabic, Persian or Hindustani and about 10 Portuguese loans. 47 say the
  origin is uncertain or only usually traced. 857 lessons cite DEDR (1193
  citations, 570 entries) and 313 cite CDIAL (338 citations). 34 dataset rows
  were rejected: homographs (DEDR 482 *iruḷ* "ironwood" for "dark", CDIAL
  12688 *śravaṇa* "hearing" for the Onam asterism), cognates of another sense,
  and loans DEDR marks as loans.
- **Ledger side effects.** 1340 Malayalam lessons gained `roots:` slugs; 234
  additions reuse an existing slug. `check:root-slug-splits` reports no new
  split. 31 lessons had `duration.max_seconds` raised to their computed
  length, all under 300 s; the Malayalam changelog lists them.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
- **Review corrections.** A second reading against the full DEDR and CDIAL text corrected 79 lessons before merge. The changes are listed in the Malayalam changelog. The pins stay at 0 and 0.
