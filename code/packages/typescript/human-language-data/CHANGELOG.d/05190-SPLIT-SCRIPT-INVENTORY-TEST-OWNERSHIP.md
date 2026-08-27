### Changed — script inventory tests no longer serialize unrelated authors

- Move all 585 exact real-corpus glyph, stroke-order, provenance, pen-lift, and
  closure assertions out of the cross-language `integration.test.ts` bottleneck.
- Give Arabic, Devanagari, Japanese, Kannada, Malayalam, Perso-Arabic, Tamil,
  Telugu, and Urdu Nastaliq stable inventory-owned evidence modules, discovered
  by the existing integration gate and backed by one production-validator
  measurement helper and one corpus load.
- Preserve the parsed assertion multiset exactly while shrinking the genuinely
  cross-corpus integration file from 1,750 to 400 lines.
