# micro-qr (Kotlin)

Micro QR Code encoder for Kotlin — ISO/IEC 18004:2015 Annex E compliant.

## What is Micro QR Code?

Micro QR Code is the compact variant of regular QR Code, designed for
applications where even the smallest standard QR symbol (21×21 modules at
version 1) is too large.  Common use cases:

- **Surface-mount component labels** on PCBs
- **Circuit-board markings** for revision and part numbers
- **Miniature industrial tags** in tight spaces
- **Tiny stickers** where a 13×13 symbol suffices

There are exactly four Micro QR symbol versions:

| Version | Size    | Numeric cap | Alpha cap | Byte cap |
|---------|---------|-------------|-----------|----------|
| M1      | 11×11   | 5           | —         | —        |
| M2      | 13×13   | 10          | 6         | 4        |
| M3      | 15×15   | 23          | 14        | 9        |
| M4      | 17×17   | 35          | 21        | 15       |

Capacities shown are for the lowest available ECC level.

## How it fits in the stack

```
Input string
  → micro-qr (this package) → ModuleGrid
  → barcode-2d layout()     → PaintScene
  → paint-vm                → SVG / Canvas / Metal pixels
```

`micro-qr` only produces the abstract boolean grid.  Rendering is handled by
`com.codingadventures.barcode2d.layout`.

## Installation

This package is part of the `coding-adventures` monorepo.  Add it as a
Gradle composite build (see `settings.gradle.kts` for the `includeBuild()`
declarations).

```kotlin
// settings.gradle.kts
includeBuild("../micro-qr")
```

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.codingadventures:micro-qr")
}
```

## Usage

```kotlin
import com.codingadventures.microqr.encode
import com.codingadventures.microqr.ECCLevel
import com.codingadventures.microqr.MicroQROptions

// Auto-select smallest symbol and ECC level:
val grid = encode("HELLO")
// → 13×13 M2 symbol

// Force a specific version and ECC:
val grid2 = encode("12345", MicroQROptions(symbol = "M1", eccLevel = ECCLevel.DETECTION))
// → 11×11 M1 symbol

// Use M4 with Q-level ECC:
val grid3 = encode("HELLO", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.Q))
// → 17×17 M4 symbol with ~25% error recovery

// Force mask pattern 2 (useful for testing or cross-language parity):
val grid4 = encode("1", MicroQROptions(maskPattern = 2))

// Render with barcode-2d:
import com.codingadventures.barcode2d.layout
import com.codingadventures.barcode2d.Barcode2DLayoutConfig

val scene = layout(grid, Barcode2DLayoutConfig(moduleSizePx = 4))
```

## Error handling

All encoding failures throw a subclass of `MicroQRError`:

```kotlin
import com.codingadventures.microqr.MicroQRError

try {
    encode("1".repeat(36))  // exceeds M4-L capacity
} catch (e: MicroQRError.InputTooLong) {
    println("Too long: ${e.message}")
}

try {
    encode("1", MicroQROptions(symbol = "M1", eccLevel = ECCLevel.L))
} catch (e: MicroQRError.InvalidECCLevel) {
    println("Bad combo: ${e.message}")
}

try {
    encode("1", MicroQROptions(symbol = "M9"))
} catch (e: MicroQRError.InvalidOptions) {
    println("Bad option: ${e.message}")
}
```

| Error                      | Cause                                           |
|----------------------------|-------------------------------------------------|
| `MicroQRError.InputTooLong`    | Data exceeds the requested symbol's capacity    |
| `MicroQRError.InvalidECCLevel` | Version+ECC combination does not exist in spec  |
| `MicroQRError.InvalidOptions`  | Out-of-range mask pattern or unknown symbol ID  |

## ECC levels

| Level     | Available in | Recovery |
|-----------|--------------|----------|
| DETECTION | M1 only      | Detects errors only, no correction |
| L         | M2, M3, M4   | ~7% of codewords |
| M         | M2, M3, M4   | ~15% of codewords |
| Q         | M4 only      | ~25% of codewords |

Level H (High, ~30%) is not available in any Micro QR symbol.

## Key differences from regular QR Code

| Feature               | Regular QR      | Micro QR        |
|-----------------------|-----------------|-----------------|
| Finder patterns       | 3               | 1 (top-left)    |
| Timing row/col        | Row 6 / Col 6   | Row 0 / Col 0   |
| Mask patterns         | 8               | 4               |
| Format XOR mask       | 0x5412          | 0x4445          |
| Format info copies    | 2               | 1               |
| Quiet zone            | 4 modules       | 2 modules       |
| Mode indicator bits   | 4               | 0–3 (grows)     |

## Encoding pipeline

```
input string
  → auto-select smallest symbol (M1–M4) and most compact mode
  → build bit stream: mode indicator + char count + data + terminator + padding
  → Reed-Solomon ECC over GF(256)/0x11D, b=0, single block
  → init 7×7 finder pattern, L-shaped separator, timing at row 0/col 0
  → two-column zigzag data placement (bottom-right to top-left snake)
  → evaluate 4 mask patterns → pick lowest penalty
  → write 15-bit format word (XOR 0x4445, single copy)
  → ModuleGrid
```

## Dependencies

- `com.codingadventures:gf256` — GF(256)/0x11D arithmetic for RS encoding
- `com.codingadventures:barcode-2d` — `ModuleGrid` and `ModuleShape` types
- `com.codingadventures:paint-instructions` — transitive dep of barcode-2d

## Testing

```bash
cd code/packages/kotlin/micro-qr
mise exec -- ./gradlew test
```

Test coverage is comprehensive: symbol dimensions, auto-selection, all ECC
levels, structural module patterns, RS encoder, mask conditions, penalty scorer,
error handling, capacity boundaries, and cross-language corpus inputs.

## Version

`0.1.0` — initial release implementing the full M1–M4 encoder.
