### Changed — Marathi's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.marathi` in `tests/root-ledger.test.ts` drops from `{ withoutHook: 1163, withoutVisible: 1116 }` to `{ withoutHook: 0, withoutVisible: 0 }`. Every Marathi word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.** 1,116 new "taken apart" blocks, and 58 lessons whose text already named an origin gained a hook, with a corrected or added sentence where needed. Sources are Turner's CDIAL for inherited words and Sanskrit loans (about 1,660 citations, each checked by script against the entry and its Marathi rows), DEDR for the few Dravidian questions, Lane's Lexicon for Arabic roots, and named donors for Persian, English and Portuguese loans. About 130 say the origin is not settled. The Marathi track changelog has the detail, including about 15 older passages corrected and the 43 `max_seconds` raises, all under 300 s.
- **No code changed.** Only the pin and its comment moved.
