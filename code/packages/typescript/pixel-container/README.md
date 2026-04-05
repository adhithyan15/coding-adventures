# @coding-adventures/pixel-container

IC00: Universal RGBA8 pixel buffer and image codec interface.

Zero runtime dependencies. The foundation for all image codecs in the IC series
(`image-codec-bmp`, `image-codec-ppm`, `image-codec-qoi`, ...).

## What it is

A `PixelContainer` is a flat `Uint8Array` of RGBA8 pixels stored row-major:

```
offset = (y * width + x) * 4
data[offset + 0] = R
data[offset + 1] = G
data[offset + 2] = B
data[offset + 3] = A
```

An `ImageCodec` converts between a `PixelContainer` and raw file bytes.

## Usage

```typescript
import {
  createPixelContainer,
  setPixel,
  pixelAt,
  fillPixels,
  type ImageCodec,
} from "@coding-adventures/pixel-container";

const c = createPixelContainer(4, 4);
fillPixels(c, 255, 255, 255, 255);   // solid white
setPixel(c, 1, 1, 255, 0, 0, 255);  // red dot at (1,1)

const [r, g, b, a] = pixelAt(c, 1, 1); // [255, 0, 0, 255]
```

## API

| Export | Description |
|--------|-------------|
| `PixelContainer` | Interface: `{ width, height, data: Uint8Array }` |
| `ImageCodec` | Interface: `{ mimeType, encode, decode }` |
| `createPixelContainer(w, h)` | Factory — zeroed RGBA8 buffer |
| `pixelAt(c, x, y)` | Read pixel → `[r, g, b, a]`; `[0,0,0,0]` if OOB |
| `setPixel(c, x, y, r, g, b, a)` | Write pixel; no-op if OOB |
| `fillPixels(c, r, g, b, a)` | Flood fill entire buffer |

## Relationship to paint-instructions

`paint-instructions` re-exports `PixelContainer` and `ImageCodec` from this
package so that existing `import { PixelContainer } from '@coding-adventures/paint-instructions'`
imports continue to work without any changes.
