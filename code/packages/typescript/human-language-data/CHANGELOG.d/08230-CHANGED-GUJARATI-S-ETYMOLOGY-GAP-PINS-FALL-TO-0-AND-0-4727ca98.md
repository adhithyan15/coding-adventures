### Changed — Gujarati's etymology gap pins fall to 0 and 0

The Gujarati etymology gap pin, in `tests/etymology-gap-pin-owners/gujarati.owner.ts`, drops from `{ withoutHook: 529, withoutVisible: 543 }` to `{ withoutHook: 0, withoutVisible: 0 }`. Every Gujarati word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.** 545 new "taken apart" blocks. 31 lessons whose block or section had no origin gained one sentence, among them all 24 Gujarati "taken apart" blocks that named no origin, so the track's empty-block count also falls to 0, and `EMPTY_ETYMOLOGY_BLOCK_PINS.gujarati` (added on main in #17461) drops from 24 to 0 with it. Sources are Turner's CDIAL for inherited words and Sanskrit loans (913 citations, each checked by script against the entry), Lane's Lexicon for Arabic roots, and named donors for Persian, English and Portuguese loans. 44 say the origin is not settled or only partly known. The Gujarati track changelog has the detail, including 8 older passages corrected and the 4 `max_seconds` raises, all under 300 s.
- **No code changed.** Only the pins and the owner's rationale moved.
