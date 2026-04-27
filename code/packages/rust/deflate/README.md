# deflate (Rust)

**CMP05 — DEFLATE lossless compression (1996)**

## Usage

```rust
use deflate::{compress, decompress};

let data = b"hello hello hello world";
let compressed = compress(data).unwrap();
let original = decompress(&compressed).unwrap();
assert_eq!(original, data);
```

## Wire Format

```
[4B] original_length    big-endian uint32
[2B] ll_entry_count     big-endian uint16
[2B] dist_entry_count   big-endian uint16
[ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
[dist_entry_count × 3B] same format
[remaining bytes]       LSB-first packed bit stream
```
