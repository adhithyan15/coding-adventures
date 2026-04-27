# Changelog

## 0.1.0 — Initial release

- u8-domain point ops: `invert`, `threshold`, `thresholdLuminance`,
  `posterize`, `swapRgbBgr`, `extractChannel`, `brightness`, `contrast`.
- Linear-light point ops: `gamma`, `exposure`, `greyscale`, `sepia`,
  `colourMatrix`, `saturate`, `hueRotate`, `srgbToLinearImage`,
  `linearToSrgbImage`.
- `GreyscaleMethod` enum (REC709 / BT601 / AVERAGE).
- LUT helpers: `buildLut1dU8`, `buildGammaLut`, `applyLut1dU8`.
- 256-entry sRGB→linear precomputed LUT at class-load time.
