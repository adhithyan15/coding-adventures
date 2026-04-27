# coding-adventures-brotli

**CMP06** — Brotli compression algorithm (educational Ruby implementation).

Part of the [coding-adventures](https://github.com/adhithyan/coding-adventures)
monorepo compression series.

## What Is Brotli?

Brotli is a lossless compression algorithm developed at Google (2013, RFC 7932).
It achieves significantly better compression ratios than DEFLATE (CMP05),
particularly on web content. It is the standard for HTTP `Content-Encoding: br`
and the WOFF2 font format.

Brotli improves on DEFLATE with three major innovations:

1. **Context-dependent literal trees** — four separate Huffman trees for
   literals, selected based on the preceding byte category (space/punct, digit,
   uppercase, lowercase). Letters following a space have very different
   frequency distributions than letters following another letter.

2. **Insert-and-copy commands** — instead of separate literal and back-reference
   tokens, Brotli bundles the insert length and copy length into a single
   Huffman symbol (ICC code), reducing overhead.

3. **Larger sliding window** — 65535 bytes vs DEFLATE's 4096, enabling matches
   across longer distances in large documents.

## Usage

```ruby
require "coding_adventures_brotli"

original = "Hello, Brotli! " * 100
compressed = CodingAdventures::Brotli.compress(original)
recovered  = CodingAdventures::Brotli.decompress(compressed)

puts recovered == original.b  # => true
puts "#{(compressed.size * 100.0 / original.size).round(1)}% of original"
```

## API

### `CodingAdventures::Brotli.compress(data) → String`

Compress binary data using the CMP06 algorithm.

- `data` — any Ruby String (forced to binary encoding internally).
- Returns a binary String in CMP06 wire format.

### `CodingAdventures::Brotli.decompress(data) → String`

Decompress CMP06 wire-format data produced by `compress`.

- `data` — binary String from `compress`.
- Returns the original binary String.

## Wire Format

```
Header (10 bytes):
  [4B] original_length     big-endian uint32
  [1B] icc_entry_count     uint8
  [1B] dist_entry_count    uint8
  [1B] ctx0_entry_count    uint8
  [1B] ctx1_entry_count    uint8
  [1B] ctx2_entry_count    uint8
  [1B] ctx3_entry_count    uint8

ICC code-length table  (icc_entry_count × 2 bytes):
  [1B] symbol      ICC code 0–63
  [1B] code_length Huffman code length

Distance code-length table  (dist_entry_count × 2 bytes):
  same structure; omitted when no copy commands

Literal tree 0–3 code-length tables  (ctx_N_entry_count × 3 bytes each):
  [2B] symbol      big-endian uint16 (literal byte value 0–255)
  [1B] code_length Huffman code length

Bit stream (remaining bytes):
  LSB-first packed bits; zero-padded to byte boundary
```

## Dependencies

- `coding-adventures-huffman-tree` (DT27) — canonical Huffman tree builder.
  No other compression package is required; LZ matching is done inline.

## Compression Series

| Package | Algorithm | Window | Min match |
|---------|-----------|--------|-----------|
| CMP00   | LZ77      | 255 B  | 3         |
| CMP01   | LZ78      | —      | —         |
| CMP02   | LZSS      | 4 KB   | 3         |
| CMP03   | LZW       | —      | —         |
| CMP04   | Huffman   | —      | —         |
| CMP05   | DEFLATE   | 4 KB   | 3         |
| **CMP06** | **Brotli** | **64 KB** | **4** |
| CMP07   | Zstd      | 8 MB   | 3         |

## License

MIT
