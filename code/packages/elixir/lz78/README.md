# LZ78 — Lossless Compression Algorithm (Elixir)

Elixir implementation of the LZ78 compression algorithm (Lempel & Ziv, 1978),
part of the CMP series in [coding-adventures](../../../../README.md).

## What Is LZ78?

LZ78 builds an explicit trie-based dictionary of byte sequences as it encodes.
Both encoder and decoder build the same dictionary independently, so no
dictionary needs to be transmitted — only the token stream.

Each **token** is a `(dict_index, next_char)` pair:
- `dict_index`: ID of the longest dictionary prefix matched so far (0 = literal)
- `next_char`: The byte that follows the match (0 = flush sentinel at end of input)

Compared to LZ77 (CMP00), which uses a fixed sliding window of recently seen
bytes, LZ78 grows a global dictionary that never forgets — making it well-suited
for highly repetitive data spread throughout the file. Its dictionary structure
directly inspired LZW (CMP03), used in GIF and TIFF.

## Usage

```elixir
alias CodingAdventures.LZ78

# One-shot compress/decompress
compressed = LZ78.compress("hello hello hello")
original   = LZ78.decompress(compressed)
# original == "hello hello hello"

# Token-level API
tokens = LZ78.encode("AABCBBABC")
# [%{dict_index: 0, next_char: 65}, %{dict_index: 1, next_char: 66}, ...]

data = LZ78.decode(tokens, 9)
# "AABCBBABC"
```

## TrieCursor

The `CodingAdventures.LZ78.TrieCursor` module is exported for reuse in other
streaming dictionary algorithms (LZW, etc.):

```elixir
alias CodingAdventures.LZ78.TrieCursor

cursor  = TrieCursor.new()
cursor2 = TrieCursor.insert(cursor, ?A, 1)

case TrieCursor.step(cursor2, ?A) do
  {:ok, advanced} -> TrieCursor.dict_id(advanced)  # => 1
  :miss            -> :not_found
end
```

## In the Series

| Spec  | Algorithm       | Year | Key Concept                            |
|-------|-----------------|------|----------------------------------------|
| CMP00 | LZ77            | 1977 | Sliding-window backreferences          |
| CMP01 | **LZ78**        | 1978 | Explicit dictionary (trie)             |
| CMP02 | LZSS            | 1982 | LZ77 + flag bits                       |
| CMP03 | LZW             | 1984 | LZ78 + pre-initialised alphabet; GIF   |
| CMP04 | Huffman Coding  | 1952 | Entropy coding                         |
| CMP05 | DEFLATE         | 1996 | LZ77 + Huffman; ZIP/gzip/PNG           |

## Development

```bash
mix deps.get
mix test
mix test --cover
```
