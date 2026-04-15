# lz77 — LZ77 Lossless Compression Algorithm (Rust)

LZ77 sliding-window compression algorithm (Lempel & Ziv, 1977). Part of the CMP compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                              |
|-------|----------------|------|------------------------------------------|
| CMP00 | **LZ77**       | 1977 | Sliding-window backreferences ← you are here |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window    |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits, no wasted literals     |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; powers GIF  |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib        |

## Usage

```rust
use lz77::{compress, decompress, encode, decode, Token};

// One-shot compression / decompression
let data = b"hello hello hello world";
let compressed = compress(data, 4096, 255, 3);
assert_eq!(decompress(&compressed), data);

// Token-level API
let tokens = encode(data, 4096, 255, 3);
let decoded = decode(&tokens, &[]);
assert_eq!(decoded, data);

// Inspect tokens
for t in &tokens {
    println!("offset={}, length={}, next_char={}", t.offset, t.length, t.next_char);
}
```

## API

| Function | Signature | Description |
|----------|-----------|-------------|
| `encode` | `(&[u8], usize, usize, usize) → Vec<Token>` | Encode to token stream |
| `decode` | `(&[Token], &[u8]) → Vec<u8>` | Decode token stream |
| `compress` | `(&[u8], usize, usize, usize) → Vec<u8>` | Encode + serialise |
| `decompress` | `(&[u8]) → Vec<u8>` | Deserialise + decode |

### Token

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub offset: u16,
    pub length: u8,
    pub next_char: u8,
}
```

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| window_size | 4096    | Maximum lookback distance. |
| max_match   | 255     | Maximum match length. |
| min_match   | 3       | Minimum match length for backreference. |

## Development

```bash
cargo test -p lz77 -- --nocapture
```

25 unit tests + 4 doc tests, all passing.
