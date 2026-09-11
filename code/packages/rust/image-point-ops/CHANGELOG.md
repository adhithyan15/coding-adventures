# Changelog — image-point-ops

All notable changes to this crate are recorded here.
Dates in YYYY-MM-DD format.

---

## [0.2.0] — 2026-09-11

### Added

- `adaptive_threshold_mean` (IMG09) — locally-adaptive threshold via the
  Bradley/Roth integral-image method: a pixel is foreground when its
  luminance falls more than a fraction `t` below the mean luminance of a
  `window` x `window` neighbourhood around it, computed in O(1) per pixel
  via a summed-area table. Unlike `threshold`/`threshold_luminance`'s
  single scalar cutoff, this survives uneven lighting across an image
  (e.g. a shadow across half a photographed document). See
  `IMG09-adaptive-threshold.md`.
- 5 new unit tests, including a test that first proves no single global
  cutoff can solve a deliberately-constructed two-region image (before
  showing the adaptive version does), an edges-don't-panic check, and a
  full degenerate-input matrix (0-size images, `window = 0`, non-finite
  `t`).

### Fixed (caught by the tests above, before this ever shipped)

- The naive left-to-right `br - tr - bl + tl` summed-area-table rectangle
  formula can panic on unsigned subtraction overflow at an intermediate
  step even though the fully-combined value is always non-negative;
  reordered to add both positive terms first, subtract once.

## [0.1.0] — 2026-04-19

### Added

- Initial release implementing IMG03 (point operations) over `PixelContainer`.
- **u8-domain operations** (work directly on sRGB bytes, no colour-space round-trip):
  - `invert` — negate RGB channels, preserve alpha.
  - `threshold` / `threshold_luminance` — hard binarise on average or Rec. 709 luma.
  - `posterize` — reduce each channel to N equally-spaced levels.
  - `swap_rgb_bgr` — swap R and B channels (BGR↔RGB conversion).
  - `extract_channel` — zero out all channels except the nominated one.
  - `brightness` — additive offset clamped to [0, 255].
- **Linear-light operations** (decode sRGB → f32, operate, re-encode to sRGB u8):
  - `contrast` — scale around mid-grey (0.5 linear).
  - `gamma` — per-channel γ power law.
  - `exposure` — multiply by 2^stops.
  - `greyscale` — Rec. 709, BT. 601, or channel-average luminance.
  - `sepia` — classic warm sepia tone matrix.
  - `colour_matrix` — arbitrary 3×3 RGB matrix (pass-through of alpha).
  - `saturate` — scale saturation 0 (greyscale) → 1 (identity) → 2 (vivid).
  - `hue_rotate` — rotate hue by degrees via HSV.
- **Colorspace utilities**: `srgb_to_linear_image`, `linear_to_srgb_image`.
- **LUT helpers**: `apply_lut1d_u8`, `build_lut1d_u8`, `build_gamma_lut`.
- Lazy-initialised 256-entry `SRGB_TO_LINEAR` decode LUT (built once, reused everywhere).
- Unit tests covering every public function.
