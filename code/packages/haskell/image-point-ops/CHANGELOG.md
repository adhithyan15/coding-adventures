# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

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
