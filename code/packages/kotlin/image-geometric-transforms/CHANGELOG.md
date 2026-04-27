# Changelog

## 0.1.0 — Initial release

- Lossless transforms: `flipHorizontal`, `flipVertical`, `rotate90CW`,
  `rotate90CCW`, `rotate180`, `crop`.
- Continuous transforms: `scale`, `rotate`, `translate`, `affine`,
  `perspectiveWarp`.
- Enums: `Interpolation` (NEAREST/BILINEAR/BICUBIC),
  `RotateBounds` (FIT/CROP), `OutOfBounds` (ZERO/REPLICATE/REFLECT/WRAP).
- Bilinear and bicubic resampling in linear light via a 256-entry sRGB→
  linear LUT; Catmull-Rom (B=0, C=0.5) kernel for bicubic.
- Inverse-warp, pixel-centre sampling model.
- `invert3x3` adjugate-method matrix inverse helper.
