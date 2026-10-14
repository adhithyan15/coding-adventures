### Changed — Japanese's etymology gap pins fall to 0 and 0

`tests/etymology-gap-pin-owners/japanese.owner.ts` drops from `{ withoutHook: 601, withoutVisible: 688 }` to `{ withoutHook: 0, withoutVisible: 0 }`, and `EMPTY_ETYMOLOGY_BLOCK_PINS.japanese` in `tests/root-ledger.test.ts` drops from 2 to 0. Every Japanese word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.**
  - 689 new "taken apart" blocks.
  - The 2 empty sections each gained one origin sentence.
  - 4 lessons gained a corrected hook, and 91 older hooks that did not say where the word comes from were replaced.
- **Sources.**
  - JMdict (EDRDG) for kanji spellings, ateji and meaning spellings, and loanword donors.
  - KANJIDIC2 for on'yomi and kun'yomi, and Unihan for character meanings, both checked by script.
  - Middle Chinese readings in Baxter–Sagart's notation.
- **Uncertain origins.** 9 say the origin is not settled.
- **More detail.** The Japanese track changelog covers the counts by origin kind; no `max_seconds` value was raised.
- **No code changed.** Only the pins and the owner's rationale moved.
