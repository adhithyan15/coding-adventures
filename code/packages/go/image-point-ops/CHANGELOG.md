# Changelog — image-point-ops (Go)

## [0.1.0] — 2026-04-19

### Added

- Initial release implementing IMG03 (point operations) over `PixelContainer`.
- **u8-domain operations**: `Invert`, `Threshold`, `ThresholdLuminance`, `Posterize`, `SwapRGBBGR`, `ExtractChannel`, `Brightness`.
- **Linear-light operations**: `Contrast`, `Gamma`, `Exposure`, `Greyscale` (Rec709 / BT601 / Average), `Sepia`, `ColourMatrix`, `Saturate`, `HueRotate`.
- **Colorspace utilities**: `SRGBToLinearImage`, `LinearToSRGBImage`.
- **LUT helpers**: `ApplyLUT1DU8`, `BuildLUT1DU8`, `BuildGammaLUT`.
- Package-level 256-entry `srgbToLinear` decode LUT (built once at `init()`).
- Full test suite covering every exported function.
