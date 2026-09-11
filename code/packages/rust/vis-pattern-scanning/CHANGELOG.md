# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-11

### Added

- Initial implementation, closing `VIS00-vision-roadmap.md`'s L2 gap:
  the last of the four core vision-primitives packages, turning "a
  binarized image" into "candidate finder-pattern locations."
- `scan_line(line, ratio, tolerance) -> Vec<RatioMatch>` — decomposes
  a single row or column into alternating runs and reports every
  window of `ratio.len()` consecutive runs (starting on a foreground
  run) within `tolerance` of the requested ratio.
- `find_pattern_candidates(bitmap, ratio, tolerance) -> Vec<PatternCandidate>`
  — scans every row for `scan_line` matches, cross-checking each one
  with a vertical scan through the same point; confirms a candidate
  only when the two scans agree within one estimated module size.
- `RatioMatch { center, module_size }` and
  `PatternCandidate { row, col, module_size }`.
- `MAX_TOLERANCE` (`10.0`) — `scan_line` rejects any `tolerance`
  outside `(0.0, MAX_TOLERANCE]`, not just non-positive/NaN values.
  Caught in security review: an unbounded `tolerance` (e.g.
  `f64::INFINITY`) made every foreground-starting window "match"
  regardless of its actual proportions, turning
  `find_pattern_candidates`' near-linear cost quadratic-or-worse on a
  caller-controlled bitmap.
- `MAX_MATCHES_PER_LINE` (`256`) — `scan_line` stops reporting matches
  once it hits this many on one line. Also caught in security review,
  as a follow-up to the `MAX_TOLERANCE` fix: a periodic, texture-like
  line (real photographic content — fabric, blinds, brick — not only
  an adversarial construction) can satisfy the ratio check at a large
  fraction of its windows using an entirely ordinary tolerance, so
  `MAX_TOLERANCE` alone didn't bound the same underlying cost.
- 24 unit tests + 2 doc-tests: exact and within-tolerance ratio
  matches, a just-outside-tolerance rejection, the dark-starting-run
  requirement (including a case that finds the real match rather than
  only rejecting a misaligned one), multiple non-overlapping matches,
  a full degenerate-input matrix for `scan_line` (empty line, empty
  ratio, zero/negative/NaN tolerance, all-zero ratio), a synthetic
  finder-pattern localization test, a cross-check false-positive
  rejection test (a dark bar with no vertical structure), two
  separated patterns both located, and degenerate `find_pattern_candidates`
  input (empty and ragged bitmaps) proven not to panic.

### Out of scope (documented, not silently dropped)

- Clustering multiple nearby confirmed candidates into one refined
  center per real pattern — deferred to `vis-connected-components`,
  used at the `L3` assembly layer.
- Any specific ratio sequence, or the tolerance a real photographed
  image needs — both caller-supplied parameters.
- Diagonal cross-checks beyond the horizontal/vertical pair.
