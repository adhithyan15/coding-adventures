### Changed — Telugu's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.telugu` in `tests/root-ledger.test.ts` is lowered from
1376 word lessons without an `etymology_hook` and 1318 without visible
etymology to **0** and **0**. Every Telugu word lesson now says where its word
comes from (HL24 §9).

- **Sources.** Native words cite the Dravidian Etymological Dictionary (DEDR,
  Burrow & Emeneau 1984) by entry number; Sanskrit loans cite Turner's
  Comparative Dictionary of the Indo-Aryan Languages (CDIAL) where it has the
  form. Both were read locally from the moli-mandala data (the full DEDR
  article texts, not the flattened `dedr.csv`) and checked lesson by lesson; no
  dataset file is vendored and no entry text is copied. A cognate is named only
  when the full DEDR article lists it under that language in the same sense,
  and DEDR's "Cf. Skt.", "?" and appendix notes and CDIAL's "← Drav.",
  non-Aryan, Munda and Austroasiatic notes are carried into the lesson.
- **Shape.** A `## The word, taken apart` (or `phrase`) block, which the parser
  types `etymology`, after the introducing block, and a matching
  `etymology_hook` of 120 characters or fewer. 47 early hand-written lessons
  state the origin and its source inside the section that already discussed
  it.
- **What they say.** Of the 1385 lessons touched: about 596 native Dravidian,
  334 Sanskrit or Indo-Aryan, 180 that combine or compare the two, 147 English,
  Persian, Arabic, Hindustani or Portuguese loans and 30 compounds of a loan
  with a native or Sanskrit part. 98 say the origin is uncertain and 26 more
  give only the usual account. 795 lessons cite DEDR (917 citations, 565
  entries) and 417 cite CDIAL (455 citations, 372 entries). Rejected rows
  include homographs (DEDR 3720 *nuvvu* "sesame" for "you", DEDR 321 *rāyi*
  "stone" for "write", CDIAL 8052 *pāta* for *pāta* "old") and comparisons DEDR
  makes for another member of an entry.
- **Ledger side effects.** 1353 Telugu lessons gained `roots:` slugs; 755 of
  the 1560 additions reuse a slug the corpus already had.
  `check:root-slug-splits` reports no new split. 7 lessons, all already over
  their declared length before this change, had `duration.max_seconds` raised
  to their computed length, all under 300 s; the Telugu changelog lists them.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
