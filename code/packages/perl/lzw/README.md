# lzw — LZW Lossless Compression Algorithm (Perl)

LZW (Lempel-Ziv-Welch, 1984) dictionary compression with variable-width bit packing.
Pre-initialized with all 256 single-byte codes. Part of the CMP compression series
in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                                       |
|-------|----------------|------|---------------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences                     |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window             |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits; no wasted literals              |
| CMP03 | **LZW**        | 1984 | Pre-initialized dictionary; powers GIF ← you are here |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE          |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib                 |

## Usage

```perl
use CodingAdventures::LZW qw(compress decompress);

my $data       = "ABABABABAB";
my $compressed = compress($data);
my $original   = decompress($compressed);  # "ABABABABAB"

# Low-level API
use CodingAdventures::LZW qw(encode_codes decode_codes pack_codes unpack_codes);

my @codes  = encode_codes($data);          # integer code array
my $packed = pack_codes(\@codes, length($data));   # CMP03 wire bytes
my ($codes_ref, $orig_len) = unpack_codes($packed);
my $result = decode_codes($codes_ref);     # original bytes
```

## API

| Function | Description |
|----------|-------------|
| `compress($data)` | Encode + pack to CMP03 wire format |
| `decompress($data)` | Unpack + decode from CMP03 wire format |
| `encode_codes($data)` | Encode byte string to array of integer codes |
| `decode_codes(\@codes)` | Decode integer codes to byte string |
| `pack_codes(\@codes, $orig_len)` | Bit-pack codes to CMP03 binary |
| `unpack_codes($binary)` | Unpack CMP03 binary to codes + orig_len |

## Wire Format (CMP03)

```
Bytes 0–3:  original_length (big-endian uint32)
Bytes 4+:   LSB-first variable-width bit-packed codes
```

Codes start at 9 bits wide and grow up to 16 bits as the dictionary expands.

## Reserved Codes

| Code | Meaning |
|------|---------|
| 0–255 | Pre-seeded single bytes |
| 256 | ClearCode — reset dictionary |
| 257 | StopCode — end of stream |
| 258+ | Dynamic dictionary entries |

## Development

```bash
cpanm --installdeps .
prove -l -v t/
```
