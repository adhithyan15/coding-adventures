# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-10

### Added

- Initial implementation of two-pass union-find connected-component
  labeling over a binary image, closing `VIS00-vision-roadmap.md`'s
  L2 gap: turning "a blob of dark pixels" into "one object with a
  bounding box, a centroid, and a pixel count," the shape a
  finder-pattern candidate needs to be in before `VIS04` (pattern
  scanning, not yet built) can cluster and verify it.
- `label_components(bitmap: &[Vec<bool>], connectivity: Connectivity) -> Labeling`
  — the crate's one public function.
- `Connectivity::{Four, Eight}` — caller-supplied, not hardcoded; which
  is correct depends on what the caller is trying to detect.
- `Component` (label, pixel count, inclusive bounding box, centroid)
  and `Labeling` (per-pixel label grid + one `Component` per blob).
- Never panics on any bitmap shape or content — empty, ragged (rows of
  differing length), all-background, all-foreground.
- 11 unit tests + 1 doc-test: known shapes (isolated pixel, solid
  rectangle with exact expected stats, two separated rectangles, a
  U-shape requiring two passes to resolve correctly), a connectivity
  test proving `Four` and `Eight` produce different component counts
  on the same diagonal-touch input (not just that both calls succeed),
  empty/all-background/all-foreground/ragged degenerate inputs, and a
  realistic-scale (40x40, several irregular blobs) case. Every test
  independently recomputes each `Component`'s stats from the returned
  label grid itself and asserts they match, proving `labels` and
  `components` are actually consistent with each other.

### Out of scope (documented, not silently dropped)

- Which `Connectivity` a specific downstream use case should pick.
- Blob shape/size filtering — the caller's job, not this crate's.
