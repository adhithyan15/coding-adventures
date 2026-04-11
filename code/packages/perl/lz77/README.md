# lz77 — LZ77 Lossless Compression Algorithm (Perl)

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

```perl
use CodingAdventures::LZ77 qw(encode decode compress decompress);

# One-shot compression / decompression
my $data       = "hello hello hello world";
my $compressed = compress($data);
my $original   = decompress($compressed);

# Token-level API
my @tokens = encode($data);
my $decoded = decode(\@tokens);

# Each token is a hashref: {offset, length, next_char}
```

## API

| Function | Signature | Description |
|----------|-----------|-------------|
| `encode` | `($data, $window_size, $max_match, $min_match) → @tokens` | Encode to token list |
| `decode` | `(\@tokens, $initial_buffer) → $bytes` | Decode token list |
| `compress` | `($data, ...) → $bytes` | Encode + serialise |
| `decompress` | `($bytes) → $data` | Deserialise + decode |

### Token

```perl
{ offset => $int, length => $int, next_char => $byte }
```

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| window_size | 4096    | Maximum lookback distance. |
| max_match   | 255     | Maximum match length. |
| min_match   | 3       | Minimum match length for backreference. |

## Development

```bash
cpanm --installdeps .
prove -l -v t/
```

23 tests, all passing. Note: Windows not supported for Perl testing.
