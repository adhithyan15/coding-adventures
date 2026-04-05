# image-codec-bmp (Go)

BMP (Bitmap) encoder and decoder implementing `pixelcontainer.ImageCodec`.

## What it does

Encodes a `PixelContainer` to standard 32-bit BGRA BMP bytes and decodes BMP
files back into a `PixelContainer`. Supports both top-down (negative biHeight)
and bottom-up (positive biHeight) BMP files on decode.

## Installation

```
require github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-bmp v0.0.0
replace github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-bmp => ../image-codec-bmp

require github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container v0.0.0
replace github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container => ../pixel-container
```

## Quick start

```go
import (
    bmp "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-bmp"
    pc  "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

img := pc.New(320, 240)
pc.FillPixels(img, 255, 0, 0, 255) // solid red

// Encode to BMP bytes
data := bmp.EncodeBmp(img)

// Decode back
img2, err := bmp.DecodeBmp(data)
```

## API

| Symbol | Description |
|---|---|
| `BmpCodec{}` | Implements `pc.ImageCodec` |
| `BmpCodec.MimeType()` | Returns `"image/bmp"` |
| `EncodeBmp(c *pc.PixelContainer) []byte` | Encode to 32-bit BGRA BMP |
| `DecodeBmp(data []byte) (*pc.PixelContainer, error)` | Decode 32-bit BI_RGB BMP |
| `IsBmp(data []byte) bool` | Magic-number detection |
| `LookupByMime(mime string) pc.ImageCodec` | Codec registry lookup |

## BMP file layout

```
Offset  Size  Field
------  ----  -----
0       2     Magic "BM"
2       4     File size (bytes)
6       4     Reserved (0)
10      4     Pixel data offset (54)
14      4     Info header size (40)
18      4     Width (pixels)
22      4     Height (negative = top-down)
26      2     Planes (1)
28      2     Bit count (32)
30      4     Compression (0 = BI_RGB)
34      20    (zeros)
54      …     Pixel data, BGRA order
```

## How it fits in the stack

```
pixel-container   (IC00)
      ↑
image-codec-bmp   ← you are here (IC01)
```
