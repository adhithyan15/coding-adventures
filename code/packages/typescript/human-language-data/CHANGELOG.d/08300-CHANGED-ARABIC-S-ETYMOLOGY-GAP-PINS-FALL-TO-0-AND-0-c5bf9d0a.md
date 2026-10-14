### Changed — Arabic's etymology gap pins fall to 0 and 0

`tests/etymology-gap-pin-owners/arabic.owner.ts` drops from `{ withoutHook: 1187, withoutVisible: 1195 }` to `{ withoutHook: 0, withoutVisible: 0 }`, and `EMPTY_ETYMOLOGY_BLOCK_PINS.arabic` in `tests/root-ledger.test.ts` drops from 19 to 0. Every Arabic word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.**
  - 1,197 new "taken apart" blocks.
  - 20 older blocks gained an origin sentence, the 19 empty ones among them.
  - All 75 older hooks were cut to 120 characters or replaced; 12 described grammar, script or the calendar rather than origin, or made a false claim.
- **Sources.**
  - Lane's *Arabic-English Lexicon* (laneslexicon/lexicon_xml) for every native root and its sense, checked by script: the root exists, the word or its base form is in the article, and the gloss fits Lane's sense.
  - EtymDB 2.1 for loan donors, each row read beside Lane rather than taken on trust.
- **Uncertain origins.** 33 say the origin is not settled, with competing accounts named.
- **The Arabic root ledger.** Each block's root or donor became a `roots:` slug, so `tests/corpus/arabic/pins-arabic-root-ledger-*.case.ts` moves from 107 to 943 roots; the payoffs it now counts are reuses of one root as Lane files it, and loans that only look like a root keep slugs of their own.
- **More detail.** The Arabic track changelog covers the counts by origin kind; no `max_seconds` value was raised.
- **No code changed.** Only the pins and the owner's rationale moved.
