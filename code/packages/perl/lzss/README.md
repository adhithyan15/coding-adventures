# lzss — LZSS Lossless Compression Algorithm (Perl)

LZSS (Lempel-Ziv-Storer-Szymanski, 1982) sliding-window compression with flag-bit token
disambiguation. Part of the CMP compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                              |
|-------|----------------|------|------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences            |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window    |
| CMP02 | **LZSS**       | 1982 | LZ77 + flag bits, no wasted literals ← you are here |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; powers GIF  |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib        |

## Usage

```perl
use CodingAdventures::LZSS qw(encode decode compress decompress);

my $data       = "hello hello hello world";
my $compressed = compress($data);
my $original   = decompress($compressed);  # "hello hello hello world"

# Token-level API
my @tokens = encode($data);
my $result = decode(\@tokens);
```

## API

| Function | Description |
|----------|-------------|
| `encode($data, ...)` | Encode to token stream |
| `decode(\@tokens, $orig_len)` | Decode token stream |
| `compress($data, ...)` | Encode + serialise to CMP02 |
| `decompress($data)` | Deserialise + decode |
| `make_literal($byte)` | Create a Literal token |
| `make_match($offset, $length)` | Create a Match token |

### Tokens

- `{kind => 'literal', byte => integer}` — raw byte (0–255).
- `{kind => 'match', offset => integer, length => integer}` — back-reference.

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| window_size | 4096    | Maximum lookback distance. |
| max_match   | 255     | Maximum match length. |
| min_match   | 3       | Minimum match length for a Match token. |

## Development

```bash
cpanm --installdeps .
prove -l -v t/
```
