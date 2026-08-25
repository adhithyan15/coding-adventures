### Fixed
- A dotted circle carrying a combining mark now joins that mark's script run when
  inline markdown is rendered to LaTeX. U+25CC DOTTED CIRCLE has
  `Script_Extensions=Common`, so it matched no script and was emitted outside the
  run — handing it to the Latin body font, which has no such glyph. The first
  build of HL12's Indic recognition segments logged 184 `Missing character`
  warnings, one per use, and printed nothing where the character being taught
  should have been. The dotted circle exists precisely to be the base a combining
  mark is shown on, so when the next character belongs to a run, it belongs to
  that run too.

