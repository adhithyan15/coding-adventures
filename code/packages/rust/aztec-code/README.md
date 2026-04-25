# aztec-code

Aztec Code encoder — ISO/IEC 24778:2008 compliant.

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995. Unlike
QR Code (three corner finder patterns), Aztec places a single **bullseye** at
the symbol center. The scanner finds the center first and reads outward in a
clockwise spiral — no large quiet zone is needed.

## Usage

```rust
use aztec_code::{encode_str, AztecOptions};

// Encode a string (auto-selects compact vs full symbol)
let grid = encode_str("Hello, World!", None).unwrap();
println!("{}×{} symbol", grid.rows, grid.cols);

// Encode with custom ECC level
let opts = AztecOptions { min_ecc_percent: Some(33) };
let grid = encode_str("Hello, World!", Some(&opts)).unwrap();

// Encode raw bytes
use aztec_code::encode;
let bytes = [0x41u8, 0x42, 0x43];
let grid = encode(&bytes, None).unwrap();
```

## API

### `encode(data: &[u8], options: Option<&AztecOptions>) -> Result<ModuleGrid, AztecError>`

Encode raw bytes. Returns a `ModuleGrid` where `modules[row][col] == true` means dark.

### `encode_str(data: &str, options: Option<&AztecOptions>) -> Result<ModuleGrid, AztecError>`

Convenience wrapper that encodes a string as UTF-8 bytes.

### `encode_and_layout(data, options, config) -> Result<PaintScene, AztecError>`

Encode and convert to a `PaintScene` in one call.

### `AztecOptions`

```rust
pub struct AztecOptions {
    pub min_ecc_percent: Option<u8>, // default: 23, range: 10–90
}
```

## Symbol variants

| Variant | Layers | Sizes |
|---------|--------|-------|
| Compact | 1–4 | 15×15 to 27×27 |
| Full | 1–32 | 19×19 to 143×143 |

## Algorithm

See `src/lib.rs` for a fully annotated literate implementation. Key steps:

1. Binary-Shift encoding from Upper mode (byte-mode path, v0.1.0)
2. Symbol size selection (compact 1–4, then full 1–32 layers)
3. Padding to exact codeword count
4. GF(256)/0x12D Reed-Solomon ECC (b=1, same polynomial as Data Matrix)
5. Bit stuffing (complement bit after every 4 identical bits)
6. GF(16) mode message with RS protection
7. Grid initialization (reference grid, bullseye, orientation marks, mode msg)
8. Clockwise data spiral placement

## Dependencies

- `barcode-2d` — `ModuleGrid` type and `layout()` function
- `paint-instructions` — `PaintScene` type

## Tests

```bash
cargo test -p aztec-code
```

48 tests covering GF arithmetic, bit stuffing, structural properties, and the
cross-language test corpus.

## License

MIT
