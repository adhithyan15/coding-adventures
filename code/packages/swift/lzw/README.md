# LZW — CMP03

Swift implementation of LZW (Lempel-Ziv-Welch, 1984) lossless compression.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## What Is LZW?

LZW is LZ78 with a **pre-seeded dictionary**: all 256 single-byte sequences are
loaded before encoding begins (codes 0–255). This eliminates LZ78's mandatory
`next_char` byte — every possible byte is already in the dictionary, so the
encoder emits pure codes.

With only codes to transmit, LZW uses **variable-width bit-packing**: codes start
at 9 bits and grow as the dictionary expands. This is exactly how GIF works.

```
Series:
  CMP00 (LZ77,    1977) — Sliding-window backreferences.
  CMP01 (LZ78,    1978) — Explicit dictionary (trie).
  CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.  ← YOU ARE HERE
  CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
```

## Wire Format (CMP03)

```
Bytes 0–3:  original_length (big-endian UInt32)
Bytes 4+:   bit-packed variable-width codes, LSB-first
```

## Reserved Codes

| Code | Value | Meaning |
|------|-------|---------|
| CLEAR_CODE | 256 | Reset dictionary to initial 256-entry state |
| STOP_CODE  | 257 | End of compressed code stream |

## Usage

```swift
import LZW

// Compress
let original: [UInt8] = Array("hello hello hello".utf8)
let compressed = compress(original)

// Decompress
let restored = decompress(compressed)
// restored == original ✓
```

### Lower-Level API

```swift
// Encode to code sequence
let (codes, originalLength) = encodeCodes(data)

// Decode from code sequence
let bytes = decodeCodes(codes)

// Pack codes into wire format
let wireBytes = packCodes(codes, originalLength: originalLength)

// Unpack wire format to codes
let (unpackedCodes, storedLength) = unpackCodes(wireBytes)
```

## Algorithm

### Encoding

1. Seed dictionary with all 256 single-byte entries (codes 0–255).
2. Emit `CLEAR_CODE` at the start.
3. For each byte `b`, try to extend the current prefix `w` to `w + b`.
   - If `w + b` is in the dictionary: set `w = w + b`.
   - If `w + b` is not in the dictionary: emit `dict[w]`; add `w + b` to
     the dictionary; set `w = [b]`.
4. Flush the remaining prefix.
5. Emit `STOP_CODE`.

### Decoding

The decoder mirrors the encoder, rebuilding the dictionary entry-by-entry.
A famous edge case called the **tricky token** occurs when the encoder emits
a code that the decoder hasn't yet added:

```
entry = dict[prevCode] + [dict[prevCode][0]]
```

### Bit-Width Growth

Both encoder and decoder track `nextCode` and grow `codeSize` in lockstep:

```swift
nextCode += 1
if nextCode > (1 << codeSize) && codeSize < maxCodeSize {
    codeSize += 1
}
```

## Running Tests

```bash
# macOS
xcrun swift test --enable-code-coverage --verbose

# Linux
swift test --enable-code-coverage --verbose
```

## Parameters

| Constant | Value | Meaning |
|----------|-------|---------|
| `clearCode` | 256 | Reset code |
| `stopCode` | 257 | End-of-stream code |
| `initialNextCode` | 258 | First dynamic code |
| `initialCodeSize` | 9 | Starting bit-width |
| `maxCodeSize` | 16 | Maximum bit-width (dict caps at 65536) |
