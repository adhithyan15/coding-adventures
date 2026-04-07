# ImageCodecPPM

IC02: PPM (Portable Pixmap) image encoder/decoder for Swift.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack — layer IC02.

## What It Does

This library encodes and decodes the P6 binary PPM image format.
PPM is part of the Netpbm toolkit and uses a human-readable ASCII header
followed by raw binary pixel data. It's ideal for learning how text and
binary data coexist in file formats.

- Encoder: `PixelContainer` → PPM bytes (drops alpha, stores raw RGB)
- Decoder: PPM bytes → `PixelContainer` (synthesises alpha=255, parses ASCII header)

## PPM File Format

```
P6
# optional comment lines
<width> <height>
255
<binary pixel data: width × height × 3 bytes, R G B per pixel>
```

Key properties:
- ASCII header, binary pixel data
- RGB order (not BGR like BMP)
- No row padding (unlike BMP)
- No alpha channel
- No compression

## API

```swift
import ImageCodecPPM
import PixelContainer

// Encode
var img = PixelContainer(width: 64, height: 64)
fillPixels(&img, r: 0, g: 128, b: 255, a: 255)  // sky blue
let ppmBytes = encodePpm(img)

// Decode
let decoded = try decodePpm(ppmBytes)

// Via protocol
let codec = PpmCodec()
let bytes  = codec.encode(img)
let pixels = try codec.decode(bytes)
print(codec.mimeType)  // "image/x-portable-pixmap"
```

## Parsing Strategy

The decoder uses a cursor-based parser: a single integer index advances
through the byte array as tokens are consumed. Comment lines (`# …`) are
skipped inline. No intermediate string allocation is needed for the header.

## Limitations

- P6 (binary) only — not P3 (ASCII pixel data)
- maxval = 255 only (no 16-bit P6)
- Alpha is dropped on encode and synthesised as 255 on decode

## Where It Fits

```
IC00 PixelContainer
IC01 ImageCodecBMP
IC02 ImageCodecPPM   ← you are here
IC03 ImageCodecQOI
```

## Running Tests

```bash
swift test
```

## License

MIT
