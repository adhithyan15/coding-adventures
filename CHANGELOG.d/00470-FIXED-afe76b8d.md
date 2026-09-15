### Fixed
- `human-language-data`: a dotted circle carrying a combining mark now joins that
  mark's script run in the generated LaTeX. U+25CC is `Script_Extensions=Common`,
  so it was handed to the Latin body font, which has no such glyph — the first
  build of these segments logged **184 "Missing character" warnings** and left a
  hole exactly where the character being taught should have been.


