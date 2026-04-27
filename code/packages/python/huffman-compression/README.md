# huffman-compression (CMP04)

Huffman (1952) lossless compression — entropy coding for the coding-adventures stack.

## What Is Huffman Compression?

Huffman coding assigns variable-length, prefix-free binary codes to symbols based on
their frequency. Frequent symbols get short codes; rare symbols get long codes. The
result is the theoretically optimal prefix-free code for a given symbol distribution.

This is fundamentally different from the LZ-family algorithms (CMP00–CMP03):

| Algorithm | Exploits | Saves via |
|---|---|---|
| LZ77/LZ78/LZSS/LZW | Repeated substrings | Back-references |
| Huffman | Symbol frequencies | Short codes for common symbols |

DEFLATE (CMP05) uses both: LZ77 removes repetition, then Huffman optimally encodes
the resulting token stream.

## Layer Position

```
DT04: heap             ← used inside DT27 construction
DT27: huffman-tree     ← builds tree and derives canonical codes
CMP04: huffman-compression  ← YOU ARE HERE
CMP05: deflate         ← combines LZ77 + Huffman
```

## Dependency

This package delegates **all tree construction and code derivation** to
`coding-adventures-huffman-tree` (DT27). It does not embed any tree logic.

## Wire Format (CMP04)

```
Offset  Size    Field
──────  ──────  ────────────────────────────────────────────
0       4       original_length   — BE uint32
4       4       symbol_count      — BE uint32 (1–256)
8       2×N     code-lengths table — sorted by (length, symbol)
                  [0] symbol value  (uint8)
                  [1] code length   (uint8)
8+2N    ⌈B/8⌉   bit stream — B total bits, packed LSB-first
```

## Usage

```python
from coding_adventures_huffman_compression import compress, decompress

data = b"AAABBC"
compressed = compress(data)
original  = decompress(compressed)
assert original == data
```

## Installation

```bash
pip install coding-adventures-huffman-compression
```

## Series

| ID | Algorithm | Year | What it exploits |
|---|---|---|---|
| CMP00 | LZ77 | 1977 | Sliding-window backreferences |
| CMP01 | LZ78 | 1978 | Explicit dictionary (trie) |
| CMP02 | LZSS | 1982 | LZ77 + flag bits |
| CMP03 | LZW | 1984 | LZ78 + pre-seeded alphabet (GIF) |
| **CMP04** | **Huffman** | **1952** | **Symbol frequencies (entropy coding)** |
| CMP05 | DEFLATE | 1996 | LZ77 + Huffman combined |
