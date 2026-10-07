# Changelog — com.codingadventures:paint-vm-ascii

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Fixed

- Diagonal `line` rendering no longer hangs for some slopes (issue #12093).
  The Bresenham loop seeded its error term with `0` instead of the
  standard `deltaCol - deltaRow` ("err = dx - dy"), so for ratios such as
  `deltaRow=1, deltaCol=3` or `deltaRow=3, deltaCol=1` the minor-axis cursor
  overshot the endpoint and the `row == r2 && col == c2` exit was never
  reached again — an infinite loop, not an error. The seed is now
  `deltaCol - deltaRow`, with a literate comment spelling out the
  error-term invariant (`error == -F(i+1, j+1)` for the ideal line
  `F(i, j) = deltaRow*i - deltaCol*j`) and a worked trace.
- Added a Bresenham regression suite that drives the public `render` API
  at 1x1 scale and checks: the exact cells for the issue's shallow
  (dRow=1, dCol=3) and steep (dx=1, dy=3) lines in both directions, all
  eight octants, the four 45-degree diagonals, horizontal, vertical and
  single-point lines, and an exhaustive sweep of every endpoint within 6
  cells of a centre point. Each case asserts the path starts at p0, ends
  at p1, has exactly `max(|dx|, |dy|) + 1` cells, and moves exactly one
  cell on the major axis and at most one on the minor axis per step.

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
