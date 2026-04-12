# lz78 — LZ78 Lossless Compression Algorithm

LZ78 (Lempel & Ziv, 1978) is an explicit-dictionary compression algorithm. Unlike
LZ77 which searches a sliding window of recent bytes, LZ78 builds a trie of all
distinct sequences seen so far. Both encoder and decoder construct the same
dictionary independently — no dictionary is transmitted.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                                  |
|-------|----------------|------|----------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences                |
| CMP01 | **LZ78**       | 1978 | Explicit dictionary (trie) ← you are here    |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits, no wasted literals         |
| CMP03 | LZW            | 1984 | LZ78 + pre-initialized alphabet; powers GIF |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE     |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib            |

## Usage

```python
from coding_adventures_lz78 import compress, decompress, encode, decode, Token

# One-shot compression / decompression
data = b"hello hello hello world"
compressed = compress(data)
original   = decompress(compressed)
assert original == data

# Token-level API
tokens = encode(data)
for t in tokens:
    print(t.dict_index, t.next_char)

# Custom parameters
tokens2 = encode(data, max_dict_size=4096)
```

## API

| Function      | Signature                                          | Description                      |
|---------------|----------------------------------------------------|----------------------------------|
| `encode`      | `(bytes, max_dict_size=65536) → list[Token]`       | Encode to token stream           |
| `decode`      | `(list[Token], original_length=-1) → bytes`        | Decode token stream              |
| `compress`    | `(bytes, max_dict_size=65536) → bytes`             | Encode + serialise               |
| `decompress`  | `(bytes) → bytes`                                  | Deserialise + decode             |

### Token

```python
class Token(NamedTuple):
    dict_index: int   # 0 = literal; k > 0 = dictionary entry k
    next_char:  int   # byte following the match (0–255)
```

### Parameters

| Parameter     | Default | Meaning                                           |
|---------------|---------|---------------------------------------------------|
| max_dict_size | 65536   | Maximum dictionary entries. After this limit, new sequences are not recorded. |

## Development

```bash
cd code/packages/python/lz78
uv venv .venv --no-project --clear
uv pip install -e .[dev]
.venv/bin/python -m pytest tests/ -v
```
