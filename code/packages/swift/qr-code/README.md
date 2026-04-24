# qr-code (Swift)

ISO/IEC 18004:2015 compliant QR Code encoder in Swift.

## What it does

Encodes any UTF-8 string into a scannable QR Code symbol. Returns a
`Barcode2D.ModuleGrid` — a 2D boolean grid (`true` = dark module) ready to be
passed to `Barcode2D.layout()` for pixel rendering or directly consumed by any
downstream package.

## Where it fits in the stack

```
input string
  → QrCode.encode()           ← THIS PACKAGE
  → ModuleGrid (Barcode2D)
  → layout() (Barcode2D)
  → PaintScene (PaintInstructions)
  → paint backend (SVG, Metal, etc.)
```

## Usage

```swift
import QrCode

// Minimal — defaults to .medium error correction
let grid = try QrCode.encode("https://example.com")
// grid.rows == grid.cols == 29 for this input at M level

// Custom ECC level
let robustGrid = try QrCode.encode("HELLO WORLD", level: .quartile)

// Render with Barcode2D
import Barcode2D
let scene = try layout(grid: grid)  // PaintScene ready for rendering
```

## Error correction levels

| Level       | Recovery | Use case                               |
|-------------|----------|----------------------------------------|
| `.low`      | ~7%      | Maximum data density                   |
| `.medium`   | ~15%     | General purpose (default)              |
| `.quartile` | ~25%     | Moderate damage, decorative overlays   |
| `.high`     | ~30%     | Heavy damage, logo on top of code      |

## Encoding modes

The encoder automatically selects the most compact mode:

- **Numeric** (`0-9` only): 10 bits per 3 digits
- **Alphanumeric** (`0-9 A-Z $%*+-./:` and space): 11 bits per 2 chars
- **Byte** (any UTF-8): 8 bits per byte

## Capacity limits

- Maximum input: 7,089 numeric characters or 2,953 bytes
- Symbol versions 1–40 are supported
- Version is selected automatically

## Errors

- `QrCodeError.inputTooLong` — input exceeds version-40 capacity
- `QrCodeError.encodingError` — internal precondition violated (should not occur for valid UTF-8)

## Implementation notes

The encoder does NOT use the `ReedSolomon` package from this repo. That package
uses the b=1 root convention (roots α^1, α^2, …), whereas QR Code requires the
b=0 convention (roots α^0=1, α^1, …). This file embeds a minimal LFSR-based RS
encoder with the correct QR generator polynomials directly.

## Dependencies

- `Barcode2D` — `ModuleGrid` type and layout utilities
- `GF256` — GF(2^8) arithmetic (log/antilog tables) for RS encoding
