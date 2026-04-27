# pdf417 (Rust)

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes its geometry: each codeword contains
exactly 4 bars and 4 spaces (8 elements) and occupies exactly 17 modules of
horizontal space.

## Where PDF417 is used

| Application | Detail |
|---|---|
| AAMVA | North American driver's licences and government IDs |
| IATA BCBP | Airline boarding passes |
| USPS | Domestic shipping labels |
| US immigration | Form I-94, customs declarations |
| Healthcare | Patient wristbands, medication labels |

## How it fits in the stack

```text
raw bytes
  → pdf417 (this crate)   — byte compaction + RS ECC + rasterization
  → ModuleGrid             — abstract boolean grid (barcode-2d)
  → layout()               — pixel coordinates (barcode-2d)
  → PaintScene             — render instructions (paint-instructions)
  → paint backend          — SVG, Metal, Direct2D, terminal, …
```

## Usage

```rust
use pdf417::{encode, PDF417Options};

// Encode with automatic ECC level and dimensions.
let opts = PDF417Options::default();
let grid = encode(b"Hello, PDF417!", &opts).unwrap();
println!("{}x{} modules", grid.rows, grid.cols);

// Encode with explicit options.
let opts = PDF417Options {
    ecc_level:  Some(4),   // higher redundancy
    columns:    Some(6),   // 6 data columns
    row_height: Some(4),   // 4 module-rows per logical row
};
let grid = encode(b"Hello, PDF417!", &opts).unwrap();

// Produce a PaintScene for rendering.
use pdf417::encode_and_layout;
let scene = encode_and_layout(b"Hello, PDF417!", &PDF417Options::default(), None).unwrap();
println!("{}x{} px", scene.width, scene.height);
```

## Encoding pipeline

1. **Byte compaction** — codeword 924 latch. 6 bytes → 5 base-900 codewords;
   remaining bytes are direct (1 codeword each).
2. **Length descriptor** — first codeword = total codewords in the symbol.
3. **Reed-Solomon ECC** — GF(929) with b=3 convention, roots α³…α^{k+2}.
   Auto-level selection: level 2 for ≤40 data codewords, up to level 6.
4. **Dimension selection** — auto: `c = ⌈√(total/3)⌉`, `r = ⌈total/c⌉`,
   clamped to 3–90 rows × 1–30 columns.
5. **Padding** — codeword 900 fills any unused slots.
6. **Row indicators** — LRI and RRI per row encode row count, column count,
   and ECC level across a repeating 3-cluster cycle.
7. **Cluster table lookup** — each codeword maps to a 17-module bar/space
   pattern that differs by cluster, enabling single-row identification.
8. **Start/stop patterns** — `11111111010101000` (17 modules) and
   `111111101000101001` (18 modules) bracket every row.
9. **ModuleGrid** — boolean 2D grid output (`true` = dark module).

## v0.1.0 scope

This release implements **byte compaction only**. Text and numeric compaction
are planned for v0.2.0.

## Dependencies

- [`barcode-2d`](../barcode-2d) — `ModuleGrid` type and `layout()` function
- [`paint-instructions`](../paint-instructions) — `PaintScene` type

## Testing

```sh
cargo test -p pdf417
```

33 tests covering GF arithmetic, byte compaction, row indicators,
start/stop patterns, module dimensions, ECC bounds, integration tests,
error handling, determinism, and cross-row structural correctness.
