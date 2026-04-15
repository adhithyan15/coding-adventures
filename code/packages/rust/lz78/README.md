# lz78 — LZ78 Lossless Compression Algorithm (Rust)

LZ78 (Lempel & Ziv, 1978) explicit-dictionary compression. Part of the CMP series.

## Usage

```rust
use lz78::{compress, decompress, encode, decode};

let data = b"hello hello hello";
let compressed = compress(data, 65536);
assert_eq!(decompress(&compressed), data);
```

## Development

```bash
cargo test -p lz78 -- --nocapture
```
