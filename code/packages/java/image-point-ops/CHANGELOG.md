# Changelog

## 0.1.0

- Add initial Java `image-point-ops` package (IMG03)
- Implement u8-domain ops: `invert`, `threshold`, `thresholdLuminance`,
  `posterize`, `swapRgbBgr`, `extractChannel`, `brightness`
- Implement linear-light ops: `contrast`, `gamma`, `exposure`, `greyscale`,
  `sepia`, `colourMatrix`, `saturate`, `hueRotate`
- Provide sRGB/linear whole-image converters and 1D LUT helpers
- Precompute sRGB → linear LUT at class load
