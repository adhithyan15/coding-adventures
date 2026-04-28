# coding_adventures_data_matrix

Pure-Ruby **Data Matrix ECC200** encoder, compliant with **ISO/IEC 16022:2006**.

Data Matrix is a two-dimensional matrix barcode found on:

- PCBs — etched directly on the substrate for automated traceability.
- Pharmaceuticals — US FDA DSCSA mandates it on unit-dose packages.
- Aerospace parts — etched marks that survive decades of heat and abrasion.
- Medical devices — surgical instruments and implants per GS1 DataMatrix.
- USPS registered mail and customs forms.

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_data_matrix"
```

Or install directly:

```sh
gem install coding_adventures_data_matrix
```

## Usage

```ruby
require "coding_adventures/data_matrix"

# Auto-select the smallest fitting symbol (default: square)
grid = CodingAdventures::DataMatrix.encode("HELLO WORLD")
grid.rows    # => 14  (14×14 for "HELLO WORLD")
grid.cols    # => 14
grid.modules # => Array<Array<Boolean>> — true = dark module

# Force a specific symbol size
grid = CodingAdventures::DataMatrix.encode("A", size: [12, 12])

# Allow rectangular symbols
grid = CodingAdventures::DataMatrix.encode("Hi", shape: :rectangle)
grid.rows  # => 8
grid.cols  # => 18

# Consider all 30 shapes — pick smallest total area
grid = CodingAdventures::DataMatrix.encode("Hi", shape: :any)

# Encode + layout (returns grid + nil scene in v0.1.0)
result = CodingAdventures::DataMatrix.encode_and_layout("Hello")
result[:grid]  # ModuleGrid
result[:scene] # nil (paint_instructions integration planned for v0.2.0)

# Debug: render as '0'/'1' string
puts CodingAdventures::DataMatrix.grid_to_string(grid)
```

## How it fits in the stack

```
coding_adventures_data_matrix
  (no runtime dependencies — self-contained)
```

This gem intentionally has no runtime dependencies. All GF(256)/0x12D arithmetic,
Reed-Solomon encoding, and Utah placement live inside the gem itself.

## Key differences from QR Code

| Property       | QR Code             | Data Matrix ECC200    |
|----------------|---------------------|-----------------------|
| GF(256) poly   | 0x11D               | **0x12D**             |
| RS root start  | b = 0 (α^0..)       | **b = 1 (α^1..)**     |
| Finder         | three corner squares | **one L-shape**       |
| Placement      | column zigzag       | **Utah diagonal**     |
| Masking        | 8 patterns, scored  | **NONE**              |
| Sizes          | 40 versions         | **30 sq + 6 rect**    |

## Algorithm overview

1. **ASCII encoding** — single chars map to `char+1`; two consecutive digits
   are compacted into a single codeword `130 + d1×10 + d2` (digit-pair
   compaction).

2. **Symbol selection** — the smallest symbol whose `data_cw` capacity is ≥
   the encoded codeword count.

3. **Scrambled padding** — unused codeword slots are filled with a deterministic
   scrambled sequence to avoid degenerate Utah placement patterns.

4. **RS ECC per block** — GF(256)/0x12D, b=1 convention (roots α^1, α^2, …).
   Larger symbols use multi-block interleaving for burst-error resilience.

5. **Grid initialization** — L-finder (left column + bottom row, all dark),
   timing clocks (top row + right column alternating), alignment borders for
   multi-region symbols.

6. **Utah placement** — diagonal zigzag with four corner special patterns.
   No masking step (unlike QR Code).

## Symbol sizes

### Square (24 symbols)

| Size     | Data CW | ECC CW |
|----------|---------|--------|
| 10×10    | 3       | 5      |
| 12×12    | 5       | 7      |
| 14×14    | 8       | 10     |
| 16×16    | 12      | 12     |
| 18×18    | 18      | 14     |
| 20×20    | 22      | 18     |
| ...      | ...     | ...    |
| 144×144  | 1558    | 620    |

### Rectangular (6 symbols)

| Size     | Data CW | ECC CW |
|----------|---------|--------|
| 8×18     | 5       | 7      |
| 8×32     | 10      | 11     |
| 12×26    | 16      | 14     |
| 12×36    | 22      | 18     |
| 16×36    | 32      | 24     |
| 16×48    | 49      | 28     |

## Error classes

```
StandardError
  CodingAdventures::DataMatrix::DataMatrixError
    InputTooLongError   — input exceeds 144×144 capacity (1558 data codewords)
    InvalidSizeError    — forced size: does not match any ECC200 symbol size
```

## Development

```sh
bundle install
bundle exec rspec          # run tests
bundle exec standardrb     # lint
```
