### Changed - `core/book-generation.json` re-indented (whitespace only)

- Twelve `marwadi` entries in `targets` were indented two spaces deeper than
  canonical — a hand-merge artifact at lines 2911–2984. 74 lines, identical line
  count, **leading whitespace only**.
- Landed as **its own commit**, and proved whitespace-only by **deep-comparing
  the parsed structures** — not by reading a 6,693-line diff. The script refused
  to write unless `isDeepStrictEqual` held, top-level key order was unchanged,
  every array length was unchanged, all 1,160 array elements had the same key
  order, and no differing line differed once trimmed.
- **This exemption is narrow.** HL21 §8 says a ledger that does not round-trip is
  reported, not reformatted — and the four other such files stay untouched,
  because they are hand-maintained *curriculum data* where whitespace churn
  buries real edits. `core/book-generation.json` is a *build manifest*:
  `(language, chapter, output, scriptSet)` triples nobody reads for meaning. And
  unlike the others a track cannot be skipped here — it is one file shared by all
  23 — so "leave it alone" would have meant "never shard it".
- `tests/grouped-shards.test.ts` is **inverted** to match: it used to assert the
  file does NOT round-trip, as an executable statement of the blocker that would
  fail the day someone re-indented it. It now asserts the file STAYS canonical,
  so a hand-edit reintroducing stray indentation fails immediately.

