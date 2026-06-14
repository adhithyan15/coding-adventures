# coding_adventures_pdf417

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

## What is PDF417?

PDF417 (Portable Data File 417) is a stacked linear barcode invented by Ynjiun P. Wang at Symbol Technologies in 1991. The "417" encodes its geometry: every codeword has exactly **4** bars and **4** spaces (8 elements), occupying exactly **17** horizontal modules. 4 + 1 + 7 = "417".

Unlike a true 2D matrix barcode (QR, Data Matrix), PDF417 is a *stack* of short 1D barcode rows. Each row is independently scannable by a laser, which is why PDF417 is the format of choice for:

- **AAMVA** — North American driver's licences and government IDs
- **IATA BCBP** — Airline boarding passes (the long thin barcode at the gate)
- **USPS** — Domestic shipping labels
- **US immigration** — Form I-94, customs declarations
- **Healthcare** — Patient wristbands, medication labels

## How it fits in the stack

This package sits alongside the other barcode encoders in the monorepo (`elixir/barcode_1d`, `elixir/qr_code`, `elixir/data_matrix`, `elixir/aztec_code`). It returns a plain `%CodingAdventures.PDF417.ModuleGrid{}` struct — a list of rows of booleans — so any render backend can consume it.

## Encoding pipeline

```
raw bytes
  -> byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
  -> length descriptor   (first codeword = total codewords in symbol)
  -> RS ECC              (GF(929) Reed-Solomon, b=3 convention, alpha=3)
  -> dimension selection (auto: roughly square symbol)
  -> padding             (codeword 900 fills unused slots)
  -> row indicators      (LRI + RRI per row, encode R/C/ECC level)
  -> cluster table lookup (codeword -> 17-module bar/space pattern)
  -> start/stop patterns (fixed per row)
  -> ModuleGrid          (abstract boolean grid)
```

## v0.1.0 scope

This release implements **byte compaction only** — every input byte is encoded without character-set translation. Text and numeric compaction (which pack ASCII letters or digit runs more densely) are planned for v0.2.0. Byte mode handles arbitrary binary content correctly, so it is the safe default.

## Quick start

```elixir
alias CodingAdventures.PDF417

# Simple encode — auto ECC, auto dimensions, row height 3
{:ok, grid} = PDF417.encode("Hello, World!")
# grid.rows / grid.cols — module dimensions
# grid.modules — list of rows, each a list of booleans
# true = dark module, false = light module

# With options
{:ok, grid} = PDF417.encode("Hello", ecc_level: 4, columns: 5, row_height: 4)

# Error handling
{:error, :invalid_ecc_level} = PDF417.encode("Hi", ecc_level: 9)
```

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `ecc_level` | `0..8 \| :auto` | `:auto` | Reed-Solomon ECC level (higher = more resilient, larger) |
| `columns` | `1..30 \| :auto` | `:auto` | Number of data columns (1–30) |
| `row_height` | `pos_integer()` | `3` | Pixel rows per logical PDF417 row |

## Installation

```elixir
# In mix.exs deps:
{:coding_adventures_pdf417, path: "../pdf417"}
```

## Running tests

```bash
mix deps.get --quiet && mix test --cover
```
