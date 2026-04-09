# image-codec-ppm (Go)

Netpbm P6 (binary PPM) encoder and decoder implementing `pixelcontainer.ImageCodec`.

## What it does

Encodes a `PixelContainer` to P6 PPM bytes and decodes PPM files back into a
`PixelContainer`. The alpha channel is dropped on encode; decoded pixels get
A=255. Comment lines (starting with `#`) are handled during decode.

## Installation

```
require github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-ppm v0.0.0
replace github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-ppm => ../image-codec-ppm

require github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container v0.0.0
replace github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container => ../pixel-container
```

## Quick start

```go
import (
    ppm "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-ppm"
    pc  "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

img := pc.New(320, 240)
pc.FillPixels(img, 0, 255, 0, 255) // solid green

data := ppm.EncodePpm(img)
img2, err := ppm.DecodePpm(data)
```

## API

| Symbol | Description |
|---|---|
| `PpmCodec{}` | Implements `pc.ImageCodec` |
| `PpmCodec.MimeType()` | Returns `"image/x-portable-pixmap"` |
| `EncodePpm(c *pc.PixelContainer) []byte` | Encode to P6 PPM (alpha dropped) |
| `DecodePpm(data []byte) (*pc.PixelContainer, error)` | Decode P6 PPM (A set to 255) |
| `IsPpm(data []byte) bool` | Magic-number detection |

## PPM file format

```
P6\n
<width> <height>\n
255\n
<RGB bytes — 3 bytes per pixel, row-major>
```

Comment lines beginning with `#` may appear anywhere in the header.

## How it fits in the stack

```
pixel-container   (IC00)
      ↑
image-codec-ppm   ← you are here (IC02)
```
