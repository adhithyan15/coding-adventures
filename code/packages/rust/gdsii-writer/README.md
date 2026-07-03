# gdsii-writer

GDSII stream binary encoder for the silicon-stack pipeline. Produces Calma-format binary streams (the industry-standard layout interchange format used by all fabs).

## Pipeline position

```
asic-routing ──► gdsii-writer ──► [GDS binary] ──► tape-out
```

## What it does

- Encodes GDS records (HEADER, BGNLIB, BGNSTR, BOUNDARY, PATH, SREF, TEXT, ENDEL, …).
- Implements GDSII's unusual **8-byte base-16 floating-point** (7-bit excess-64 exponent, 56-bit mantissa) for UNITS records.
- Converts µm coordinates to database units (1 DBU = 1 nm; 1 µm = 1000 DBU).
- One `GdsCell` per stdcell footprint or top-level design.

## Key types

| Type | Description |
|------|-------------|
| `GdsWriter` | Library-level encoder; holds `cells` vec and library name |
| `GdsCell` | Structure with `boundaries`, `paths`, `srefs`, `texts` |
| `GdsBoundary` | Filled polygon (layer, datatype, XY) |
| `GdsPath` | Wire (layer, datatype, width, XY) |
| `GdsSref` | Structure reference (cell name, placement x/y) |
| `GdsText` | Text annotation |

## GDSII float format

```
byte 0:  sign (bit 7) | exponent (bits 6-0) in excess-64 base-16
bytes 1-7: 56-bit mantissa, MSB first, represents fraction in [1/16, 1)
value = sign × mantissa × 16^(exponent - 64)
```

## Usage

```rust
use gdsii_writer::{GdsWriter, GdsCell, GdsBoundary, stream::um_to_dbu};

let mut writer = GdsWriter::new("my_chip");
let mut cell = GdsCell::new("inv_1");
cell.boundaries.push(GdsBoundary {
    layer: 68, datatype: 20,
    xy: vec![(0,0), (um_to_dbu(0.46),0), (um_to_dbu(0.46),um_to_dbu(2.72)), (0,um_to_dbu(2.72)), (0,0)],
});
writer.cells.push(cell);
let gds_bytes: Vec<u8> = writer.encode();
```

## Testing

```
cargo test -p gdsii-writer -- --nocapture
```

14 tests covering: GDS real encoding (0, 1, -1, fractions), unit conversions, stream structure (HEADER magic bytes, ENDLIB tail), cell with boundary/path/sref/text, two-cell library.
