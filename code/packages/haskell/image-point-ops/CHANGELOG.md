# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 — 2026-04-20

- Initial release of IMG03 point operations for Haskell.
- Index-remapping: `invert`, `threshold`, `thresholdLuminance`,
  `posterize`, `swapRgbBgr`, `extractChannel`.
- Tone: `brightness`, `contrast`, `gamma`, `exposure`.
- Colour: `greyscale` (Rec709 / BT.601 / Average), `sepia`,
  `colourMatrix`, `saturate`, `hueRotate`.
- sRGB <-> linear: `srgbToLinearImage`, `linearToSrgbImage`.
- 1D LUTs: `applyLut1dU8`, `buildLut1dU8`, `buildGammaLut`.
- Hspec test suite.
