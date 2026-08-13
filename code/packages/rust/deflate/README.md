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

`compress` emits a standard raw RFC 1951 stream with no zlib or gzip wrapper.
It chooses the smaller of fixed and dynamic Huffman coding for its LZSS token
stream. `inflate` and `inflate_counted` accept stored, fixed, dynamic, and
multi-block streams with the full 32 KiB distance window.

## Strict counted inflate

```rust
use deflate::{inflate_counted, RAW_INFLATE_MAX_OUTPUT};

let input = [0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x90, 0x00,
             0xde, 0xad];
let result = inflate_counted(&input, RAW_INFLATE_MAX_OUTPUT)?;
assert_eq!(result.output, b"hello hello hello");
assert_eq!(result.bytes_consumed, 10);
# Ok::<(), deflate::InflateError>(())
```

The typed API validates its caller-selected output limit before allocation,
returns no partial output on error, and uses the stable payload-blind error
codes from `zip-owned-raw-rfc1951-v1`. The legacy `inflate` and `decompress`
functions remain string-returning compatibility wrappers with the 256 MiB
default ceiling.
