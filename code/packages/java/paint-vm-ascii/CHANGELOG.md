# Changelog — com.codingadventures:paint-vm-ascii

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0]

### Added

- Initial implementation of the full `P2D02-paint-vm-ascii.md` contract:
  `rect` (fill and/or stroke), `line`, `glyph_run`, `group`, `clip`, and
  `layer`. First `paint-vm-ascii` package built from scratch for this repo
  (Java had `paint-instructions` but no ASCII backend before this).
- `group`/`layer` reject non-identity transforms, non-default opacity,
  filters, and non-normal blend modes, matching every other language
  port's "fail loudly" behavior.
- Box-drawing merge via directional bit-flag tags: intersecting strokes
  from a rect and a line combine into the correct corner/tee/cross
  character regardless of draw order.
- Geometry validation, saturating coordinate conversion, and bounded scene
  dimensions applied from the start (see README "Hardening" section) —
  carried over directly from the hardening rounds the Haskell
  `paint-vm-ascii` port went through for the same contract.
- This backend is consumed by the Java `cowsay` port (see
  `code/specs/cowsay-paintvm-pipeline.md`).
