# image-codec-qoi (Go)

QOI (Quite OK Image) encoder and decoder implementing `pixelcontainer.ImageCodec`.

## What it does

Encodes a `PixelContainer` to QOI bytes and decodes QOI files back into a
`PixelContainer`. All six QOI opcodes are supported: OP_INDEX, OP_DIFF,
OP_LUMA, OP_RUN, OP_RGB, and OP_RGBA.

## Installation

```
require github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-qoi v0.0.0
replace github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-qoi => ../image-codec-qoi

require github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container v0.0.0
replace github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container => ../pixel-container
```

## Quick start

```go
import (
    qoi "github.com/adhithyan15/coding-adventures/code/packages/go/image-codec-qoi"
    pc  "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

img := pc.New(320, 240)
pc.FillPixels(img, 0, 0, 255, 255) // solid blue

data := qoi.EncodeQoi(img)
img2, err := qoi.DecodeQoi(data)
```

## API

| Symbol | Description |
|---|---|
| `QoiCodec{}` | Implements `pc.ImageCodec` |
| `QoiCodec.MimeType()` | Returns `"image/x-qoi"` |
| `EncodeQoi(c *pc.PixelContainer) []byte` | Encode to QOI |
| `DecodeQoi(data []byte) (*pc.PixelContainer, error)` | Decode QOI |
| `IsQoi(data []byte) bool` | Magic-number detection |

## QOI file layout

```
Offset  Size  Field
------  ----  -----
0       4     Magic "qoif"
4       4     Width (big-endian)
8       4     Height (big-endian)
12      1     Channels (4 = RGBA)
13      1     Colorspace (0 = sRGB)
14      …     Compressed stream (OP_INDEX / OP_DIFF / OP_LUMA / OP_RUN / OP_RGB / OP_RGBA)
-8      8     End marker: 0x00*7 + 0x01
```

## Opcode summary

| Opcode | Bytes | When used |
|---|---|---|
| OP_INDEX | 1 | Pixel matches the colour index (hash table) |
| OP_DIFF | 1 | ΔR,ΔG,ΔB ∈ [−2,+1], ΔA=0 |
| OP_LUMA | 2 | ΔG ∈ [−32,+31], (ΔR−ΔG),(ΔB−ΔG) ∈ [−8,+7], ΔA=0 |
| OP_RUN | 1 | Same pixel repeated (up to 62 times) |
| OP_RGB | 4 | A unchanged, large colour delta |
| OP_RGBA | 5 | A changed, large delta |

## How it fits in the stack

```
pixel-container   (IC00)
      ↑
image-codec-qoi   ← you are here (IC03)
```
