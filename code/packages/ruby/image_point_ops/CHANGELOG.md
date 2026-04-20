# Changelog — coding-adventures-image-point-ops (Ruby)

## [0.1.0] — 2026-04-19

### Added

- Initial release implementing IMG03 (point operations) over `PixelContainer`.
- **u8-domain operations**: `invert`, `threshold`, `threshold_luminance`, `posterize`, `swap_rgb_bgr`, `extract_channel`, `brightness`.
- **Linear-light operations**: `contrast`, `gamma`, `exposure`, `greyscale` (Rec. 709 / BT. 601 / average), `sepia`, `colour_matrix`, `saturate`, `hue_rotate`.
- **Colorspace utilities**: `srgb_to_linear_image`, `linear_to_srgb_image`.
- **LUT helpers**: `apply_lut1d_u8`, `build_lut1d_u8`, `build_gamma_lut`.
- Module-level 256-entry `SRGB_TO_LINEAR` decode LUT (built once at load).
- Full test suite (30 tests) with Minitest.
