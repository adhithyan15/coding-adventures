# huffman-compression — CMP04 Huffman Entropy Compression (Perl)

Canonical Huffman coding (1952) with CMP04 wire format. Implements entropy
compression: assigns short bit codes to frequent symbols and long codes to rare
ones. Part of the CMP compression series in the coding-adventures monorepo.

Depends on [`CodingAdventures::HuffmanTree`](../huffman-tree) (DT27).

## In the Series

| Spec  | Algorithm      | Year | Key Concept                                           |
|-------|----------------|------|-------------------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences                         |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window                 |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits; no wasted literals                  |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; powers GIF                |
| CMP04 | **Huffman**    | 1952 | Entropy coding; prerequisite for DEFLATE ← you are here |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib                     |

## Usage

```perl
use CodingAdventures::HuffmanCompression qw(compress decompress);

my $data       = "AAABBC";
my $compressed = compress($data);
my $original   = decompress($compressed);  # "AAABBC"
```

## API

| Function | Description |
|----------|-------------|
| `compress($data)` | Encode bytes to CMP04 wire format |
| `decompress($data)` | Decode CMP04 wire format back to original bytes |

## Wire Format (CMP04)

```
Bytes 0–3:    original_length  (big-endian uint32)
Bytes 4–7:    symbol_count     (big-endian uint32)
Bytes 8–8+2N: code-lengths table — N entries × 2 bytes:
                [0] symbol value  (uint8, 0–255)
                [1] code length   (uint8, 1–16)
              Sorted by (code_length, symbol_value) ascending.
Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
```

## Development

```bash
PERL5LIB=$(cd ../huffman-tree && pwd)/lib:${PERL5LIB:-} prove -l -v t/
```
