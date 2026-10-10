### Fixed — script closure reaches zero: Chinese chapter 1 describes its components in words

- **What changed.** ZH-C01-ni and ZH-C01-hao, the last two lessons the
  script-closure measurement flagged, printed the components 亻 尔 人 and 女 子
  a chapter before the chapter 2 writing lessons that teach them. They now
  describe each component in words and pinyin (the person piece *rén* and
  sound piece *ěr*; woman *nǚ* beside child *zǐ*), and their practice asks
  the learner to point at halves of the headword instead of naming
  components they cannot yet read. The decoding itself already lives in
  ZH-W01-ni-build and ZH-W01-hao-build.
- **Report.** Script-closure violations 2 -> 0 (every non-Latin track at
  zero; Chinese exposure-only lessons 3 -> 5). ZH-C01-ni also leaves the
  glyph budget (script-ramp over-budget lessons 28 -> 27), so Chinese's
  gentle-ramp `glyph-step` finding clears (findings 90 -> 89). No other
  measurement moves, and no new finding of any kind.
- **Ceiling lowered, re-measured:** `tests/script-closure.test.ts` now pins
  corpus closure violations at exactly 0.
- `src/script-closure.ts` is unchanged.
