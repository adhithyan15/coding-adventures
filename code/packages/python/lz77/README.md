# LZ77 — Lossless Compression Algorithm

Pure Python implementation of the 1977 Lempel-Ziv sliding-window compression algorithm. This is the ancestor of LZSS, LZW, DEFLATE, zstd, LZ4, and virtually every modern compressor used in ZIP, gzip, PNG, and zlib.

## What Is LZ77?

LZ77 is the foundational compression algorithm that exploits locality in data. Instead of storing every byte verbatim, it notices when a sequence of bytes has appeared recently and replaces it with a cheap reference: the offset (how far back) and length (how many bytes).

### Key Concepts

- **Sliding window**: The encoder maintains a window of recently seen bytes (the search buffer) and tries to match the current position against it.
- **Token triple**: `(offset, length, next_char)` — where `offset` is how many bytes back the match starts, `length` is how many bytes it covers, and `next_char` is the literal byte that follows.
- **Overlapping matches**: A match can extend into bytes that haven't been written yet, allowing for automatic run-length encoding of repeating patterns.

## How It Works

The encoder processes input left-to-right:

1. At each position, scan the search buffer (last `window_size` bytes) for the longest substring that matches the start of the lookahead buffer.
2. If a match is found and is at least `min_match` bytes long, emit a backreference token.
3. Otherwise, emit a literal token for the current byte.

Example: compressing `ABABABAB`

```
Step 1: Emit literal A → output [A]
Step 2: Emit literal B → output [A, B]
Step 3: Find match: offset=2 (back to position 0), length=5 (matches ABABA) → emit (2, 5, 'B')
        Output: [A, B, A, B, A, B, A, B]
```

**Result: 3 tokens instead of 8 bytes.**

## In the Series

This is **CMP00** in the compression series:

| Spec | Algorithm     | Year | Key Idea                                      |
|------|---------------|------|-----------------------------------------------|
| CMP00 | LZ77         | 1977 | Sliding-window backreferences                 |
| CMP01 | LZ78         | 1978 | Explicit dictionary (trie), no sliding window |
| CMP02 | LZSS         | 1982 | LZ77 + flag bits; no wasted literal byte      |
| CMP03 | LZW          | 1984 | Pre-initialized dictionary; used in GIF       |
| CMP04 | Huffman      | 1952 | Entropy coding; prerequisite for DEFLATE      |
| CMP05 | DEFLATE      | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib             |

## Usage

### One-Shot API

```python
from coding_adventures_lz77 import compress, decompress

original = b"ABABABAB"
compressed = compress(original)
decompressed = decompress(compressed)

assert decompressed == original
```

### Streaming API (Token Manipulation)

```python
from coding_adventures_lz77 import encode, decode, Token

# Encode to tokens.
tokens = encode(b"HELLO")
print(tokens)  # [Token(0,0,72), Token(0,0,69), Token(0,0,76), Token(0,0,76), Token(0,0,79)]

# Manipulate tokens (if needed).
# ...

# Decode back.
result = decode(tokens)
assert result == b"HELLO"
```

## API

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `encode` | `(data: bytes, window_size=4096, max_match=255, min_match=3) -> list[Token]` | Tokenise input into LZ77 token stream. |
| `decode` | `(tokens: list[Token], initial_buffer=b"") -> bytes` | Reconstruct bytes from token stream. |
| `compress` | `(data: bytes, window_size=4096, max_match=255, min_match=3) -> bytes` | Encode then serialise tokens to bytes. |
| `decompress` | `(data: bytes) -> bytes` | Deserialise and decode back to original bytes. |

### Types

| Type | Definition |
|------|-----------|
| `Token` | Named tuple `(offset: int, length: int, next_char: int)` |

### Parameters

| Parameter | Default | Meaning |
|-----------|---------|---------|
| `window_size` | 4096 | Maximum offset (bytes back). Larger = better compression, more memory. |
| `max_match` | 255 | Maximum match length. Limited by token format. |
| `min_match` | 3 | Minimum match length to emit a backreference (break-even point). |

## Development

### Install Dev Dependencies

```bash
uv pip install -e ".[dev]"
```

### Run Tests

```bash
pytest tests/ -v
```

### Coverage

All code must have at least 80% test coverage (enforced by pytest). The implementation targets 95%+.

```bash
pytest tests/ --cov=coding_adventures_lz77 --cov-report=term-missing
```

### Linting

```bash
ruff check src/ tests/
```

Type annotations are required (ANN rule enforced by ruff).

## Reference

Lempel, A., & Ziv, J. (1977). "A Universal Algorithm for Sequential Data Compression". IEEE Transactions on Information Theory, 23(3), 337–343.

See also: `code/specs/CMP00-lz77.md` for the full specification and worked examples.
