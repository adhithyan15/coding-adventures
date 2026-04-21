# Changelog

## 0.1.0

- Add initial Java `image-geometric-transforms` package (IMG04)
- Lossless: `flipHorizontal`, `flipVertical`, `rotate90CW`, `rotate90CCW`,
  `rotate180`, `crop`
- Continuous: `scale`, `rotate` (FIT/CROP), `translate`, `affine`,
  `perspectiveWarp`
- Three interpolation modes (NEAREST, BILINEAR, BICUBIC Catmull-Rom)
- Four out-of-bounds modes (ZERO, REPLICATE, REFLECT, WRAP)
- Linear-light colour blending; u8 alpha blending
