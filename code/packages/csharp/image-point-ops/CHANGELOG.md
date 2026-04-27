# Changelog

All notable changes to `CodingAdventures.ImagePointOps` are recorded here.
The format follows Keep a Changelog and the project uses Semantic Versioning.

## [0.1.0] - 2026-04-20

### Added
- Initial C# port of IMG03 per-pixel point operations.
- u8-domain operations: `Invert`, `Threshold`, `ThresholdLuminance`,
  `Posterize`, `SwapRgbBgr`, `ExtractChannel`, `Brightness`, `Contrast`.
- Linear-light operations: `Gamma`, `Exposure`, `Greyscale` (Rec709/BT.601/
  Average), `Sepia`, `ColourMatrix`, `Saturate`, `HueRotate`.
- sRGB encode/decode utilities: `SrgbToLinearImage`, `LinearToSrgbImage`.
- 1-D LUT plumbing: `ApplyLut1dU8`, `BuildLut1dU8`, `BuildGammaLut`.
- 256-entry sRGB→linear LUT at static init for fast linear-light decodes.
- xUnit test suite with 40+ tests exceeding the 80% coverage threshold.
