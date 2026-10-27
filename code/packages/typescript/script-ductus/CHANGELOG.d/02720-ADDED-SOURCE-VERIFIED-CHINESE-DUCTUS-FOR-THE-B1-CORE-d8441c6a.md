### Added — source-verified Chinese ductus for the B1 core

- Add font-checked ductus for the first characters the B1 core writes
  (chapters 278-289):
  最 然 第 突.
  Each is fitted to the vendored Noto Sans SC outline from Hanzi Writer Data's
  ordered medians (snapshot 68d10a4) with the source's order, directions,
  turns, hooks and lifts; every stroke lies wholly on the ink and none of the
  glyph is left untraced.
- Pin the Chinese inventory at 215 letters and the stroke-ownership owner at
  215 keys with its new key and data hashes; add a ductusview case and a
  per-character strokes case for each character.
