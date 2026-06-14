# aztec-code (Kotlin)

Aztec Code encoder for Kotlin — ISO/IEC 24778:2008 compliant.

## What is Aztec Code?

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
published as a patent-free format. Unlike QR Code (which uses three finder
squares at three corners), Aztec Code places a single **bullseye at the center**
of the symbol. The scanner finds the center first, then reads outward in a spiral
— no large quiet zone is required.

It is in heavy operational use today:

- **IATA boarding passes** — the barcode on every airline boarding pass
- **Eurostar and Amtrak rail tickets** — printed and on-screen tickets
- **PostNL, Deutsche Post, La Poste** — European postal routing
- **US military ID cards**

## Symbol variants

| Variant | Layers | Symbol size | Formula        |
|---------|--------|-------------|----------------|
| Compact | 1–4    | 15×15–27×27 | 11 + 4*layers  |
| Full    | 1–32   | 19×19–143×143 | 15 + 4*layers |

The encoder automatically selects the smallest symbol that fits the data at
the requested ECC level.

## How it fits in the stack

```
Input string / bytes
  → aztec-code (this package) → ModuleGrid
  → barcode-2d layout()       → PaintScene
  → paint-vm                  → SVG / Canvas / Metal pixels
```

`aztec-code` only produces the abstract boolean grid. Rendering is handled by
`com.codingadventures.barcode2d.layout`.

## Installation

This package is part of the `coding-adventures` monorepo. Add it as a
Gradle composite build (see `settings.gradle.kts` for `includeBuild()` declarations).

```kotlin
// settings.gradle.kts
includeBuild("../paint-instructions")
includeBuild("../barcode-2d")
includeBuild("../aztec-code")
```

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.codingadventures:aztec-code")
}
```

## Usage

```kotlin
import com.codingadventures.azteccode.encode
import com.codingadventures.azteccode.AztecOptions

// Auto-select smallest compact or full symbol at 23% ECC:
val grid = encode("Hello")        // → 19×19 compact 2-layer symbol
val grid2 = encode("A")           // → 15×15 compact 1-layer symbol

// Raise ECC threshold to 33%:
val highEcc = encode("Hello", AztecOptions(minEccPercent = 33))

// Encode raw bytes (e.g. binary data):
val bytes = byteArrayOf(0x01, 0x02, 0x03)
val binaryGrid = encode(bytes)

// Use with barcode-2d:
import com.codingadventures.barcode2d.layout
import com.codingadventures.barcode2d.Barcode2DLayoutConfig

val scene = layout(grid, Barcode2DLayoutConfig(moduleSizePx = 4))
```

## Error handling

All encoding failures throw a subclass of `AztecError` (a sealed class):

```kotlin
import com.codingadventures.azteccode.AztecError

try {
    encode("X".repeat(2000))
} catch (e: AztecError.InputTooLong) {
    println("Too long: ${e.message}")
}
```

| Error                     | Cause                                            |
|---------------------------|--------------------------------------------------|
| `AztecError.InputTooLong` | Data exceeds 32-layer full symbol capacity       |

## API reference

```kotlin
// Encode string (UTF-8) → ModuleGrid
fun encode(data: String, options: AztecOptions = AztecOptions()): ModuleGrid

// Encode raw bytes → ModuleGrid
fun encode(data: ByteArray, options: AztecOptions = AztecOptions()): ModuleGrid

// Options
data class AztecOptions(
    val minEccPercent: Int = 23,   // ECC threshold (10–90, default 23)
)
```

The returned `ModuleGrid` is fully immutable:
- `grid.rows`, `grid.cols` — symbol dimensions
- `grid.modules[row][col]` — `true` = dark module, `false` = light module
- `grid.moduleShape` — always `ModuleShape.SQUARE` for Aztec

## Encoding pipeline

```
input string / bytes
  → Binary-Shift from Upper mode (byte-mode only, v0.1.0)
  → symbol size selection (smallest compact, then full)
  → pad to exact codeword count
  → GF(256)/0x12D Reed-Solomon ECC
  → bit stuffing (insert complement after 4 consecutive identical bits)
  → GF(16) mode message (layers + codeword count + RS nibbles)
  → draw bullseye (concentric dark/light rings)
  → draw reference grid (full symbols only)
  → place orientation marks (4 dark corners of mode ring)
  → place mode message bits in mode ring
  → spiral data+ECC bits outward from innermost layer
  → return ModuleGrid
```

## Symbol structure

```
┌─────────────────────────────────────┐
│  data layers (spiraling outward)     │
│  ┌──────────────────────────────┐   │
│  │  mode message ring           │   │
│  │  ┌───────────────────────┐   │   │
│  │  │  orientation marks    │   │   │
│  │  │  ┌─────────────────┐  │   │   │
│  │  │  │  bullseye rings  │  │   │   │
│  │  │  │  (center DARK)   │  │   │   │
│  │  │  └─────────────────┘  │   │   │
│  │  └───────────────────────┘   │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
```

## Reed-Solomon parameters

| Usage          | Field    | Poly  | Roots           |
|----------------|----------|-------|-----------------|
| Data codewords | GF(256)  | 0x12D | α^1 … α^n (b=1) |
| Mode message   | GF(16)   | 0x13  | α^1 … α^5 or α^6 |

The GF(256) polynomial `0x12D` (same as Data Matrix ECC200) differs from
QR Code's `0x11D`. The existing `reed-solomon` package uses `0x11D`, so
`aztec-code` implements its own GF(256)/0x12D arithmetic inline.

## Dependencies

- `com.codingadventures:barcode-2d` — `ModuleGrid` and `ModuleShape` types
- `com.codingadventures:paint-instructions` — transitive dep of barcode-2d

## Testing

```bash
cd code/packages/kotlin/aztec-code
mise exec -- gradle test
```

All 87 JUnit 5 assertions cover:
- GF(16) and GF(256)/0x12D arithmetic
- Bit stuffing algorithm
- Binary-Shift encoding
- Mode message encoding (compact and full)
- Symbol size selection
- Full encode pipeline (smoke, determinism, corpus)
- Bullseye pattern validation
- Orientation mark placement
- Error handling
- Grid immutability

## v0.1.0 limitations

1. **Byte mode only** — all input encoded via Binary-Shift from Upper mode.
   Multi-mode optimization (Digit/Upper/Lower/Mixed/Punct) is v0.2.0.
2. **Auto-select only** — force-compact option is v0.2.0.
3. **GF(256) RS only** for data codewords. GF(16)/GF(32) RS for shorter codewords is v0.2.0.

## Version

`0.1.0` — initial release implementing the full compact (1-4 layers) and full
(1-32 layers) Aztec Code encoder.
