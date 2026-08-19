# Changelog — coding_adventures_paint_vm_ascii

All notable changes to this package are documented here.
Follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0]

### Added

- Initial implementation of the full `P2D02-paint-vm-ascii.md` contract:
  `rect` (fill and/or stroke), `line`, `glyph_run`, `group`, `clip`, and
  `layer`. First `paint-vm-ascii` package built from scratch for this repo
  in Dart (Dart had `paint-instructions` but no ASCII backend before this).
- `group`/`layer` reject non-identity transforms, non-default opacity,
  filters, and non-normal blend modes, matching every other language
  port's "fail loudly" behavior.
- Box-drawing merge via directional bit-flag tags: intersecting strokes
  from a rect and a line combine into the correct corner/tee/cross
  character regardless of draw order.
- Geometry validation, saturating coordinate conversion, and bounded scene
  dimensions applied from the start (see README "Hardening" section) —
  carried over directly from the hardening rounds the Haskell, Java, and
  Kotlin `paint-vm-ascii` ports went through for the same contract.
- Unlike the JVM ports, `glyph_run` accepts the full Unicode scalar-value
  range (not just the Basic Multilingual Plane), since `String.fromCharCode`
  builds a correct surrogate pair for a supplementary-plane code point — see
  README "A Dart-specific relaxation".
- `group`/`clip`/`layer` nesting is capped at 64 levels (a new
  `SceneTooDeep` error past that), closing a `StackOverflowError` DoS found
  via `/security-review` — recursion into container children had no depth
  bound.
- This backend is consumed by the Dart `cowsay` port (see
  `code/specs/cowsay-paintvm-pipeline.md`).
