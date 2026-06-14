# pdf417 (Haskell)

ISO/IEC 15438:2015 PDF417 stacked linear barcode encoder.

## What is PDF417?

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes its geometry: each codeword has exactly
**4** bars and **4** spaces (8 elements), and every codeword occupies exactly
**17** modules of horizontal space. PDF417 is a **stacked linear** barcode — many
rows of 1D-like encoding stacked vertically — governed by ISO/IEC 15438:2015.

### Where PDF417 is used

| Application | Detail |
|---|---|
| AAMVA | North American driver's licences and government IDs |
| IATA BCBP | Airline boarding passes |
| USPS | Domestic shipping labels |
| US immigration | Form I-94, customs declarations |
| Healthcare | Patient wristbands, medication labels |

## Encoding pipeline

```
raw input string
  → compaction  (auto: text / byte / numeric)
  → length descriptor (total codeword count)
  → GF(929) Reed-Solomon ECC (b=3, α=3)
  → dimension selection (roughly square symbol)
  → padding (codeword 900 fills unused slots)
  → row indicators (LRI + RRI per row)
  → cluster table lookup (codeword → 17-module pattern)
  → start/stop patterns
  → ModuleGrid (abstract boolean grid)
```

## Key algorithmic facts

- **GF(929)**: Reed-Solomon uses the prime field ℤ/929ℤ (not GF(256)). Since
  929 is prime, GF(929) is simply integers modulo 929. No primitive polynomial
  needed. Generator α = 3 (primitive root mod 929).
- **Three codeword clusters**: rows cycle through clusters 0, 1, 2. Each
  codeword value has a different 17-module bar/space pattern per cluster,
  making every row self-identifying.
- **b=3 convention**: RS roots are α³, α⁴, ..., α^{k+2}. Different from QR
  Code (b=0) or Data Matrix (b=1).

## Usage

```haskell
import CodingAdventures.PDF417

-- Encode with defaults (auto-compaction, auto ECC level, auto dimensions)
case encodePDF417 "HELLO WORLD" of
  Left err   -> putStrLn ("Error: " ++ show err)
  Right grid -> putStrLn ("Grid: " ++ show (mgRows grid) ++ "×" ++ show (mgCols grid))

-- Encode with custom options
let opts = defaultOptions
      { eccLevel  = Just 4       -- ECC level 4 (32 ECC codewords)
      , columns   = Just 5       -- 5 data columns
      , rowHeight = 4            -- 4 module-rows per logical row
      }
case encodePDF417With "BOARDING PASS" opts of
  Left err   -> putStrLn ("Error: " ++ show err)
  Right grid -> renderGrid grid
```

## Dependencies

- `barcode-2d` — provides `ModuleGrid`, `emptyGrid`, `setModule`
- `base`, `vector`, `containers`

## Package structure

```
pdf417/
├── src/CodingAdventures/PDF417.hs   — encoder implementation
├── test/PDF417Spec.hs               — hspec test suite
├── pdf417.cabal
├── BUILD
├── README.md
└── CHANGELOG.md
```

## Test coverage

Run tests:

```bash
cabal test
```

The test suite covers:
- GF(929) arithmetic (add, sub, mul, inverses, edge cases)
- RS generator polynomial degree and leading coefficient
- RS encoder output length and range
- All three compaction modes (byte, text, numeric)
- Auto-compaction mode selection
- ECC level auto-selection
- Dimension selection (range and capacity constraints)
- Row indicator formulas (verified against known test vectors)
- Main encoder success paths and error paths
- Start/stop pattern structural invariants
- Determinism
