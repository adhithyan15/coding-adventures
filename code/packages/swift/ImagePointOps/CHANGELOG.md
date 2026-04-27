# Changelog — ImagePointOps (Swift)

## [0.1.0] — 2026-04-19

### Added

- Initial release implementing IMG03 (point operations) over `PixelContainer`.
- **u8-domain operations**: `invert`, `threshold`, `thresholdLuminance`, `posterize`, `swapRGBBGR`, `extractChannel`, `brightness`.
- **Linear-light operations**: `contrast`, `gamma`, `exposure`, `greyscale` (rec709 / bt601 / average), `sepia`, `colourMatrix`, `saturate`, `hueRotate`.
- **Colorspace utilities**: `srgbToLinearImage`, `linearToSRGBImage`.
- **LUT helpers**: `applyLUT1DU8`, `buildLUT1DU8`, `buildGammaLUT`.
- Module-level 256-entry `srgbToLinear` decode LUT (built once at module load).
- Full XCTest suite covering every public function.
