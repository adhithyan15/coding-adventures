# lzw — LZW Compression (CMP03)

LZW (Lempel-Ziv-Welch, 1984) refines LZ78 by pre-seeding the dictionary with all 256
single-byte sequences. Because every byte already has a code (0–255), the encoder emits
only dictionary codes — no raw literals. This enables variable-width bit-packing (9–16
bits per code), the scheme used in GIF, TIFF, and Unix `compress`.

## Usage

```rust
use lzw::{compress, decompress};

let data = b"hello hello hello";
let compressed = compress(data);
assert_eq!(decompress(&compressed).unwrap(), data);
```

## Wire Format (CMP03)

```
Bytes 0–3:  original_length (big-endian u32)
Bytes 4+:   LSB-first bit-packed variable-width codes (9–16 bits each)
```

## Development

```bash
cargo test -p lzw
```
