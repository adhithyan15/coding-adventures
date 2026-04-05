# ImageCodecBMP

IC01: BMP image encoder/decoder for Swift.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack — layer IC01.

## What It Does

This library encodes and decodes the Windows BMP (Bitmap) image format.
BMP is the simplest well-documented binary image format: a fixed 54-byte
header followed by uncompressed pixel rows. It's ideal for learning binary
file I/O and little-endian byte layout.

We implement the 24-bit BGR variant (no palette, no compression):
- Encoder: `PixelContainer` → BMP bytes (drops alpha, stores BGR)
- Decoder: BMP bytes → `PixelContainer` (synthesises alpha=255, converts BGR→RGB)

## BMP File Layout

```
Offset  Bytes  Field
──────  ─────  ─────────────────────────────────────────────
0       2      Signature "BM"
2       4      File size (LE uint32)
6       4      Reserved (zeros)
10      4      Pixel data offset = 54
14      4      DIB header size = 40
18      4      Width (LE int32)
22      4      Height (LE int32, negative = top-down)
26      2      Color planes = 1
28      2      Bits per pixel = 24
30      4      Compression = 0 (BI_RGB)
34      4      Pixel data size
38      8      X/Y pixels per metre (2835 ≈ 72 DPI)
46      8      Palette info (zeros)
54     ...     Pixel data (BGR, rows padded to 4-byte boundary)
```

## API

```swift
import ImageCodecBMP
import PixelContainer

// Encode
var img = PixelContainer(width: 64, height: 64)
fillPixels(&img, r: 255, g: 128, b: 0, a: 255)  // orange
let bmpBytes = encodeBmp(img)

// Decode
let decoded = try decodeBmp(bmpBytes)

// Via protocol
let codec = BmpCodec()
let bytes  = codec.encode(img)          // "image/bmp"
let pixels = try codec.decode(bytes)
```

## Limitations

- 24-bit BGR only (no 32-bit BGRA, no 8-bit palette, no RLE compression)
- Alpha is dropped on encode and synthesised as 255 on decode
- No progressive or embedded colour profiles

## Where It Fits

```
IC00 PixelContainer   (shared pixel buffer)
      ↓
IC01 ImageCodecBMP    ← you are here
IC02 ImageCodecPPM
IC03 ImageCodecQOI
```

## Running Tests

```bash
swift test
```

## License

MIT
