### Added — source-verified Chinese ductus for the A1 core

- Add font-checked ductus for the first characters the A1 core writes (chapters 85-90):
  在 哪 没 两 给 能.
  Each is fitted to the vendored Noto Sans SC outline from Hanzi Writer Data's
  ordered medians (snapshot 68d10a4) with the source's order, directions,
  turns, hooks and lifts; every stroke lies wholly on the ink and none of the
  glyph is left untraced.
- Pin the Chinese inventory at 122 letters and the stroke-ownership owner at
  122 keys with its new key and data hashes; add a ductusview case and a
  per-character strokes case for each character.
