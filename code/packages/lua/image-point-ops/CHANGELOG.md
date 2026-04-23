# Changelog — coding-adventures-image-point-ops (Lua)

## [0.1.0] — 2026-04-19

### Added

- Initial release implementing IMG03 (point operations) over `pixel_container`.
- **u8-domain operations**: `invert`, `threshold`, `threshold_luminance`, `posterize`, `swap_rgb_bgr`, `extract_channel`, `brightness`.
- **Linear-light operations**: `contrast`, `gamma`, `exposure`, `greyscale` (rec709 / bt601 / average), `sepia`, `colour_matrix`, `saturate`, `hue_rotate`.
- **Colorspace utilities**: `srgb_to_linear_image`, `linear_to_srgb_image`.
- **LUT helpers**: `apply_lut1d_u8`, `build_lut1d_u8`, `build_gamma_lut`.
- Module-level 256-entry `SRGB_TO_LINEAR` decode table (built once at load).
- Full Busted test suite covering every exported function.
