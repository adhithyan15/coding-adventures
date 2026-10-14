### Changed — Chinese's etymology gap pins fall to 0 and 0

`tests/etymology-gap-pin-owners/chinese.owner.ts` drops from `{ withoutHook: 0, withoutVisible: 62 }` to `{ withoutHook: 0, withoutVisible: 0 }`, and `EMPTY_ETYMOLOGY_BLOCK_PINS.chinese` in `tests/root-ledger.test.ts` drops from 43 to 0. Every Chinese word lesson now states its word's origin in its body, and its hook says where the word comes from.

- **What the lessons gained.**
  - 30 new "taken apart" blocks, and an origin sentence in pinyin in 32 listening lessons.
  - The 43 empty blocks each gained one origin sentence.
  - All 105 hooks were replaced: they described character shapes, strokes or tones, not the word's origin.
- **Sources.**
  - Middle Chinese readings in Baxter's transcription, read from the Guangyun categories (`tshet-uinh`), all 57 checked by script.
  - Unihan for character glosses.
- **Origin kinds.** 41 single inherited words, 58 compounds or patterns of inherited words, 5 origins stated as not settled, 1 the Hindu-Arabic digits; 4 hedged Tibetan comparisons.
- **More detail.** The Chinese track changelog covers the roots added and the readings chosen; no `max_seconds` value was raised.
- **No code changed.** Only the pins and the owner's rationale moved.
