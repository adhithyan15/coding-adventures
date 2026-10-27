### Added — source-verified Chinese 很 ductus

- Add a font-checked ductus for `很`, fitted to the vendored Noto Sans SC
  outline from Hanzi Writer Data's nine ordered medians (snapshot 68d10a4):
  彳 (two left-falling strokes, then the vertical), then 艮 (the turning top
  and right side, two horizontals, the left side with its flick, a short
  left-falling stroke and the long right-falling stroke), with eight lifts.
  Every stroke lies wholly on the ink and none of the glyph is left untraced.
- Pin the Chinese inventory at 62 letters and the stroke-ownership owner at 62
  keys with its new key and data hashes; add a strokes test that 彳's last two
  strokes head the way 亻's do, and a ductusview case for the nine-frame strip.
