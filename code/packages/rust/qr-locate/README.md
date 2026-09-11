# `qr-locate` — the L3 QR-locating consumer

Given a photographed image, find and decode the QR code in it. The
actual motivating goal of `VIS00-vision-roadmap.md`'s whole
computer-vision investment (issue #14456) — see
`code/specs/qr-locate.md` for the full design.

```rust
use pixel_container::PixelContainer;
use qr_locate::locate_and_decode;

let photo: PixelContainer = /* a photographed QR code */;
let bytes = locate_and_decode(&photo)?;
```

One function. No tuning parameters — matches `qr_decoder::decode`'s
own minimal surface; every internal threshold/tolerance is a
documented constant.

## Where this fits

```text
PixelContainer (a photographed image)
  │  adaptive_threshold_mean (IMG09)
  ▼
binarized bitmap
  │  find_pattern_candidates (VIS04)
  ▼
raw finder-pattern candidates
  │  stamp + label_components (VIS02)
  ▼
refined candidate centers
  │  right-isoceles-triangle search
  ▼
3 finder-pattern centers
  │  estimate_affine (VIS03), both chiralities tried
  ▼
3x3 transform
  │  perspective_warp (already existed)
  ▼
straightened image
  │  Otsu threshold + module sampling
  ▼
ModuleGrid
  │  qr_decoder::decode (already existed, MA04)
  ▼
decoded bytes
```

Every step above `perspective_warp` and below `qr_decoder::decode` is
new; both endpoints were already shipped and unchanged.

## Two deliberate scope decisions

- **Affine only, no true perspective correction.** A QR code has 3
  finder patterns; `estimate_homography` needs 4 correspondences. Real
  perspective correction needs a genuinely measured 4th point (the
  alignment pattern, for version ≥ 2) — real, separate future work, not
  faked here by extrapolating a corner.
- **No morphological cleanup.** `VIS02`'s clustering and `VIS04`'s
  ratio tolerance already absorb some binarization noise; morphology
  would help with real photographed sensor noise, but this crate's
  tests are synthetic — no real photograph exists yet to prove that
  need against.

Both are documented in the spec's §2, not silently dropped.

## Errors

```rust
pub enum QrLocateError {
    NoFinderPatternTriangleFound,
    InvalidEstimatedVersion { measured_modules: f64 },
    Decode(Vec<qr_decoder::QrDecodeError>),
}
```

Never panics on any input — degenerate images (blank, tiny, no finder
patterns) return a documented error.

## A real bug this crate's own tests caught

The adaptive-threshold window must scale with the image's own size
(Bradley & Roth's own recommendation), not a fixed pixel count — a QR
finder pattern's solid 3-module center block is a real uniform-dark
region several modules wide, and a window smaller than it gives that
region's own interior a local mean of ~0, so nothing looks "darker
than the mean" there. An unrotated synthetic test caught this
directly: zero finder-pattern candidates found with a fixed 25px
window, before this crate's first push.

A second, subtler one: a rotated finder pattern's horizontal/vertical
scanline crosses it diagonally, not perpendicular to its edges, so the
measured module size carries a real rotation-dependent bias (~4%
observed on a 17°-rotated test) — enough to round the wrong way
entirely. Version estimation now searches every legal QR size directly
and picks the closest match by implied module size, rather than
rounding a single noisy ratio. See the spec's §4.4.

## Testing

```
cargo test -p qr-locate -- --nocapture
```

The synthetic end-to-end tests render a real QR code (via the
already-merged `qr-code` encoder), build a "photo" of it at several
rotations using `perspective_warp` itself with a known transform, and
assert `locate_and_decode` recovers the exact original payload.
