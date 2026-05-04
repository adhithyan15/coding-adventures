# coding_adventures_pdf417

Pure-Ruby PDF417 stacked-linear barcode encoder, ISO/IEC 15438:2015 compliant.

## What is PDF417?

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes the symbol's geometry:

- Every codeword has **4 bars** and **4 spaces** (8 elements total).
- Every codeword occupies exactly **17 modules** of horizontal space.
- 4 + (the 1 implied by "of each") + 7 = **"417"**.

PDF417 is a **stacked linear barcode** — a 2D symbology that looks like
multiple 1D barcodes stacked on top of each other. Unlike QR Code (matrix
arrangement) or Aztec Code (concentric bullseye), PDF417 is composed of rows
of standard 1D bar/space patterns.

### Where PDF417 is deployed

| Application | Details |
|---|---|
| **AAMVA** | North American driver's licences and government ID cards |
| **IATA BCBP** | Airline boarding passes (the tall thin barcode at the gate) |
| **USPS** | Domestic shipping and package labels |
| **US immigration** | Form I-94, customs declarations, arrival/departure records |
| **Healthcare** | Patient wristbands, medication labels, lab specimen tubes |

## Installation

```ruby
# Gemfile
gem "coding_adventures_pdf417"
```

```sh
bundle install
```

## Quick start

```ruby
require "coding_adventures/pdf417"

# Encode a string (arbitrary bytes)
grid = CodingAdventures::PDF417.encode("HELLO WORLD")

grid.rows    # => Integer — module height of the complete symbol
grid.cols    # => Integer — module width  of the complete symbol

# modules[r][c] is true when the module at row r, column c is dark (black)
grid.modules # => Array<Array<Boolean>>

# Render as ASCII art (dark = '#', light = ' ')
grid.modules.each do |row|
  puts row.map { |dark| dark ? "#" : " " }.join
end
```

## Options

```ruby
CodingAdventures::PDF417.encode(data, opts = {})
```

| Option | Type | Default | Description |
|---|---|---|---|
| `ecc_level:` | Integer 0–8 | auto | Reed-Solomon error correction level |
| `columns:` | Integer 1–30 | auto | Number of data columns |
| `row_height:` | Integer ≥ 1 | 3 | Module rows per logical row |

### ECC level reference

| Level | ECC codewords | Approx recovery capacity |
|---|---|---|
| 0 | 2 | Minimal — pristine conditions only |
| 1 | 4 | |
| 2 | 8 | Auto-selected for ≤ 40 data codewords |
| 3 | 16 | Auto-selected for ≤ 160 data codewords |
| 4 | 32 | Auto-selected for ≤ 320 data codewords |
| 5 | 64 | Auto-selected for ≤ 863 data codewords |
| 6 | 128 | Auto-selected for > 863 data codewords |
| 7 | 256 | |
| 8 | 512 | Maximum — very large physical symbols |

## Encoding pipeline

```
raw bytes (String or Array<Integer 0..255>)
  → byte compaction      codeword 924 latch + 6 bytes → 5 codewords (base 900)
  → length descriptor    first codeword = total symbol codeword count
  → GF(929) Reed-Solomon b=3 convention, α=3, auto or explicit ECC level
  → dimension selection  auto: roughly square symbol (3 ≤ rows ≤ 90, 1 ≤ cols ≤ 30)
  → padding              codeword 900 fills empty grid slots
  → row indicators       LRI + RRI per row encode R/C/ECC so partial rows are decodable
  → cluster table lookup codeword → 17-module packed bar/space pattern
  → start/stop patterns  fixed patterns at each row's left and right edge
  → ModuleGrid           2D boolean array, true = dark module
```

### Byte compaction (v0.1.0 only)

This release implements **byte compaction only**. Six input bytes pack into
five codewords by treating them as a 48-bit integer in base 900:

```
n = b0·256^5 + b1·256^4 + b2·256^3 + b3·256^2 + b4·256 + b5
codewords = digits(n, base=900)    # exactly 5 digits
```

Proof that this is lossless: 2^48 = 281,474,976,710,656 < 900^5 = 590,490,000,000,000.

Text and numeric compaction (which produce shorter sequences for ASCII text
and digit strings) are planned for v0.2.0.

## Error handling

```ruby
begin
  grid = CodingAdventures::PDF417.encode(data, ecc_level: 9)
rescue CodingAdventures::PDF417::InvalidECCLevelError => e
  puts e.message   # "InvalidECCLevelError: ecc_level must be an integer in 0..8..."
rescue CodingAdventures::PDF417::InvalidDimensionsError => e
  puts e.message
rescue CodingAdventures::PDF417::InputTooLongError => e
  puts e.message
rescue CodingAdventures::PDF417::PDF417Error => e
  puts "Generic encoding error: #{e.message}"
end
```

## GF(929) arithmetic

The encoder builds GF(929) log/antilog tables at module load time (once,
~0.1 ms, ~7 KB). These tables power O(1) Reed-Solomon multiplication across
the 929-element Galois field.

## ModuleGrid

The returned struct has three fields:

```ruby
grid.rows    # Integer — total module rows (logical rows × row_height)
grid.cols    # Integer — total module columns (69 + 17 × data_columns)
grid.modules # Array<Array<Boolean>> — 0-indexed [row][col], true = dark
```

Module width formula: `69 + 17 × cols` where `cols` is the number of data
columns. The 69 constant accounts for start (17) + LRI (17) + RRI (17) +
stop (18) = 69 fixed modules per row.

## Dependencies

**None** — the encoder is entirely self-contained. GF(929) tables, Reed-Solomon
encoding, cluster tables, and the ModuleGrid output struct all live in this gem.

## Development

```sh
bundle install
bundle exec rake spec          # run test suite
bundle exec standardrb lib/   # lint
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

MIT
