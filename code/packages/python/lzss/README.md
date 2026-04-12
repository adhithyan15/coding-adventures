# LZSS — Lossless Compression Algorithm (CMP02)

LZSS (Lempel-Ziv-Storer-Szymanski, 1982) is a refinement of LZ77 that eliminates
a systematic waste: the mandatory `next_char` byte appended after every token. By
using flag bits to distinguish literals from back-references, LZSS makes literals
cost 1 byte and matches cost 3 bytes — compared to LZ77's flat 4 bytes per token.

This is part of the **CMP** (Compression) series in coding-adventures:

| Spec  | Algorithm      | Year | Key Idea                                     |
|-------|----------------|------|----------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences                |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie)                   |
| CMP02 | **LZSS**       | 1982 | LZ77 + flag bits — this package              |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; GIF              |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE     |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib            |

## Installation

```bash
pip install coding-adventures-lzss
```

## Usage

```python
from coding_adventures_lzss import compress, decompress, encode, decode, Literal, Match

# One-shot compress / decompress
data = b"hello hello hello"
compressed = compress(data)
assert decompress(compressed) == data

# Token-level API
tokens = encode(b"ABABAB")
# [Literal(byte=65), Literal(byte=66), Match(offset=2, length=4)]

decoded = decode(tokens, original_length=6)
# b'ABABAB'
```

## API

| Function / Type | Description |
|-----------------|-------------|
| `Literal(byte)` | A single literal byte token |
| `Match(offset, length)` | A back-reference token |
| `Token` | Union type alias: `Literal \| Match` |
| `encode(data, window_size=4096, max_match=255, min_match=3)` | Encode bytes to token list |
| `decode(tokens, original_length=-1)` | Decode token list to bytes |
| `compress(data, ...)` | Encode + serialise to CMP02 wire format |
| `decompress(data)` | Deserialise + decode from CMP02 wire format |

## Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `window_size` | 4096 | Maximum lookback distance (offset range) |
| `max_match` | 255 | Maximum match length (fits in uint8) |
| `min_match` | 3 | Minimum match length for a Match token |

## Development

```bash
uv venv .venv
uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
