# CodingAdventures.MicroQR.FSharp

ISO/IEC 18004:2015 Annex E-compliant **Micro QR Code encoder** for F#.

Produces a `ModuleGrid` (from `CodingAdventures.Barcode2D`) scannable by any
standard Micro QR Code decoder.

---

## What is Micro QR Code?

Micro QR Code is the compact variant of standard QR Code, designed for
applications where even the smallest standard QR symbol (21×21 modules at
version 1) is too large.

Common use cases:
- Surface-mount component labels
- Circuit board markings
- Miniature industrial tracking tags
- Wearable or implantable device labels

There are four symbol sizes:

| Symbol | Size    | Max numeric | Max alphanumeric | Max byte |
|--------|---------|-------------|------------------|----------|
| M1     | 11×11   | 5           | —                | —        |
| M2     | 13×13   | 10 (L) / 8 (M)  | 6 (L) / 5 (M)   | 4 (L) / 3 (M) |
| M3     | 15×15   | 23 (L) / 18 (M) | 14 (L) / 11 (M) | 9 (L) / 7 (M) |
| M4     | 17×17   | 35 (L) / 30 (M) / 21 (Q) | 21 (L) / 18 (M) / 13 (Q) | 15 (L) / 13 (M) / 9 (Q) |

The size formula is `size = 2 × version_number + 9`.

---

## Where this fits in the pipeline

```
Input data (string)
  → CodingAdventures.MicroQR.encode
  → ModuleGrid                   ← this package produces this
  → Barcode2D.layout             ← converts to pixel PaintScene
  → PaintVM backend (SVG, Canvas, Metal, ...)
```

---

## Installation

```xml
<PackageReference Include="CodingAdventures.MicroQR.FSharp" Version="0.1.0" />
```

---

## Usage

### Fully automatic (recommended)

```fsharp
open CodingAdventures.MicroQR
open CodingAdventures.Barcode2D

// Auto-select smallest symbol and best encoding mode
match encode "HELLO" defaultOptions with
| Ok grid ->
    printfn "Symbol: %d×%d" grid.Rows grid.Cols
    // grid.Modules.[row].[col] = true  → dark module (ink)
    // grid.Modules.[row].[col] = false → light module (background)
| Error err ->
    printfn "Encoding failed: %A" err
```

### Force a specific symbol and ECC level

```fsharp
let opts = { defaultOptions with Symbol = Some "M4"; ECCLevel = Some L }
match encode "Hello, World!" opts with
| Ok grid -> printfn "%d×%d" grid.Rows grid.Cols  // 17×17
| Error e -> printfn "%A" e
```

### Force a specific mask pattern

```fsharp
let opts = { defaultOptions with MaskPattern = Some 2 }
let Ok grid = encode "12345" opts
```

---

## API Reference

### Types

```fsharp
type ECCLevel = Detection | L | M | Q

type MicroQROptions = {
    Symbol: string option      // "M1"/"M2"/"M3"/"M4" or None (auto)
    ECCLevel: ECCLevel option  // None for auto
    MaskPattern: int option    // None for auto (lowest-penalty mask)
}

let defaultOptions = { Symbol = None; ECCLevel = None; MaskPattern = None }

type MicroQRError =
    | InputTooLong of string
    | InvalidECCLevel of string
    | InvalidOptions of string
```

### Function

```fsharp
val encode : data: string -> opts: MicroQROptions -> Result<ModuleGrid, MicroQRError>
```

---

## ECC level availability

| ECC Level | Available in | Error recovery |
|-----------|-------------|----------------|
| `Detection` | M1 only | Detects errors, no correction |
| `L`         | M2, M3, M4 | ~7% of codewords |
| `M`         | M2, M3, M4 | ~15% of codewords |
| `Q`         | M4 only     | ~25% of codewords |

Level H is not defined for any Micro QR symbol.

---

## Key differences from regular QR Code

| Feature              | Regular QR                | Micro QR                       |
|---------------------|---------------------------|--------------------------------|
| Finder patterns      | Three 7×7 corners         | One 7×7 top-left only          |
| Timing row/col       | Row 6 / col 6             | Row 0 / col 0                  |
| Mask patterns        | 8 patterns (0–7)          | 4 patterns (0–3)               |
| Format XOR mask      | 0x5412                    | 0x4445                         |
| Format info copies   | Two (one per corner pair) | One L-shaped strip             |
| Quiet zone           | 4 modules                 | 2 modules                      |
| Mode indicator bits  | Always 4                  | 0 (M1), 1 (M2), 2 (M3), 3 (M4)|
| RS interleaving      | Multi-block possible      | Always single block            |

---

## Encoding pipeline

```
input string
  → select smallest symbol (M1..M4) + encoding mode (Numeric/Alphanumeric/Byte)
  → build bit stream:
      [mode indicator] [char count] [data bits] [terminator] [byte-align] [pad]
  → Reed-Solomon ECC:
      GF(256) / primitive poly 0x11D, b=0 convention, single block
  → initialize grid:
      finder (7×7 top-left) + L-separator + timing (row0/col0) + format reserved
  → zigzag data placement:
      two-column snake, bottom-right to top-left, skipping reserved modules
  → evaluate 4 mask patterns, pick lowest penalty score
  → write format info:
      15-bit BCH word XOR 0x4445, placed in L-shaped strip at row 8 / col 8
  → ModuleGrid (immutable bool[][] via Barcode2D)
```

---

## Dependencies

- `CodingAdventures.Barcode2D.FSharp` — `ModuleGrid`, `ModuleShape`, `Barcode2D.makeModuleGrid`
- `CodingAdventures.Gf256.FSharp` — GF(256) field (referenced by the project, arithmetic implemented locally for RS encoding)

---

## License

MIT
