# Changelog — @coding-adventures/image-point-ops

## [0.1.0] — 2026-04-19

### Added

- Initial release implementing IMG03 (point operations) over `PixelContainer`.
- **u8-domain operations**: `invert`, `threshold`, `thresholdLuminance`, `posterize`, `swapRgbBgr`, `extractChannel`, `brightness`.
- **Linear-light operations**: `contrast`, `gamma`, `exposure`, `greyscale` (Rec. 709 / BT. 601 / average), `sepia`, `colourMatrix`, `saturate`, `hueRotate`.
- **Colorspace utilities**: `srgbToLinearImage`, `linearToSrgbImage`.
- **LUT helpers**: `applyLut1dU8`, `buildLut1dU8`, `buildGammaLut`.
- Pre-built 256-entry `SRGB_TO_LINEAR` Float32Array decode LUT (module-level, built once).
- Full test suite with >90% line coverage.
