# PixelContainer

IC00: Fixed RGBA8 pixel buffer — the shared data type for the image codec stack.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack — layer IC00.

## What It Does

`PixelContainer` is a simple, flat in-memory image buffer. It stores pixels as
raw RGBA bytes in row-major order: 4 bytes per pixel (Red, Green, Blue, Alpha),
`width × height` pixels total.

Every image codec in this stack (BMP, PPM, QOI, PNG, …) encodes FROM or decodes
TO a `PixelContainer`. This separation of concerns means:

- Format-specific code only has to deal with bytes ↔ PixelContainer.
- Pipeline composition is trivial: decode one format, encode another.

## Memory Layout

```
Byte offset for pixel (x, y) = (y × width + x) × 4

data[offset + 0]  = Red
data[offset + 1]  = Green
data[offset + 2]  = Blue
data[offset + 3]  = Alpha   (0 = transparent, 255 = opaque)
```

Row-major means all pixels on the same row are contiguous in memory —
the natural order for C arrays, most file formats, and GPU uploads.

## API

```swift
import PixelContainer

// Create a 64×32 transparent-black buffer
var img = PixelContainer(width: 64, height: 32)

// Set individual pixels
setPixel(&img, x: 10, y: 5, r: 255, g: 0, b: 0, a: 255)   // red dot

// Read pixels
let (r, g, b, a) = pixelAt(img, x: 10, y: 5)               // (255, 0, 0, 255)

// Fill entire buffer
fillPixels(&img, r: 0, g: 0, b: 255, a: 255)               // solid blue

// Raw byte access
let offset = (5 * Int(img.width) + 10) * 4
let redByte = img.data[offset]
```

## ImageCodec Protocol

Any type that handles a specific image format should conform to `ImageCodec`:

```swift
public protocol ImageCodec {
    var mimeType: String { get }
    func encode(_ pixels: PixelContainer) -> [UInt8]
    func decode(_ bytes: [UInt8]) throws -> PixelContainer
}
```

## Where It Fits

```
IC00 PixelContainer   ← you are here (the pixel buffer type)
      ↓
IC01 ImageCodecBMP    (BMP encoder/decoder)
IC02 ImageCodecPPM    (PPM encoder/decoder)
IC03 ImageCodecQOI    (QOI encoder/decoder)
```

## Running Tests

```bash
swift test
```

## License

MIT
