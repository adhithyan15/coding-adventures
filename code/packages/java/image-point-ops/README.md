# Java Image Point Ops

IMG03 — per-pixel point operations on `PixelContainer`. Every method in
this package transforms each pixel independently, using only that pixel's
own value — no neighbourhood lookups, no frequency-domain tricks, no
geometry. That orthogonality makes point ops trivially parallelisable and
easy to reason about.

## Two domains

Point operations split cleanly by where they do their arithmetic:

- **u8 domain** — `invert`, `threshold`, `posterize`, channel ops,
  `brightness`. These work directly on the sRGB bytes.
- **Linear-light domain** — `contrast`, `gamma`, `exposure`, `greyscale`,
  `sepia`, `colourMatrix`, `saturate`, `hueRotate`. These decode sRGB to
  linear light first, operate, and re-encode.

Doing colour mixing in sRGB space produces the classic "muddy midtones"
artifact; doing it in linear light is physically correct.

## sRGB ↔ linear

```
decode: c = byte/255; c <= 0.04045 ? c/12.92 : ((c+0.055)/1.055)^2.4
encode: c <= 0.0031308 ? c*12.92 : 1.055*c^(1/2.4)-0.055; round*255
```

A precomputed 256-entry LUT handles the sRGB → linear direction at class
load time, since there are only 256 possible input bytes.

## Usage

```java
import com.codingadventures.imagepointops.ImagePointOps;
import com.codingadventures.imagepointops.GreyscaleMethod;

PixelContainer inverted = ImagePointOps.invert(src);
PixelContainer grey     = ImagePointOps.greyscale(src, GreyscaleMethod.REC709);
PixelContainer brighter = ImagePointOps.exposure(src, +1.0);
```

## Depends on

- `pixel-container` (IC00) — the underlying buffer format.
