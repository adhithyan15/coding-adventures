# ImageCodecQOI

IC03: QOI (Quite OK Image) encoder/decoder for Swift.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack — layer IC03.

## What It Does

This library encodes and decodes the QOI image format. QOI is a lossless
image format designed for simplicity and speed. Its spec fits on one page.
It achieves 2-4x compression on photographic images and near-PNG compression
on pixel art, while encoding faster than PNG.

Reference spec: <https://qoiformat.org/qoi-specification.pdf>

## QOI File Structure

```
Offset  Bytes  Field
──────  ─────  ─────────────────────────────────────────────────
0       4      Magic "qoif"
4       4      Width  (big-endian uint32)
8       4      Height (big-endian uint32)
12      1      Channels: 4 = RGBA
13      1      Colorspace: 0 = sRGB
14     ...     Compressed chunk stream
end     8      End marker [0,0,0,0,0,0,0,1]
```

## QOI Compression (6 Chunk Types)

| Op           | Bytes | Condition |
|--------------|-------|-----------|
| QOI_OP_RUN   | 1     | px == prev (up to 62 in a row) |
| QOI_OP_INDEX | 1     | px is in the 64-entry hash table |
| QOI_OP_DIFF  | 1     | dr,dg,db ∈ [-2,+1], alpha unchanged |
| QOI_OP_LUMA  | 2     | dg ∈ [-32,+31], dr-dg / db-dg ∈ [-8,+7], alpha unchanged |
| QOI_OP_RGB   | 4     | alpha unchanged (fallback) |
| QOI_OP_RGBA  | 5     | alpha changed (fallback) |

Hash: `(R×3 + G×5 + B×7 + A×11) % 64`

## API

```swift
import ImageCodecQOI
import PixelContainer

// Encode
var img = PixelContainer(width: 256, height: 256)
fillPixels(&img, r: 128, g: 0, b: 255, a: 255)
let qoiBytes = encodeQoi(img)

// Decode
let decoded = try decodeQoi(qoiBytes)

// Via protocol
let codec = QoiCodec()
let bytes  = codec.encode(img)
let pixels = try codec.decode(bytes)
print(codec.mimeType)  // "image/qoi"
```

## Where It Fits

```
IC00 PixelContainer
IC01 ImageCodecBMP
IC02 ImageCodecPPM
IC03 ImageCodecQOI   ← you are here
```

## Running Tests

```bash
swift test
```

## License

MIT
