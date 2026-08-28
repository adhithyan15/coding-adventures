# Changelog

## Unreleased

- implement the full `P2D02-paint-vm-ascii.md` contract: `rect` (now
  including stroke, merged with fill via box-drawing/fill-character
  priority), `line` (horizontal/vertical fast paths plus Bresenham for the
  diagonal case), `glyph_run`, `group`, `clip`, `layer`. `path` and any
  unrecognized instruction kind still fail loudly via `error(...)`, per
  spec.
- seed the diagonal-line Bresenham error term to `delta_col - delta_row`
  from the first iteration, not `0` -- the zero-seeded version hangs forever
  for some slopes (e.g. `deltaRow=1, deltaCol=3`), a real bug already found
  and fixed in the haskell/java/kotlin ports (GitHub issue #12093) and
  avoided from the start in dart/swift. This port follows the dart/swift
  precedent.
- add hardening carried over from the dart/swift ports of this same
  contract: rect/line/clip geometry validation (negative rect dimensions,
  non-finite line coordinates, non-finite or negative-size clip extents
  including the `x + width` / `y + height` sum-to-infinity edge case), a
  2000x2000-cell scene-size cap checked both per-axis and by total cell
  count (a product-only check is defeated by a zero-width, huge-height
  scene), saturating scene-to-cell coordinate conversion (a
  billion-cell bound, so a huge-but-finite coordinate can't defeat clip
  clamping via arithmetic overflow downstream), a 64-level cap on nested
  `group`/`clip`/`layer` children (recursion is otherwise bounded only by
  the Lua call stack, which cannot be trapped with `pcall` the way every
  other failure in this module can), and a glyph safety filter that
  replaces control characters, UTF-16 surrogate code points
  (`U+D800`-`U+DFFF` -- meaningless as UTF-8, but `utf8.char` does not
  reject them), bidi-control code points, and out-of-range code points with
  `"?"` rather than emitting them or raising.
- unlike the C#/F#/Haskell/Java/Kotlin/Dart/Swift ports of this contract
  (which report failures through a typed Result/Either value), this port
  keeps the `error(...)`-raises idiom the original rect-only version
  already used -- Lua has no sum types, and this matches how
  `code/packages/perl/paint-vm-ascii` (`die`/`eval`) already does the same
  thing in this repo's other "script language" port. See the module's doc
  comment for the full reasoning.
- extend the test suite to cover Bresenham slopes in every direction
  (shallow, steep, 45-degree, reversed), zero-size scenes, nesting past the
  64-level cap, and adversarial glyph input (control characters, lone
  surrogates, out-of-range code points, non-finite positions).

## 0.1.0

- add the initial Lua `paint_vm_ascii` backend
- render filled rect scenes to block-character terminal output
- add rockspec, tests, and build scripts
- pin the Lua rockspec source to the package commit for reproducible installs
