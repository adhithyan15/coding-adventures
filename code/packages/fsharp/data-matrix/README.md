# CodingAdventures.DataMatrix (F#)

An ISO/IEC 16022:2006 Data Matrix ECC200 encoder for F#. Encodes any UTF-8
string into a scannable Data Matrix ECC200 symbol and returns a `ModuleGrid`
ready for rendering with `CodingAdventures.Barcode2D`.

## What is Data Matrix?

Data Matrix ECC200 is a two-dimensional matrix barcode standardised as
ISO/IEC 16022:2006. It packs data into a grid of dark and light modules
surrounded by an L-shaped finder and alternating timing borders.

You meet Data Matrix on every printed circuit board (etched for traceability),
on pharmaceutical unit-dose packaging (US FDA DSCSA mandate), and on aerospace
parts where ink-on-paper labels cannot survive the environment.

## Where this fits in the stack

```
input string
  → DataMatrix.encode               ← this package
  → ModuleGrid
  → Barcode2D.layout                ← CodingAdventures.Barcode2D
  → PaintScene
  → paint-vm / SVG backend
```

## Quick start

```fsharp
open CodingAdventures.DataMatrix
open CodingAdventures.Barcode2D

// Encode "Hello World"
match DataMatrix.encode "Hello World" with
| Ok grid ->
    printfn "Symbol size: %d × %d modules" grid.Rows grid.Cols
    // Pass grid to Barcode2D.layout for pixel-level rendering
| Error e ->
    printfn "Encoding failed: %A" e
```

To encode arbitrary bytes (e.g. binary payloads):

```fsharp
let bytes : byte[] = [| 0x01uy; 0x02uy; 0xFFuy |]
match DataMatrix.encodeBytes bytes with
| Ok grid -> printfn "Symbol: %d × %d" grid.Rows grid.Cols
| Error e -> printfn "Error: %A" e
```

## Symbol sizes

The encoder automatically selects the smallest square ECC200 symbol whose
data capacity fits the encoded codeword count.

| Symbol   | Data regions | DataCW | ECC CW |
|----------|-------------|--------|--------|
| 10×10    | 1×1         |      3 |      5 |
| 12×12    | 1×1         |      5 |      7 |
| 14×14    | 1×1         |      8 |     10 |
| …        | …           |      … |      … |
| 144×144  | 6×6         |   1558 |    620 |

The 144×144 symbol can hold up to approximately 1556 ASCII characters or
up to 2335 numeric digit characters (using digit-pair packing).

## Encoding pipeline (v0.1.0)

1. **ASCII encoding** — each input byte is encoded as its ASCII value plus 1;
   consecutive digit pairs are compressed to a single codeword (130 + value).
2. **Symbol selection** — smallest square symbol with sufficient dataCW capacity.
3. **Scrambled padding** — unused capacity filled with scrambled pad values.
4. **Reed-Solomon ECC** — GF(256)/0x12D, b=1 roots (α^1 … α^n); multi-block
   interleaving for large symbols.
5. **Grid initialisation** — L-finder (solid left column + solid bottom row),
   alternating timing clock (top row + right column), alignment borders for
   multi-region symbols.
6. **Utah placement** — diagonal codeword placement with four corner patterns
   and residual fill. No masking step.

## Key differences from QR Code

| Feature              | Data Matrix ECC200 | QR Code           |
|----------------------|--------------------|-------------------|
| GF(256) polynomial   | 0x12D              | 0x11D             |
| RS root convention   | b=1 (α^1 … α^n)    | b=0 (α^0 … α^{n-1}) |
| Finder pattern       | L-shaped border    | Three corner squares |
| Data placement       | Diagonal (Utah)    | Two-column zigzag |
| Masking              | None               | Eight mask patterns |

## v0.1.0 limitations

- ASCII encoding mode only; C40, Text, X12, EDIFACT, Base256 are v0.2.0.
- Square-symbol selection only; rectangular-preference option is v0.2.0.

## Errors

- `InputTooLong msg` — input exceeds 144×144 capacity (~1558 codewords).

## Building and testing

```sh
mise exec -- dotnet test tests/CodingAdventures.DataMatrix.Tests/
```
