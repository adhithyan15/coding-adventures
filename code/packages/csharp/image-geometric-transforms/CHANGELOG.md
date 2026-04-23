# Changelog

All notable changes to `CodingAdventures.ImageGeometricTransforms` are
recorded here. Follows Keep a Changelog and Semantic Versioning.

## [0.1.0] - 2026-04-20

### Added
- Initial C# port of IMG04 geometric transforms.
- Lossless: `FlipHorizontal`, `FlipVertical`, `Rotate90CW`, `Rotate90CCW`,
  `Rotate180`, `Crop`.
- Continuous: `Scale`, `Rotate`, `Translate`, `Affine`, `PerspectiveWarp`.
- Enums: `Interpolation` (Nearest/Bilinear/Bicubic), `RotateBounds` (Fit/Crop),
  `OutOfBounds` (Zero/Replicate/Reflect/Wrap).
- Catmull-Rom bicubic kernel, pixel-centre scale convention, inverse-warp
  dispatch for all continuous transforms.
- RGB sampling in linear light; alpha preserved in u8 space.
- xUnit test suite with 40+ tests exceeding the 80% coverage threshold.
