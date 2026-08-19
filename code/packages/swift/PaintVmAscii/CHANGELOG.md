# Changelog

## [0.1.0]

### Added

- Initial implementation of the full `P2D02-paint-vm-ascii.md` contract:
  `rect` (fill and/or stroke), `line`, `glyph_run`, `group`, `clip`, and
  `layer`. First `PaintVmAscii` package built from scratch for this repo
  in Swift (Swift had `PaintInstructions` but no ASCII backend before
  this — see that package's own CHANGELOG for the sum-type conversion
  this backend needed).
- `group`/`layer` reject non-identity transforms, non-default opacity,
  filters, and non-normal blend modes, matching every other language
  port's "fail loudly" behavior.
- Box-drawing merge via directional bit-flag tags: intersecting strokes
  from a rect and a line combine into the correct corner/tee/cross
  character regardless of draw order.
- Geometry validation, saturating coordinate conversion, bounded scene
  dimensions, and a 64-level nesting-depth cap applied from the start (see
  README "Hardening" section) — carried over directly from the hardening
  rounds the Haskell, Java, Kotlin, and Dart `paint-vm-ascii` ports went
  through for the same contract.
- The diagonal-line Bresenham loop is seeded with `error = deltaCol -
  deltaRow` from the start, avoiding a hang bug found in this package's
  Haskell/Java/Kotlin siblings (see
  [issue #12093](https://github.com/adhithyan15/coding-adventures/issues/12093)).
- Two Swift-specific overflow-trap fixes (`Int` addition traps on
  overflow, unlike the other languages' wrapping 64-bit arithmetic): the
  `ceilDiv` scene-size computation and `renderRectangle`'s `x+width`/
  `y+height` extent are both done in `Double` instead of `Int` — see
  README "A Swift-specific hardening note".
- This backend is consumed by the Swift `cowsay` port (see
  `code/specs/cowsay-paintvm-pipeline.md`).
