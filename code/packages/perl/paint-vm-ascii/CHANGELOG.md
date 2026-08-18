# Changelog

## 0.2.0

- implement the full `P2D02-paint-vm-ascii.md` contract: add `line`,
  `glyph_run`, `group`, `clip`, and `layer` instruction handling (previously
  `rect`-only)
- `group`/`layer` now reject non-identity transforms, non-default opacity,
  filters, and non-normal blend modes by `die`ing loudly, matching every
  other language port
- rect rendering now supports `stroke` (box-drawing characters) in addition
  to `fill`, and correctly treats an omitted/undefined `fill` as no fill
  (transparent) instead of defaulting to black -- a divergence from the
  P2D00 spec in the original rect-only implementation
- replace the libc-dependent `sprintf('%.0f', ...)` coordinate rounding with
  an explicit round-half-away-from-zero helper, matching every other port's
  rounding convention
- this backend is now consumed by the Perl `cowsay` port (see
  `code/specs/cowsay-paintvm-pipeline.md`)

## 0.1.0

- add the initial Perl paint-vm-ascii backend
- render filled rectangles to block-character terminal output
- add publishable package metadata and tests
