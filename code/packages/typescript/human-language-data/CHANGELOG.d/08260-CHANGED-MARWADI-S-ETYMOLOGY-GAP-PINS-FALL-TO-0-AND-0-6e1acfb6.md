### Changed — Marwadi's etymology gap pins fall to 0 and 0

The Marwadi etymology gap pin, in `tests/etymology-gap-pin-owners/marwadi.owner.ts`, drops from `{ withoutHook: 552, withoutVisible: 716 }` to `{ withoutHook: 0, withoutVisible: 0 }`. Every Marwadi word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.** 717 new "taken apart" blocks, and one origin sentence in MW-C05-hear-naam. `EMPTY_ETYMOLOGY_BLOCK_PINS.marwadi` stays at 0. Sources are Turner's CDIAL for inherited words and Sanskrit loans (1,237 citations, each checked by script against the entry), DEDR for two Dravidian comparisons, Lane's Lexicon for Arabic roots, and named donors for Persian, English and Portuguese loans. 52 words say the origin is not settled or only partly known. The Marwadi track changelog has the detail, including the 166 rewritten hooks and the 3 `max_seconds` raises, all under 300 s.
- **No code changed.** Only the pins and the owner's rationale moved.
