# CodingAdventures::Zstd

ZStd (RFC 8878) lossless compression and decompression implemented from
scratch in pure Perl — part of the **CMP07** entry in the CodingAdventures
compression series.

## What is ZStd?

Zstandard (RFC 8878) is a high-ratio, fast lossless compression format
created by Yann Collet at Meta (Facebook) in 2015. It is the compression
algorithm behind Facebook's production data stores, Linux kernel firmware,
Python package wheels, and many other systems.

ZStd combines two ideas:

1. **LZ77 back-references** (via LZSS): find repeated byte sequences in the
   last 32 KB of output and encode them as (offset, length) pairs instead
   of copying the bytes verbatim.

2. **FSE (Finite State Entropy)**: encode the LZ77 descriptor symbols
   (literal length, match length, match offset) using asymmetric numeral
   systems — a modern entropy coder that approaches the Shannon limit in a
   single pass, outperforming Huffman coding for skewed distributions.

## Compression Series

```
CMP00 (LZ77)     — Sliding-window back-references
CMP01 (LZ78)     — Explicit dictionary (trie)
CMP02 (LZSS)     — LZ77 + flag bits                  ← dependency
CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman)  — Entropy coding
CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)   — DEFLATE + context modelling + static dict
CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed    ← this module
```

## Installation

```bash
cpanm --installdeps .
```

Or manually with the `lzss` dependency on the include path:

```bash
PERL5LIB=../lzss/lib prove -l -v t/
```

## Usage

```perl
use CodingAdventures::Zstd qw(compress decompress);

# Compress a string
my $data       = "the quick brown fox jumps over the lazy dog " x 100;
my $compressed = compress($data);
printf "Compressed %d → %d bytes (%.1f%%)\n",
    length($data), length($compressed),
    100 * length($compressed) / length($data);

# Decompress
my $original = decompress($compressed);
die "mismatch!" unless $original eq $data;

# Class method syntax also works
my $frame = CodingAdventures::Zstd->compress($data);
```

## Frame Format (RFC 8878 §3)

```
+--------+-----+--------------------+--------+
| Magic  | FHD | Frame_Content_Size | Blocks |
| 4 B LE | 1 B | 8 B LE             | ...    |
+--------+-----+--------------------+--------+
```

Each block has a 3-byte header encoding the block type:

| Block_Type | Meaning    | Description                                |
|-----------|------------|--------------------------------------------|
| `00`      | Raw        | Verbatim bytes — no compression            |
| `01`      | RLE        | One byte repeated N times                  |
| `10`      | Compressed | LZ77 sequences + FSE-coded descriptors     |
| `11`      | Reserved   | Invalid — decoder must reject              |

## Limitations

- Only **Predefined FSE mode** (RFC 8878 Appendix B) is implemented.
  Frames with per-frame FSE table descriptions will be rejected.
- Only **Raw_Literals** (type 0) is supported. Huffman-coded literals
  (types 2, 3) from other encoders will cause an error.
- No content checksum validation (flag is accepted but ignored).
- No custom dictionary support.
- Maximum decompressed output is capped at 256 MB to prevent bombs.

## Testing

```bash
PERL5LIB=../lzss/lib prove -l -v t/
```

19 subtests covering empty input, single byte, all 256 byte values,
RLE detection, prose compression ratio, random data round-trips,
multi-block (200 KB+, 300 KB), bad magic error detection, and internal
FSE/bit-stream unit tests.

## License

MIT
