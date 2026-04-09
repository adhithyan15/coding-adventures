# pixel-container (Go)

A fixed RGBA8 pixel buffer and the `ImageCodec` interface shared by all
image-format packages in this monorepo.

## What it does

`PixelContainer` holds a raw RGBA byte array in row-major order. It is the
single common currency between image codecs: a BMP decoder produces one,
a QOI encoder consumes one, and your rendering code never needs to know which
format came off disk.

The `ImageCodec` interface lets you write generic image-processing code that
works with any supported format without importing the format package directly.

## Installation

This package uses Go modules with a `replace` directive. Add it to your
`go.mod`:

```
require github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container v0.0.0
replace github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container => ../pixel-container
```

## Quick start

```go
import pc "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"

// Create a 320×240 blank canvas
img := pc.New(320, 240)

// Paint pixel (10, 20) solid red
pc.SetPixel(img, 10, 20, 255, 0, 0, 255)

// Read it back
r, g, b, a := pc.PixelAt(img, 10, 20)
// r=255, g=0, b=0, a=255

// Fill the entire canvas with white
pc.FillPixels(img, 255, 255, 255, 255)
```

## API

| Function / Type | Description |
|---|---|
| `PixelContainer` | Struct holding Width, Height, and flat RGBA Data slice |
| `ImageCodec` | Interface: `MimeType() string`, `Encode(*PC) []byte`, `Decode([]byte) (*PC, error)` |
| `New(w, h uint32) *PixelContainer` | Allocate a zeroed pixel buffer |
| `PixelAt(c, x, y) (r,g,b,a byte)` | Read one pixel; returns (0,0,0,0) if out of bounds |
| `SetPixel(c, x, y, r,g,b,a)` | Write one pixel; no-op if out of bounds |
| `FillPixels(c, r,g,b,a)` | Fill every pixel with one colour |
| `Validate(c) error` | Check internal consistency of Data length |

## Memory layout

```
offset(x, y) = (y * Width + x) * 4

Data[offset]   = R
Data[offset+1] = G
Data[offset+2] = B
Data[offset+3] = A
```

## How it fits in the stack

```
pixel-container   ← you are here (IC00)
      ↑
image-codec-bmp   (IC01)
image-codec-ppm   (IC02)
image-codec-qoi   (IC03)
```
