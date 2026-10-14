### Changed — Bengali's etymology gap pins fall to 0 and 0

`tests/etymology-gap-pin-owners/bengali.owner.ts` drops from `{ withoutHook: 1136, withoutVisible: 1144 }` to `{ withoutHook: 0, withoutVisible: 0 }`, with a factual rationale in place of the migration note. Every Bengali word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.** 1,144 new "taken apart" blocks or origin sentences, and 23 hooks for lessons that already named an origin. Sources are Turner's CDIAL for inherited words and Sanskrit loans (1,502 citations, each checked by script against the entry and its Bengali rows), Lane's Lexicon for Arabic roots, and named donors for Persian, English and Portuguese loans. 36 say the origin is not settled, after an independent review of every changed lesson fixed 24 (2.1%), 10 of them false "not settled" claims. The Bengali track changelog has the detail, including the older passages corrected and the nine `max_seconds` raises, all under 300 s.
- **Empty etymology block pin 3 → 0.** `EMPTY_ETYMOLOGY_BLOCK_PINS.bengali` in `tests/root-ledger.test.ts` falls to 0.
- **No code changed.** Only test pins and the owner's rationale moved.
