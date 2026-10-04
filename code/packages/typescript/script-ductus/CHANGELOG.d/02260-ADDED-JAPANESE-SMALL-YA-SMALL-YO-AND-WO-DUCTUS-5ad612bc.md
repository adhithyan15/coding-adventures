### Added Japanese small ゃ, small ょ and を ductus

- `ゃ` (U+3083), `ょ` (U+3087) and `を` (U+3092) enter
  `src/strokes/japanese.ts` for the chapter 131 writing lessons. Each one
  already has a `strokeOrderSource` in `data/scripts/japanese.d`. Without a
  ductus, the gate that every verified claim can be drawn would fail.
- **Small `ゃ` and `ょ` follow the small `ゅ` rule.** The order and captions
  are `や`'s and `よ`'s, word for word. The coordinates are the full-size
  sign's verified path, mapped through the two glyphs' bounding boxes and
  snapped to the small glyph's own medial line. No final point lies more than
  23 (ゃ) or 34 (ょ) units from the mapped path. A named test checks that
  each small path sits in a smaller box than its full-size twin.
- **`を` takes its order and direction from KanjiVG's three directed paths**
  (`kanji/03092.svg`). Those paths were scaled onto the Noto Sans JP subset
  outline, and every sample was snapped to that outline's skeleton. So the
  coordinates are the print glyph's medial line, not KanjiVG's handwriting
  proportions. Stroke 2 is one run with a sharp turn at its foot, and the
  turn is the boundary between its two labelled segments.
- Measured, not asserted: `fractionOnInk` is 1.0000 on every stroke except
  `ょ`'s looping stroke at 0.9922, which is above the 0.97 floor with no
  override. Every join is exact. No ink point is left untraced (530 sampled
  for ゃ, 483 for ょ, 804 for を). The three filmstrips were rendered and
  checked by eye.
- `tests/stroke-ownership.test.ts` was re-measured: keys 412 -> 415,
  Japanese 23 -> 26, plus the ordered key hash and the non-Tamil data hash.
  Tamil and both shared-identity values do not move. The filmstrip-geometry
  ledger was regenerated.
