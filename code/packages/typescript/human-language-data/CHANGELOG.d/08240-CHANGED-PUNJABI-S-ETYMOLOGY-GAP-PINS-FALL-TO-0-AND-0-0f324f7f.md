### Changed — Punjabi's etymology gap pins fall to 0 and 0

`tests/etymology-gap-pin-owners/punjabi.owner.ts` drops from `{ withoutHook: 549, withoutVisible: 524 }` to `{ withoutHook: 0, withoutVisible: 0 }`, and `EMPTY_ETYMOLOGY_BLOCK_PINS.punjabi` in `tests/root-ledger.test.ts` drops from 31 to 0. Every Punjabi word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.**
  - 524 new "taken apart" blocks.
  - The 31 empty sections each gained one origin sentence.
  - 21 lessons gained a hook.
- **Sources.**
  - Turner's CDIAL for inherited words and Sanskrit loans: 810 citations, each checked by script against the entry's Punjabi rows.
  - Lane's Lexicon for Arabic roots.
  - Named donors for Persian, English and Portuguese loans.
- **Uncertain origins.** 29 say the origin is not settled.
- **More detail.** The Punjabi track changelog covers the three `max_seconds` raises, all under 300 s.
- **No code changed.** Only the pins and the owner's rationale moved.
