# qr-locate — the L3 QR-locating consumer

## Status

The L3 application `VIS00-vision-roadmap.md` set out to reach: given a
photographed image, find and decode the QR code in it. Assembles
`IMG09` (adaptive threshold), `VIS04` (pattern scanning), `VIS02`
(connected components), `VIS03` (geometric estimation), and the
already-shipped `image-geometric-transforms::perspective_warp` +
`qr_decoder::decode` — nothing below this layer is new.

## 1. Why this exists

Every layer below this one is real and merged, but nothing connects
them. `qr_decoder::decode` says so explicitly in its own module doc:
"this crate does not locate a QR code inside a photograph... That is
real, separable follow-up work." This crate is that follow-up work —
the actual motivating goal of the whole `VIS` investment (issue
#14456).

```text
PixelContainer (a photographed or otherwise-captured image)
  │  adaptive_threshold_mean (IMG09)                          [L1]
  ▼
binarized bitmap
  │  find_pattern_candidates, ratio [1,1,3,1,1]                [VIS04]
  ▼
raw finder-pattern candidates (many, mostly false positives)
  │  stamp + label_components (VIS02) -- cluster into refined centers
  ▼
refined candidate centers
  │  right-isoceles-triangle search -- pick the 3 real finder patterns
  ▼
3 finder-pattern centers, corner unambiguous, left/right ambiguous
  │  estimate_affine (VIS03, tried both left/right assignments)
  ▼
3x3 affine transform, clean space -> photo space
  │  perspective_warp (already exists)
  ▼
straightened image
  │  Otsu threshold + module-center sampling
  ▼
ModuleGrid
  │  qr_decoder::decode (already exists, MA04)
  ▼
decoded bytes
```

## 2. Scope: affine only, no morphology — both explicit, both documented

**No true perspective (homography) correction.** A QR code has exactly
3 finder patterns; `estimate_homography` needs 4 correspondences. Real
perspective correction needs a genuinely *measured* 4th point — the
correct way to get one is detecting the alignment pattern (present for
version >= 2, itself a `[1,1,1,1,1]`-ratio pattern scannable with the
same `vis_pattern_scanning` primitives, at a version-dependent expected
position). That's real, separable future work
(`qr-locate v2: alignment-pattern-based homography`), not faked here by
extrapolating a 4th "corner" from the other three (which would silently
degrade to an affine fit anyway while claiming to do more). v1 uses
`estimate_affine` — correct for arbitrary in-plane rotation and
reasonably fronto-parallel photos, not steep camera angles.

**No morphology (IMG08).** `VIS02`'s clustering already absorbs
small binarization noise (isolated specks form their own tiny,
low-support components); `VIS04`'s ratio tolerance absorbs some
run-length noise. Real photographed sensor noise may need morphological
cleanup too, but this crate's tests are synthetic/rendered — there's no
real photograph to prove that need against yet. Deferred, documented,
revisited when real-photo testing exists.

## 3. The coordinate convention `perspective_warp` actually needs

`perspective_warp`'s own doc is explicit: `h` maps `[x', y', 1]` in the
*output* plane to `[x, y, 1]` in the *input* plane, in the standard
graphics sense — `x` is the horizontal/column axis, `y` is the
vertical/row axis. `vis_geometric_estimation::estimate_affine`'s point
tuples carry no fixed meaning (its own spec §3.3: "not enforced at the
type level, caller's choice") — but to get an `h` `perspective_warp`
can use directly, without a transpose/swap step, this crate constructs
every point tuple as `(x, y) = (col, row)`, not `(row, col)`, even
though `VIS02`'s `Component` and `VIS04`'s `PatternCandidate` both use
named `row`/`col` fields. Getting this backwards doesn't panic or
error — it silently produces a rotated-90°-and-mirrored transform that
fails to decode, which is exactly the kind of bug the synthetic
end-to-end test (§6) exists to catch.

## 4. Pipeline

### 4.1 Binarize + scan (IMG09, VIS04)

`adaptive_threshold_mean(image, window, t)` → convert to
`Vec<Vec<bool>>` (true = dark) → `find_pattern_candidates(&bitmap,
&[1,1,3,1,1], tolerance)`. Internal constants, not exposed — this
crate's only public entry point takes just the image, matching
`qr_decoder::decode`'s own minimal surface.

`window` is *not* a fixed pixel count — it's `image_size /
ADAPTIVE_WINDOW_DIVISOR` (Bradley & Roth's own recommendation),
clamped to a floor for small images. Caught empirically before this
crate's first push: a QR finder pattern's solid 3-module center block
is a real uniform-dark region several modules wide, and a window
smaller than it gives that region's own interior pixels a local mean
of ~0 — nothing looks "darker than the mean," so the interior
misclassifies as background. A fixed small window (e.g. 25px) breaks
on exactly this shape; scaling with the image's own size keeps the
window bigger than any real QR's finder-pattern block regardless of
how large the photographed code is in frame.

Raw candidates are capped at `MAX_RAW_CANDIDATES` (truncated in scan
order, the same defensive-not-optimal truncation `VIS04`'s own
`MAX_MATCHES_PER_LINE` already established as this workspace's
precedent) before the clustering step, so a pathological/adversarial
bitmap can't make the O(candidates) stamping-and-clustering step scale
with an unbounded VIS04 output.

### 4.2 Cluster into refined centers (VIS02)

Per `VIS04`'s own spec §2: stamp a small square (radius = that
candidate's own estimated `module_size`, rounded) around each raw
candidate onto a fresh same-size bitmap, `label_components` (Eight
connectivity) over it. For each raw candidate, look up which component
its own stamped center landed in; group raw candidates by that
component; a refined center is the mean `(row, col, module_size)` of
its group — averaging the *original measured candidates*, not the
stamped pixels' own centroid, for sub-pixel precision `VIS02`'s
integer-grid centroid alone wouldn't have.

Refined centers are sorted by group size (most-corroborated first) and
capped at `MAX_REFINED_CANDIDATES` before the next step — this is the
cap that actually matters for the O(n³) triangle search below, and
"most raw candidates agree this is a real pattern" is a real
correctness-relevant ranking, not just a DoS-safety truncation.

### 4.3 Select the real 3 finder patterns (geometry)

Real finder-pattern centers form a right isoceles triangle: two legs of
equal length (in pixels) meeting at a right angle (the true top-left
corner), hypotenuse ≈ √2 × leg. Try every 3-combination of refined
candidates and every choice of which vertex is the corner; score by
combined deviation from that geometry (leg-length ratio from 1.0,
|cos(angle)| from 0.0, hypotenuse ratio from √2); keep the best-scoring
triple that clears a tolerance. No triple clears it →
`QrLocateError::NoFinderPatternTriangleFound`.

### 4.4 Estimate QR version

A finder-pattern center sits 3.5 modules from its own edge, so the gap
between the top-left and top-right centers is `N - 7` modules for an
`N`x`N`-module QR symbol. Rather than rounding `leg_px / module_size`
to a module count and checking the result is a legal size (`21 + 4k`,
`k` in `0..40`), search every legal size directly and pick the one
whose *implied* module size (`leg_px / (n - 7)`) is closest to the
measured one, within `VERSION_MATCH_TOLERANCE` — reject with
`QrLocateError::InvalidEstimatedVersion` if even the closest legal
size is a poor match.

This is more than a style choice: a rotated finder pattern's
horizontal/vertical scanline crosses it diagonally, not perpendicular
to its edges, so the measured run-length — and therefore
`module_size` — carries a real, rotation-dependent bias (~4% observed
empirically on a 17°-rotated synthetic test). That's enough to round
`leg_px / module_size` down to the *wrong* module count entirely (13
instead of 14, landing on an illegal size) rather than just missing
"legal" — caught by the synthetic end-to-end test at a non-axis-
aligned angle, not by hand-derivation. Picking the closest legal size
by implied module size is robust to this bias since neighbouring
legal sizes are 4 modules apart, well outside plausible measurement
noise.

### 4.5 Try both chiralities, warp, decode (VIS03 + existing pieces)

The right-angle vertex is unambiguous; which of the other two
candidates is "top-right" vs "bottom-left" is not — swapping them
mirrors the reading. Build both assignments; for each, in `(x, y)` per
§3: `from` = clean points at `(3.5, 3.5)`, `(N-3.5, 3.5)`, `(3.5,
N-3.5)` (all × `MODULE_PIXELS`), `to` = the matching measured centers →
`estimate_affine(from, to)` → cast `f64` → `f32` →
`perspective_warp(image, h, N*MODULE_PIXELS, N*MODULE_PIXELS,
Bilinear, Zero)` → Otsu-threshold the warped image (a small
self-contained helper — global, not adaptive, since the warp has
already normalized scale/rotation; not worth its own `IMG` package) →
sample each module's center pixel → build `ModuleGrid` →
`qr_decoder::decode`. Return the first chirality that decodes
successfully. Both failing → `QrLocateError::Decode(Vec<QrDecodeError>)`
(one entry per attempt).

## 5. API

```rust
pub enum QrLocateError {
    NoFinderPatternTriangleFound,
    InvalidEstimatedVersion { measured_modules: f64 },
    Decode(Vec<qr_decoder::QrDecodeError>),
}

pub fn locate_and_decode(image: &PixelContainer) -> Result<Vec<u8>, QrLocateError>;
```

One function. No tuning parameters exposed — matches `qr_decoder::
decode`'s own minimal surface; every internal threshold/tolerance/cap
is a documented constant, adjustable later from real-photo testing
without an API break.

## 6. Test strategy

- **Synthetic end-to-end**: render a real QR via the already-merged
  `qr-code` encoder + `barcode-2d` layout/render, then build a "photo"
  by applying a *known* forward affine transform (rotation +
  translation) via `perspective_warp` itself — invert the chosen
  forward matrix (via `matrix::Matrix::invert`, a dev-dependency only)
  to get the `h` `perspective_warp` wants, warp with `OutOfBounds::
  Replicate` (the clean render's quiet-zone border is already white,
  so edge-replication gives a plausible uniform background for free,
  no separate compositing step needed). Run `locate_and_decode`,
  assert the recovered bytes match the original payload. Repeated at
  several rotations (0°, 90°, an arbitrary non-axis-aligned angle) to
  exercise the chirality-retry logic for real.
- **Version estimation**: unit test the module-count arithmetic
  directly against known versions.
- **Triangle selection**: unit test §4.3 directly on hand-built
  candidate lists — a real triple among noise candidates is found;
  pure noise returns the documented error, not a panic or a
  wrong-but-plausible pick.
- **Degenerate/failure input**: a blank image, a photo-like image with
  no finder patterns, a tiny image — all documented errors, never a
  panic.
- **Caps actually trigger**: `MAX_RAW_CANDIDATES` and
  `MAX_REFINED_CANDIDATES` are each proven to truncate on a
  constructed input that would otherwise exceed them, the same
  empirical-proof standard `VIS04`'s own DoS tests set.

## 7. Acceptance gates

1. §6's full test matrix is green, including the synthetic end-to-end
   round trip at multiple rotations.
2. No panic on any input shape, including degenerate/adversarial
   bitmaps.
3. `cargo clippy -p qr-locate --all-targets -- -D warnings` clean.
4. Security review passed before push.
