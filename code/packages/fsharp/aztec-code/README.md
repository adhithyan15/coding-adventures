# CodingAdventures.AztecCode (F#)

An ISO/IEC 24778:2008-compliant Aztec Code encoder for F#. Encodes any UTF-8
string into a scannable Aztec Code symbol and returns a `ModuleGrid` ready for
rendering with `CodingAdventures.Barcode2D`.

## What is Aztec Code?

Aztec Code is a 2D matrix barcode invented by Andrew Longacre Jr. at Welch
Allyn in 1995. Where QR Code uses three square finder patterns at three
corners, Aztec Code uses a single **bullseye finder pattern at the center** of
the symbol — so a scanner finds the center first and then reads outward in a
clockwise spiral. Because the bullseye fully determines orientation, no large
quiet zone is required.

You meet Aztec Code every time you fly: it is the format used on every IATA
boarding pass. Eurostar, Amtrak, the SBB Swiss railway and many European
postal services all use it too.

## Where this fits in the stack

```
input string
  → AztecCode.encode               ← this package
  → ModuleGrid
  → Barcode2D.layout               ← CodingAdventures.Barcode2D
  → PaintScene
  → paint-vm / SVG backend
```

## Quick start

```fsharp
open CodingAdventures.AztecCode
open CodingAdventures.Barcode2D

// Encode "HELLO" with default 23% minimum ECC
match AztecCode.encode "HELLO" with
| Ok grid ->
    printfn "Symbol size: %d × %d modules" grid.Rows grid.Cols
    // Pass grid to Barcode2D.layout for pixel-level rendering
| Error e ->
    printfn "Encoding failed: %A" e
```

To tune the error correction level:

```fsharp
let opts = { defaultOptions with MinEccPercent = 50 }
match AztecCode.encodeWith "MY DATA" opts with
| Ok grid -> ...
| Error e -> ...
```

To encode arbitrary bytes (e.g. binary payloads):

```fsharp
let bytes : byte[] = [| 0x01uy; 0x02uy; 0xffuy |]
match AztecCode.encodeBytes bytes with
| Ok grid -> ...
| Error e -> ...
```

## Symbol variants

| Variant   | Layers | Size                |
|-----------|--------|---------------------|
| Compact   | 1–4    | 15×15 to 27×27      |
| Full      | 1–32   | 19×19 to 143×143    |

The encoder picks the smallest symbol that can hold the input at the requested
ECC level — compact 1 → 4 first, then full 1 → 32.

## Encoding pipeline (v0.1.0)

1. **Binary-Shift from Upper mode** — every input byte is wrapped in a single
   Binary-Shift block so all data flows through byte mode.
2. **Symbol selection** — smallest compact then full at minimum 23% ECC.
3. **Padding** — bit stream is padded to the chosen data codeword count.
4. **Reed-Solomon ECC** — GF(256) with primitive polynomial `0x12D`
   (the same polynomial as Data Matrix, **not** QR Code's `0x11D`).
5. **Bit stuffing** — insert one complement bit after every run of 4
   identical bits.
6. **Mode message** — 28-bit (compact) or 40-bit (full) word containing
   `(layers, dataCwCount)`, protected by GF(16) RS over `0x13`.
7. **Grid construction** — bullseye at center, orientation marks, mode
   message ring, then a clockwise spiral of data + ECC bits through each
   layer band.

## v0.1.0 simplifications

- Byte mode only (no per-character Digit/Upper/Lower/Mixed/Punct optimization).
- 8-bit codewords + GF(256) Reed-Solomon only.
- Default ECC = 23%.
- Auto-select compact vs full (no force-compact option).

These are all candidates for v0.2.0.

## Errors

The encoder returns an `AztecError` discriminated union:

- `InputTooLong msg` — the input exceeds 32-layer full symbol capacity.
- `InvalidOptions msg` — `MinEccPercent` was outside the range 10–90.

## Building

```
mise exec -- dotnet test tests/CodingAdventures.AztecCode.Tests/
```
