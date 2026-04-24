# aztec-code (Rust)

Aztec Code encoder conforming to **ISO/IEC 24778:2008**.

Aztec Code is the barcode on airline boarding passes (IATA), train tickets
(Eurostar, Amtrak), US driver's licences (AAMVA), and European postal labels.
Its single central bullseye finder pattern enables orientation detection without
corner markers, eliminating the need for a quiet zone.

## Stack position

```
barcode-2d (layout)
    │
aztec-code  ← this crate
```

## Usage

```rust
use aztec_code::{encode, encode_str, AztecOptions};

// Encode bytes to a ModuleGrid.
let grid = encode(b"https://example.com", None).unwrap();
println!("{}×{} Aztec Code", grid.rows, grid.cols);

// Encode a string.
let grid = encode_str("Hello World", None).unwrap();

// Options: force compact, higher ECC.
let opts = AztecOptions {
    min_ecc_percent: Some(50),
    compact: Some(true),
    ..Default::default()
};
let grid = encode_str("Hi", Some(&opts)).unwrap();
```

## API

### `encode(input: &[u8], options: Option<&AztecOptions>) → Result<ModuleGrid, AztecError>`

Encodes raw bytes into an Aztec Code `ModuleGrid`.

### `encode_str(input: &str, options: Option<&AztecOptions>) → Result<ModuleGrid, AztecError>`

Convenience wrapper around `encode` that accepts a `&str`.

### `encode_and_layout(input: &[u8], options, config: &Barcode2DLayoutConfig) → Result<PaintScene, AztecError>`

Encode and convert to a pixel-resolved `PaintScene` via `barcode_2d::layout()`.

### `AztecOptions`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_ecc_percent` | `Option<u32>` | `None` (→ 23) | ECC percentage (10–90). |
| `compact` | `Option<bool>` | `None` (→ false) | Force compact form. |

## Symbol structure

```
┌────────────────────────────┐
│         quiet zone         │
│  ┌──────────────────────┐  │
│  │     data layers      │  │
│  │  ┌────────────────┐  │  │
│  │  │  mode message  │  │  │
│  │  │  ┌──────────┐  │  │  │
│  │  │  │ bullseye │  │  │  │
│  │  │  └──────────┘  │  │  │
│  │  └────────────────┘  │  │
│  └──────────────────────┘  │
└────────────────────────────┘
```

- **Bullseye**: d ≤ 1 = solid 3×3 dark core; d ≥ 2 = odd → dark, even → light.
  Compact = radius 5 (11×11); Full = radius 7 (15×15).
- **Mode message ring**: 4 dark corner orientation marks + 28-bit (compact)
  or 40-bit (full) GF(16)-RS mode message.
- **Reference grid** (full only): alternating dark/light lines at ±16n from
  centre row/col.
- **Data layers**: clockwise 2-module-wide spiral bands radiating outward.

## Dependencies

- `barcode-2d` — `ModuleGrid` type and `layout()` function.
- `paint-instructions` — `PaintScene` type (transitive via `barcode-2d`).

GF(256)/0x12D and GF(16)/0x13 are implemented inline (the shared `gf256`
crate uses 0x11D, the QR Code polynomial, which is incompatible).

## Testing

```bash
cargo test -p aztec-code
```

40 unit tests covering GF arithmetic, mode message, bit stuffing, bullseye
structure, symbol sizing, and the full encode integration.
