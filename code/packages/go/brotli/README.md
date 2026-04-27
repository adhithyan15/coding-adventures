# coding-adventures-brotli (Go)

Go implementation of **CMP06: Brotli** lossless compression, part of the
[CodingAdventures](https://github.com/adhithyan15/coding-adventures) series.

## What is Brotli?

Brotli (2013, RFC 7932) is a lossless compression algorithm developed at Google
that achieves significantly better compression ratios than DEFLATE, particularly
on web content. It is the standard for HTTP `Content-Encoding: br` and the WOFF2
font format.

CMP06 builds on DEFLATE (CMP05) with three key innovations:

1. **Context-dependent literal trees** — Four Huffman trees, one per context
   bucket, based on the character class of the preceding byte. After a space,
   English words typically start with consonants; after 't', 'h' is very likely.
   Separate trees capture these distributions precisely.

2. **Insert-and-copy commands** — One ICC symbol encodes both an insert run
   length and a copy length, reducing the overhead of separate literal and match
   tokens.

3. **Larger sliding window** — 65535 bytes instead of DEFLATE's 4096, allowing
   matches across longer distances.

This CodingAdventures implementation omits the real RFC 7932 static dictionary
(122,784 entries of common English word forms) to keep implementations tractable
and consistent across all 9 languages.

## Position in the series

```
CMP00 (LZ77,    1977) — Sliding-window backreferences.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
CMP06 (Brotli,  2013) — Context modeling + insert-copy + large window.  ← this package
CMP07 (Zstd,    2016) — ANS/FSE + LZ4 matching; modern universal codec.
```

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/brotli"

// Compress data.
compressed, err := brotli.Compress([]byte("hello world hello world"))
if err != nil {
    log.Fatal(err)
}

// Decompress data.
original, err := brotli.Decompress(compressed)
if err != nil {
    log.Fatal(err)
}
fmt.Println(string(original)) // "hello world hello world"
```

## Wire Format

```
Header (10 bytes):
  Bytes 0–3:  original_length    big-endian uint32
  Byte  4:    icc_entry_count    uint8 (entries in ICC code-length table)
  Byte  5:    dist_entry_count   uint8 (entries in dist code-length table; 0 if no copies)
  Byte  6:    ctx0_entry_count   uint8 (literal tree 0: space/punct context)
  Byte  7:    ctx1_entry_count   uint8 (literal tree 1: digit context)
  Byte  8:    ctx2_entry_count   uint8 (literal tree 2: uppercase context)
  Byte  9:    ctx3_entry_count   uint8 (literal tree 3: lowercase context)

ICC code-length table (icc_entry_count × 2 bytes):
  [1B] symbol, [1B] code_length — sorted by (len ASC, symbol ASC)

Dist code-length table (dist_entry_count × 2 bytes):
  [1B] symbol, [1B] code_length — sorted by (len ASC, symbol ASC)

Literal tree 0–3 (× 3 bytes each):
  [2B] symbol (big-endian uint16), [1B] code_length

Bit stream: LSB-first packed bits, zero-padded to byte boundary.

Encoding order per regular command (copy_length > 0):
  [ICC code] [insert extras] [copy extras] [literals...] [dist] [dist extras]

End of stream:
  [ICC=63]  [flush literals, if any]
```

### Flush Literals

Trailing literals that cannot be bundled into a regular ICC command (because
no ICC code has copy_length=0) are emitted as "flush literals" after the
sentinel ICC=63. The decoder reads these after the sentinel until
`original_length` bytes have been produced. This cleanly handles pure-literal
inputs (no LZ matches) of any length without requiring dummy copies.

## Dependencies

- [`coding-adventures-huffman-tree`](../huffman-tree) (DT27) — canonical Huffman
  tree builder, shared with CMP04 and CMP05.

## Running tests

```sh
go test ./... -v -cover
```

## Context bucket assignment

```
literalContext(lastByte):
  if no previous byte → bucket 0
  'a'–'z'  → bucket 3  (lowercase)
  'A'–'Z'  → bucket 2  (uppercase)
  '0'–'9'  → bucket 1  (digit)
  otherwise → bucket 0  (space/punctuation)
```
