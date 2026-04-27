# Changelog — image-point-ops

All notable changes to this crate are recorded here.
Dates in YYYY-MM-DD format.

---

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
