# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-11

### Added

- Initial implementation: `locate_and_decode(image: &PixelContainer)
  -> Result<Vec<u8>, QrLocateError>` — the L3 application
  `VIS00-vision-roadmap.md` set out to reach (issue #14456). Assembles
  `IMG09` (adaptive threshold), `VIS04` (pattern scanning), `VIS02`
  (connected components), `VIS03` (geometric estimation), and the
  already-shipped `perspective_warp` + `qr_decoder::decode` — nothing
  below this layer is new.
- Pipeline: binarize → scan for finder-pattern candidates → cluster
  into refined centers (stamp-and-label via VIS02, per VIS04's own
  spec §2 recommendation) → select the real 3 via a right-isoceles-
  triangle search → estimate QR version from finder spacing → try both
  possible left/right chiralities → affine-fit (VIS03) → warp →
  Otsu-threshold → sample module centers → decode.
- `QrLocateError::{NoFinderPatternTriangleFound, InvalidEstimatedVersion,
  Decode}` — never panics; every degenerate/failure case is a
  documented error.
- `MAX_RAW_CANDIDATES` / `MAX_REFINED_CANDIDATES` — two-tier caps
  bounding the O(candidates) clustering step and the O(n^3) triangle
  search against a pathological/adversarial bitmap, applying this
  session's established DoS-capping discipline from the start rather
  than as a follow-up fix.
- 19 tests (9 inline unit tests for the private triangle-selection/
  version-estimation/clustering helpers + 9 integration tests + 1
  doctest), including a synthetic end-to-end round trip (render a real
  QR via the `qr-code` encoder, photograph-simulate it via
  `perspective_warp` at 0°/90°/180°/17° rotations, recover the exact
  original payload) and a dense-pathological-input test proving the
  caps keep cost bounded.

### Fixed (caught by the crate's own tests, before this ever shipped)

- The adaptive-threshold window must scale with the image's own size
  (`image_size / 8`, Bradley & Roth's own recommendation), not a fixed
  pixel count — a QR finder pattern's solid 3-module center block is a
  real uniform-dark region several modules wide; a window smaller than
  it gives that region's own interior a local mean of ~0, making
  nothing look "darker than the mean." An unrotated synthetic test
  found zero candidates with a fixed 25px window before this was
  caught.
- `otsu_threshold`'s tie-break picked the *first* threshold value
  achieving maximum between-class variance, not the midpoint of the
  tied plateau. On a cleanly bimodal image (no anti-aliasing at all,
  e.g. a QR straightened by a near-integer scale factor) the variance
  ties across the entire gap between the two clusters, and picking the
  low end (as small as `0`) is a real bug: `threshold_luminance`
  classifies a pixel dark iff its luminance is strictly less than the
  threshold, so `t=0` makes every pixel light. Fixed to pick the
  midpoint of the tied plateau.
- Version estimation (§4.4) originally rounded `leg_px / module_size`
  to a module count and checked the result was a legal QR size. A
  rotated finder pattern's scanline-measured module size carries a
  real, rotation-dependent bias (a horizontal/vertical scanline
  crosses a rotated pattern diagonally, not perpendicular to its
  edges) — ~4% observed on a 17°-rotated synthetic test, enough to
  round to the *wrong* module count rather than just miss "legal."
  Fixed to search every legal QR size directly and pick the one whose
  implied module size is closest to the measurement, robust to this
  bias since neighbouring legal sizes are 4 modules apart.

### Out of scope (documented, not silently dropped)

- True perspective (homography) correction — needs a genuinely
  measured 4th point (the alignment pattern, version ≥ 2); deferred as
  `qr-locate v2`, not faked by extrapolating a corner.
- Morphological cleanup (`IMG08`) of the pre-scan binarization — no
  real photograph exists yet to prove the need against; this crate's
  tests are synthetic/rendered.
