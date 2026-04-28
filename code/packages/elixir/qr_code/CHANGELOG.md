# Changelog — coding_adventures_qr_code

All notable changes to this package are documented here.
This project follows [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-24

### Added

- **`CodingAdventures.QrCode.encode/2`** — main public API.  
  Encodes any UTF-8 string into a `%CodingAdventures.Barcode2D.ModuleGrid{}`.  
  Supports ECC levels `:l`, `:m`, `:q`, `:h` (default `:m`).  
  Returns `{:ok, grid}` or `{:error, :input_too_long}`.

- **`CodingAdventures.QrCode.Tables`** — ISO 18004:2015 lookup tables:  
  ECC codewords per block, number of RS blocks, alignment pattern positions,
  remainder bits, raw data module counts, data codeword counts, and RS
  generator polynomial builder.

- **`CodingAdventures.QrCode.Encoder`** — data encoding subsystem:  
  Mode selection (numeric / alphanumeric / byte), version selection,
  character count field widths, and full bit-stream assembly with
  mode indicator, char count, payload, terminator, and pad bytes.

- **`CodingAdventures.QrCode.RS`** — Reed-Solomon ECC subsystem:  
  LFSR-based RS encoding using the b=0 QR convention, block splitting,
  and round-robin interleaving of data and ECC codewords across blocks.

- **Full grid construction pipeline:**  
  Finder patterns (7×7), separators, timing strips, alignment patterns (v2+),
  format information reservation, version information reservation (v7+),
  always-dark module, and zigzag data placement.

- **Masking:** All 8 ISO 18004 mask patterns evaluated; lowest-penalty mask
  selected using the 4-rule penalty scoring system (runs, 2×2 blocks,
  finder-like patterns, dark ratio).

- **Format information:** BCH(15,5) error detection code with XOR mask 0x5412,
  written to both copy locations. Bit ordering follows the lessons.md
  correction: MSB-first (f14 at col 0) in row 8, per ISO 18004 §7.9.

- **Version information (v7+):** BCH(18,6) error detection code written to
  both 6×3 copy locations.

- **147 unit tests** covering all modules and integration paths. Test coverage:
  93.87% total (90.14% QrCode, 97.44% Tables, 98.88% Encoder, 100% RS).

### Dependencies

- `coding_adventures_barcode_2d` — `ModuleGrid` struct output type
- `coding_adventures_gf256` — GF(256) arithmetic for RS generator
- `coding_adventures_reed_solomon` — (transitive dependency)
- `coding_adventures_polynomial` — (transitive dependency)
- `coding_adventures_paint_instructions` — (transitive dependency)
