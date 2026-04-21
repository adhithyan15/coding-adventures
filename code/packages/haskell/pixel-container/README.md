# pixel-container (Haskell)

IC00 — Universal RGBA8 pixel buffer for the coding-adventures image stack.

## Overview

`pixel-container` is the zero-dependency foundation every image codec and
processing stage in the coding-adventures image pipeline depends on.  It
provides a fixed-format RGBA8 buffer (4 channels, 8 bits each) stored row-major
with the top-left origin, plus an abstract `ImageCodec` type class used by
codec packages (PNG, BMP, PPM, …).

By fixing the pixel format at the lowest layer we avoid a combinatorial
explosion of format conversions higher up: point operations only need to
reason about RGBA8, and geometric transforms only need to know how to sample
and write back RGBA8 pixels.

## Usage

```haskell
import PixelContainer

main :: IO ()
main = do
    let pc  = createPixelContainer 4 4
        pc' = setPixel pc 1 2 255 0 0 255
    print (pixelAt pc' 1 2)  -- (255, 0, 0, 255)
```

## API

- `data PixelContainer` — record with `pcWidth`, `pcHeight`, `pcPixels` (a
  flat `ByteString` of `w * h * 4` bytes).
- `createPixelContainer :: Int -> Int -> PixelContainer` — new, fully
  transparent-black buffer.
- `pixelAt :: PixelContainer -> Int -> Int -> (Word8, Word8, Word8, Word8)` —
  read a pixel; out-of-bounds returns `(0,0,0,0)`.
- `setPixel :: PixelContainer -> Int -> Int -> Word8 -> Word8 -> Word8 -> Word8
   -> PixelContainer` — write a pixel; out-of-bounds is a no-op.
- `fillPixels :: PixelContainer -> Word8 -> Word8 -> Word8 -> Word8
   -> PixelContainer` — set every pixel to a colour.
- `class ImageCodec` — `mimeType`, `encode`, `decode` contract for codec
  packages.

## Layout

```
offset = (y * width + x) * 4
data[offset + 0] = R
data[offset + 1] = G
data[offset + 2] = B
data[offset + 3] = A
```
