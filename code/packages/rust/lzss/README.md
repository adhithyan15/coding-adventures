# lzss — LZSS Compression (CMP02)

LZSS (1982) refines LZ77 with flag bits: literals cost 1 byte, matches cost 3 bytes.

## Usage

```rust
use lzss::{compress, decompress};

let data = b"hello hello hello";
let compressed = compress(data);
assert_eq!(decompress(&compressed), data);
```

## Development

```bash
cargo test -p lzss
```
