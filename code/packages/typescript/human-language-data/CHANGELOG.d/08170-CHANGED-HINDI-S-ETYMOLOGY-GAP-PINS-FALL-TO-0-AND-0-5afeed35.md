### Changed — Hindi's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.hindi` in `tests/root-ledger.test.ts` drops from `{ withoutHook: 1126, withoutVisible: 967 }` to `{ withoutHook: 0, withoutVisible: 0 }`. Every Hindi word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.** 962 new "taken apart" blocks. 43 lessons whose block or section had no origin gained one sentence. Sources are Turner's CDIAL for inherited words and Sanskrit loans (1,492 citations, each checked by script against the entry), Lane's Lexicon for Arabic roots, and named donors for Persian, English and Portuguese loans. About 48 say the origin is uncertain. The Hindi track changelog has the detail, including 15 older blocks corrected and the 41 `max_seconds` raises, all under 300 s.
- **No code changed.** Only the pin and its comment moved.
