# CodingAdventures.PDF417.FSharp

ISO/IEC 15438:2015-compliant **PDF417 stacked linear barcode encoder** for F#.

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes its geometry: each codeword has
exactly **4 bars and 4 spaces** (8 elements) occupying exactly **17 modules**.

## Where PDF417 is deployed

| Application    | Detail                                               |
|----------------|------------------------------------------------------|
| AAMVA          | North American driver's licences and government IDs  |
| IATA BCBP      | Airline boarding passes                              |
| USPS           | Domestic shipping labels                             |
| US immigration | Form I-94, customs declarations                      |
| Healthcare     | Patient wristbands, medication labels                |

## Package position in the stack

```
paint-vm-svg  (P2D01)
      │
paint-instructions (P2D00)
      │
  barcode-2d  ← layout, ModuleGrid
      │
   pdf417     ← THIS PACKAGE
```

`pdf417` depends on `barcode-2d` for the `ModuleGrid` type. It does **not**
depend on `gf256` or `reed-solomon` — PDF417 uses its own GF(929) field
(prime characteristic, not binary polynomial arithmetic).

## Usage

```fsharp
open CodingAdventures.PDF417
open CodingAdventures.Barcode2D

// Encode a UTF-8 string with default options.
let grid = encodeString "HELLO WORLD" defaultOptions |> Result.get

// Encode raw bytes.
let bytes = System.Text.Encoding.UTF8.GetBytes("My payload")
let grid2 = encode bytes defaultOptions |> Result.get

// Customise options.
let opts =
    { defaultOptions with
        EccLevel  = Some 4    // ECC level 4 (32 ECC codewords)
        Columns   = Some 5    // 5 data columns
        RowHeight = Some 4 }  // 4 module-rows per logical row
let grid3 = encodeString "Custom symbol" opts |> Result.get

// Access the boolean module grid.
printfn "Symbol size: %d × %d modules" grid.Cols grid.Rows
```

## Encoding pipeline

```
raw bytes
  → byte compaction      codeword 924 + 6-bytes-to-5-codewords base-900
  → length descriptor    codeword 0 = total codewords in symbol
  → GF(929) RS ECC       b=3 convention, roots α^3..α^{k+2}, α=3
  → dimension selection  c = ceil(sqrt(total/3)), r = ceil(total/c)
  → padding              codeword 900 fills unused grid slots
  → row indicators       LRI + RRI per row (encode R, C, ECC level)
  → cluster table lookup codeword → 17-module bar/space pattern
  → start/stop patterns  fixed 17-module start + 18-module stop
  → ModuleGrid           abstract boolean grid (no pixels yet)
```

## Options

| Field       | Type        | Default       | Description                          |
|-------------|-------------|---------------|--------------------------------------|
| `EccLevel`  | `int option`| auto-selected | RS ECC level 0–8                     |
| `Columns`   | `int option`| auto-selected | Data columns 1–30                    |
| `RowHeight` | `int option`| 3             | Module-rows per logical PDF417 row   |

**ECC auto-selection thresholds:**

| Data codewords | ECC level | ECC codewords |
|---------------|-----------|---------------|
| ≤ 40          | 2         | 8             |
| ≤ 160         | 3         | 16            |
| ≤ 320         | 4         | 32            |
| ≤ 863         | 5         | 64            |
| > 863         | 6         | 128           |

## Error types

```fsharp
type PDF417Error =
    | InputTooLong of string       // data exceeds symbol capacity
    | InvalidDimensions of string  // columns out of 1–30
    | InvalidECCLevel of string    // ECC level out of 0–8
```

## GF(929) — the prime field

Unlike QR Code (which uses GF(256)), PDF417 performs Reed-Solomon over
**GF(929)**. Since 929 is prime, this is just the integers modulo 929.
Every non-zero element has a multiplicative inverse. The generator
α = 3 is a primitive root: 3^928 ≡ 1 (mod 929) (Fermat's little theorem),
and 3^k ≢ 1 for 0 < k < 928.

Log/antilog lookup tables reduce each GF(929) multiplication to two table
lookups and a modular addition — fast and constant-time.

## Cluster tables

PDF417 uses three distinct 929-entry codeword-to-bar/space pattern tables
(clusters 0, 3, 6) cycling row by row. The cluster for row r is
`(r mod 3)`. Having different patterns for the same codeword value in each
cluster lets a scanner determine which row it is reading independently
of counting from the top.

Tables are embedded as compile-time `uint32[]` constants (11,148 bytes total),
extracted from the Python pdf417 library (MIT License) and matching
ISO/IEC 15438:2015 Annex B.

## v0.1.0 scope

- Byte compaction only (all input treated as raw bytes).
- No text compaction (v0.2.0).
- No numeric compaction (v0.2.0).
- No Macro PDF417 (codewords 925–928).
