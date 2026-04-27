# CodingAdventures.QRCode (F#)

An ISO/IEC 18004:2015-compliant QR Code encoder for F#.  Encodes any UTF-8
string into a scannable QR Code symbol and returns a `ModuleGrid` ready for
rendering with `CodingAdventures.Barcode2D`.

## Where this fits in the stack

```
input string
  → QRCode.encode              ← this package
  → ModuleGrid
  → Barcode2D.layout           ← CodingAdventures.Barcode2D
  → PaintScene
  → paint-vm / SVG backend
```

## Quick start

```fsharp
open CodingAdventures.QRCode
open CodingAdventures.Barcode2D

// Encode "HELLO WORLD" with ECC level M (≈15% recovery)
match QRCode.encode "HELLO WORLD" EccLevel.M with
| Ok grid  ->
    printfn "Symbol size: %d × %d modules" grid.Rows grid.Cols
    // Pass grid to Barcode2D.layout for pixel-level rendering
| Error e  ->
    printfn "Encoding failed: %A" e
```

## Encoding modes

The encoder automatically picks the most compact mode for the input:

| Mode         | Characters allowed                                | Bits per char |
|--------------|---------------------------------------------------|---------------|
| Numeric      | `0`–`9`                                           | ~3.3          |
| Alphanumeric | `0–9 A–Z` and ` $%*+-./:` (45 chars total)        | ~5.5          |
| Byte         | Any UTF-8 byte sequence (fallback)                | 8             |

## Error correction levels

| Level | Recovery capacity | Use case                  |
|-------|-------------------|---------------------------|
| L     | ~7 %              | Clean environments        |
| M     | ~15 %             | Most applications         |
| Q     | ~25 %             | Partial logo overlay      |
| H     | ~30 %             | Maximum damage tolerance  |

## Dependencies

- `CodingAdventures.Gf256` — GF(2^8) arithmetic for Reed-Solomon ECC.
- `CodingAdventures.Barcode2D` — `ModuleGrid` type and `layout` function.

## Implementation notes

- Versions 1–40, all three common encoding modes.
- RS ECC: GF(256) with primitive poly 0x11D, b=0 convention.
- Format info: 15-bit BCH (generator 0x537) XOR'd with 0x5412, written
  MSB-first (f14→f9) across row 8 cols 0–5 (see `lessons.md` for the
  ordering lesson).
- Version info: 18-bit BCH (generator 0x1F25) for versions ≥ 7.
- All 8 ISO mask patterns evaluated; lowest penalty score wins.
- Output: `bool[] array` jagged array via `ModuleGrid.Modules`.

## Running the tests

```sh
# From the package root
bash BUILD
```

Target: ≥ 90 % line coverage.
