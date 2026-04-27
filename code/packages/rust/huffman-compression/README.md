# huffman-compression — Huffman Compression (CMP04)

Huffman coding (1952) is an entropy coding algorithm that assigns variable-length,
prefix-free binary codes to symbols based on their frequency. Frequent symbols get
short codes; rare symbols get long codes. The resulting code is provably optimal for
a given symbol frequency distribution.

This crate delegates all Huffman tree construction and code derivation to
[`huffman-tree`](../huffman-tree) (DT27), mirroring the pattern where LZ78 (CMP01)
delegates to the `trie` crate (DT13).

## Wire Format (CMP04)

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32)
Bytes 8–8+2N: code-lengths table — N entries × 2 bytes: [symbol, code_length]
              Sorted by (code_length, symbol_value) ascending.
Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
```

The code-lengths table lets the decompressor reconstruct the exact canonical Huffman
codes without the full tree structure — the same trick DEFLATE uses.

## Usage

```rust
use huffman_compression::{compress, decompress};

let data = b"AAABBC";
let compressed = compress(data).unwrap();
assert_eq!(decompress(&compressed).unwrap(), data);
```

## Series

```
CMP00 (LZ77,    1977) — Sliding-window backreferences.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits.
CMP03 (LZW,     1984) — LZ78 + pre-initialised dict; powers GIF.
CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.  ← this crate
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
```

## Development

```bash
cargo test -p huffman-compression
```
