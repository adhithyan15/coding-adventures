# qr-code (Kotlin)

QR Code encoder — ISO/IEC 18004:2015 compliant.

Encodes any UTF-8 string into a scannable QR Code symbol.  Outputs a
`ModuleGrid` (abstract boolean grid) that can be passed to `barcode-2d`'s
`layout()` for pixel rendering, or used directly to produce SVG/PNG/canvas
output through any PaintVM backend.

Part of the coding-adventures multi-language barcode monorepo.

---

## What is a QR Code?

A QR Code is a square grid of dark and light **modules**.  Its size is
`(4V + 17) × (4V + 17)` where V is the version from 1 to 40.

```
Version 1:  21×21    (minimum)
Version 7:  45×45    (first version with version information blocks)
Version 40: 177×177  (maximum — holds up to 2953 bytes)
```

The symbol contains:
- Three **finder patterns** (top-left, top-right, bottom-left corners) — the
  distinctive square-in-square patterns that let scanners locate and orient the symbol.
- **Timing strips** — alternating dark/light strips along row 6 and column 6.
- **Alignment patterns** (version 2+) — small secondary patterns for distortion correction.
- **Format information** — BCH-protected 15-bit code encoding ECC level and mask index.
- **Version information** (version 7+) — BCH-protected 18-bit code.
- **Data and ECC modules** — the actual encoded message plus Reed-Solomon redundancy.

---

## Usage

```kotlin
import com.codingadventures.qrcode.EccLevel
import com.codingadventures.qrcode.encode
import com.codingadventures.qrcode.encodeAndLayout
import com.codingadventures.barcode2d.Barcode2DLayoutConfig

// Encode to a ModuleGrid (abstract boolean grid)
val grid = encode("HELLO WORLD", EccLevel.M).getOrThrow()
println("Grid size: ${grid.rows}×${grid.cols}")  // 21×21 for version 1

// Encode + layout to PaintScene (pixel-resolved)
val config = Barcode2DLayoutConfig(moduleSizePx = 10, quietZoneModules = 4)
val scene = encodeAndLayout("https://example.com", EccLevel.M, config).getOrThrow()
println("Canvas: ${scene.width}×${scene.height} px")
```

---

## API

### `encode(input, ecc, minVersion?)`

Encode a UTF-8 string into a `ModuleGrid`.

| Parameter    | Type       | Default | Description                              |
|--------------|------------|---------|------------------------------------------|
| `input`      | `String`   | —       | UTF-8 string to encode                   |
| `ecc`        | `EccLevel` | —       | Error correction level (L/M/Q/H)         |
| `minVersion` | `Int`      | `1`     | Minimum version to use (1–40)            |

Returns `Result<ModuleGrid>`.  Fails with `QRCodeError.InputTooLong` if the
input exceeds version-40 capacity.

### `encodeAndLayout(input, ecc, config?)`

Convenience: encode + layout in one call.

Returns `Result<PaintScene>` ready for any PaintVM backend.

---

## Error Correction Levels

| Level | Recovery | Use case                               |
|-------|----------|----------------------------------------|
| L     | ~7%      | Clean environments, maximum data       |
| M     | ~15%     | General purpose (recommended default)  |
| Q     | ~25%     | Labels, outdoor use, rough surfaces    |
| H     | ~30%     | Industrial, high-damage environments   |

---

## Encoding Modes

The encoder automatically selects the most compact mode for the input:

| Mode          | Characters                            | Bits/char |
|---------------|---------------------------------------|-----------|
| Numeric       | 0–9                                   | ~3.3      |
| Alphanumeric  | 0–9, A–Z, SP, $%*+-./:               | ~5.5      |
| Byte (UTF-8)  | Any string (raw UTF-8 bytes)          | 8         |

---

## Dependency stack

```
paint-instructions (P2D00)
        │
    barcode-2d          gf256 (MA01)
        │                   │
    qr-code ────────────────┘
```

- **gf256** (MA01): GF(256) arithmetic over polynomial 0x11D.
- **barcode-2d**: `ModuleGrid` type and `layout()` function.
- **paint-instructions**: `PaintScene` type consumed by paint backends.

`qr-code` does NOT depend on **reed-solomon** (MA02) because QR Code uses
the b=0 RS convention (roots α^0..α^{n-1}) while MA02 uses the b=1 convention
(roots α^1..α^n).  The RS encoder is implemented directly in this package.

---

## Implementation

This package is a direct port of the Rust reference implementation at
`code/packages/rust/qr-code/src/lib.rs`.  All algorithms follow ISO/IEC
18004:2015.

Key implementation decisions:
- Format info bit ordering is MSB-first for row 8, cols 0–5 (f14→f9). See
  lessons.md for the hard-won lesson about this critical detail.
- GF(256) multiplication delegates to the `GF256.mul()` function from the
  gf256 (MA01) package.
- The RS generator polynomial uses the b=0 convention: roots are α^0, α^1,
  …, α^{n-1} rather than α^1, α^2, …, α^n.

---

## Version

`0.1.0` — Initial release.

See [CHANGELOG.md](CHANGELOG.md) for full history.
