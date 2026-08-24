# IC18 — `image-codec-png`: PNG Encoder/Decoder

## Overview

PNG (Portable Network Graphics, RFC 2083) is the lossless raster format the web
settled on. It is the first codec in this series where almost nothing in the file
is pixels, and that is what makes it worth implementing: three separate ideas are
stacked, and each has to be right before the next one means anything.

```
PNG file
  |
  +-- signature, then a sequence of CHUNKS          <- layer 1: framing
  |     each: length, 4-letter type, data, CRC-32
  |
  +-- the IDAT chunks' data, concatenated,
  |     is one ZLIB stream                          <- layer 2: RFC 1950
  |     header, DEFLATE stream, Adler-32
  |
  +-- which decompresses to FILTERED scanlines      <- layer 3: RFC 2083
        each row: 1 filter byte, then the row's bytes,
        each byte predicted from its neighbours
```

Compare the series so far. IC01 (BMP) is a header and then the pixels. IC02 (PPM)
is barely even that. IC03 (QOI) introduced real compression but invented its own
scheme in a single pass. PNG is the first format that reuses a general-purpose
compressor, and therefore the first that has to say precisely how the pixels are
*prepared* for it.

**A port MUST NOT carry its own DEFLATE.** The RFC 1951 bit stream inside `IDAT`
is the same one inside a ZIP entry, and the CRC-32 on a PNG chunk is the same
polynomial as the CRC-32 on a ZIP entry. Both already exist in the language's
`zip` package (CMP09). Duplicating either would create a second place for the
same class of bit-packing bug to hide. See CMP09's `raw_deflate` / `raw_inflate`
contract.

### Numbering note

IC00's roadmap table reserved IC04 for PNG. That number was later taken by
`image-codec-jpeg`, and the roadmap was not updated, so PNG fell out of the
series entirely while remaining a stated dependency of IC08 (`image-codec-ico`,
whose 256x256 frames are whole PNG files). This spec takes the next free number
and IC00's table is corrected to match the specs that actually exist.

---

## Layer 1: Chunks

Every PNG begins with the same eight bytes:

```
89  50 4E 47   0D 0A   1A   0A
^   P  N  G    \r \n   ^    ^
|                      |    |
|                      |    +-- LF, to catch the reverse conversion
|                      +------- DOS end-of-file, so TYPE stops here
+------------------------------ high bit set, to catch 7-bit transfers
```

Every byte is a scar from a real transfer bug. A decoder MUST reject a file whose
first eight bytes differ.

After the signature, chunks to the end of the file:

```
offset  size  field
0       4     length of DATA only (u32 BE)
4       4     type (4 ASCII bytes)
8       len   data
8+len   4     CRC-32 over TYPE + DATA (u32 BE) -- NOT over length
```

A decoder MUST verify every chunk CRC before using the chunk.

Three chunk types are required:

| Type | Position | Meaning |
|---|---|---|
| `IHDR` | **first**, exactly one | 13 bytes: dimensions and pixel format |
| `IDAT` | one or more, **consecutive** | the zlib stream, possibly split |
| `IEND` | **last**, **empty** | terminator |

Those emphases are normative and a decoder MUST enforce all four. Each describes
a file that decodes to exactly the right image while carrying bytes the image
does not need -- a chunk ahead of `IHDR`, an intervening chunk between two
`IDAT`s, a payload inside `IEND`, or anything after `IEND`. The picture is
identical either way, which is why tolerating them turns a valid-looking PNG
into free carriage: a scanner that renders the image sees nothing wrong.

**Chunk type case is a bitfield.** Bit 5 of each of the four letters is a flag,
and the first letter's is the one that matters here: uppercase means **critical**,
lowercase means **ancillary**. A decoder MUST skip an unknown ancillary chunk
(`gAMA`, `pHYs`, `tEXt`, ...) and MUST refuse the file on an unknown critical
one, because a critical chunk it does not understand may change what the image
means.

All four chunk-type bytes MUST be ASCII letters. The third letter's bit 5 is
reserved by PNG and MUST be zero, so the third letter MUST be uppercase. Reject
an invalid type before interpreting its critical/ancillary flag.

`acTL`, `fcTL`, and `fdAT` are ancillary in spelling but are the semantic
control and frame-data chunks of APNG. A decoder implementing this profile MUST
refuse each exact name as `unsupported-feature` after the ordinary chunk-type
and CRC checks and the existing first-chunk IHDR rule, regardless of its payload
or later position. It MUST NOT parse APNG state and MUST NOT let the generic
unknown-ancillary rule skip these chunks.

`PLTE` is a known critical chunk even when palette images are out of scope.
For truecolour types 2 and 6 it is an optional suggested palette: a decoder
MUST accept and ignore one well-formed table before `tRNS` and `IDAT`. It MUST reject a
duplicate, a table after `IDAT`, a table on greyscale types 0 or 4, or a length
that is not 1 to 256 complete three-byte RGB entries.

`tRNS` is ancillary in spelling but changes rendered pixels, so it MUST NOT be
silently skipped. Before `IDAT`, one `tRNS` supplies a two-byte transparent
greyscale sample for type 0 or three two-byte transparent samples for type 2.
At depth 8 each sample MUST fit in 0 through 255. A matching pixel receives
alpha 0 and every other pixel alpha 255. Reject `tRNS` on types 4 and 6, after
`IDAT`, with the wrong length, outside the active bit depth, or when repeated.

`IHDR` is:

```
offset  size  field
0       4     width  (u32 BE, non-zero)
4       4     height (u32 BE, non-zero)
8       1     bit depth
9       1     colour type
10      1     compression method -- 0 (deflate) is the only one defined
11      1     filter method      -- 0 (adaptive) is the only one defined
12      1     interlace method   -- 0 (none) or 1 (Adam7)
```

Colour types, and the channel count each implies:

| Type | Channels | Meaning |
|---|---|---|
| 0 | 1 | greyscale |
| 2 | 3 | truecolour (RGB) |
| 3 | 1 | palette index -- needs a `PLTE` chunk |
| 4 | 2 | greyscale + alpha |
| 6 | 4 | truecolour + alpha (RGBA) |

---

## Layer 2: The zlib wrapper

`IDAT` does not hold raw DEFLATE. It holds a zlib stream (RFC 1950):

```
CMF  FLG   <raw DEFLATE stream>   Adler-32 (u32 BE)
```

- `CMF` low nibble is the method (8 = deflate); high nibble is the window size.
- `CMF` high nibble (`CINFO`) MUST be at most 7. Larger values advertise a
  window above DEFLATE's 32 KiB maximum and MUST be rejected even when the
  mod-31 header check passes.
- `FLG` low five bits are chosen so `CMF * 256 + FLG` is a multiple of 31. Bit 5
  is `FDICT`, a preset dictionary, which PNG forbids.
- The trailing Adler-32 covers the **uncompressed** bytes.

Adler-32 is two running sums mod 65521 (the largest prime below 65536):

```
a = 1, b = 0
for each byte:  a = (a + byte) mod 65521
                b = (b + a)    mod 65521
result = (b << 16) | a
```

It is not CRC-32 and the two are not interchangeable. It is far weaker, and
deliberately so: the chunk CRC already does the real integrity work.

**Multiple `IDAT` chunks are one stream.** A split may fall anywhere, including
mid-symbol, so a decoder MUST concatenate all `IDAT` data before parsing any of
it.

**The compressed data MUST end exactly where the Adler-32 begins.** DEFLATE
announces its own end -- the last block sets BFINAL -- so a stream can finish
well before the checksum after it, and a decoder that only asks for the pixels
never looks at the gap. That gap is the `IDAT` cavity, and it is the same
carriage problem as the chunk rules above, one layer down. A decoder MUST
compare the bytes its inflater actually consumed against the length of the
region and reject any difference. This requires an inflate that reports its
consumption; CMP09 exports `raw_inflate_counted` for exactly this.

---

## Layer 3: Filtering

This is where PNG's compression actually comes from, and it is one idea: **a
pixel usually resembles the pixel to its left and the one above it.** So store
the difference from a prediction rather than the value. A smooth gradient becomes
a run of zeroes, and DEFLATE compresses runs of zeroes extremely well.

Each row chooses its own predictor and names it in a leading byte:

| # | Name | Filtered value |
|---|---|---|
| 0 | None | `x` |
| 1 | Sub | `x - a` |
| 2 | Up | `x - b` |
| 3 | Average | `x - floor((a + b) / 2)` |
| 4 | Paeth | `x - paeth(a, b, c)` |

where for byte `i` of the row: `a` is the byte one whole pixel earlier in the
same row, `b` the byte at the same position in the row above, and `c` the byte
one pixel earlier in the row above.

Three rules that are easy to get wrong and produce a plausible-looking but wrong
image rather than an error:

1. **Filters operate on BYTES, not pixels.** `a` is byte `i - bpp`, never byte
   `i - 1`, where `bpp` is bytes per pixel (channels, at 8-bit depth). Off the
   left edge, `a` and `c` are 0.
2. **The row above the first row is all zeroes**, which is what lets Up and Paeth
   apply to row 0 without a special case.
3. **The direction is asymmetric.** The encoder subtracts using the ORIGINAL
   neighbouring bytes; the decoder adds using bytes it has ALREADY reconstructed.
   Both see the same values only because the decoder proceeds strictly left to
   right, top to bottom.

All arithmetic is mod 256.

The Paeth predictor:

```
p  = a + b - c
pa = |p - a| ;  pb = |p - b| ;  pc = |p - c|
if pa <= pb and pa <= pc:  return a
if pb <= pc:               return b
return c
```

`a + b - c` is the value that would make the four bytes a parallelogram; the
function then returns whichever ACTUAL neighbour is nearest it, so the prediction
is always a real neighbouring value. That is what makes it good at edges. **The
tie-breaking order is normative**: an encoder and decoder that break ties
differently produce different images.

### Choosing a filter

The spec's own heuristic, and what every real encoder does: apply all five, sum
the absolute values of the filtered bytes read as SIGNED values, and keep the
smallest total. Ties keep the lowest-numbered filter. The signed reading is the
point -- a filtered byte of 255 means -1, a tiny correction, and reading it as
255 would rank the best filter worst.

Running DEFLATE five times per row would be more accurate and costs far more than
it saves.

---

## Encode Algorithm

```
1. Reject width or height of 0, non-integer or negative dimensions, and a data
   array whose length is not width * height * 4.
2. Emit the 8-byte signature.
3. Emit IHDR: width, height, bit depth 8, colour type 6, compression 0,
   filter 0, interlace 0.
4. For each row y:
     a. take the raw RGBA bytes of row y
     b. choose a filter against the previous RAW row (zeroes for y = 0)
     c. append the filter byte, then the filtered bytes
5. Compress the whole filtered stream with raw DEFLATE.
6. Emit IDAT: 0x78 0x9C, the DEFLATE bytes, then Adler-32 of the FILTERED
   (uncompressed) stream, big-endian.
7. Emit IEND, empty.
```

Colour type 6 at depth 8 is required for encoding because it is exactly what a
`PixelContainer` holds: no channel is dropped and no palette is guessed at, so
the round trip is lossless by construction rather than by luck.

## Decode Algorithm

```
1. Verify the signature.
2. Walk chunks:
     - reject a declared length larger than the remaining file BEFORE using it
       in any arithmetic
     - verify the CRC over type + data
     - IHDR: parse and validate; reject a second IHDR
     - PLTE: validate one optional suggested palette for types 2/6 before IDAT
     - tRNS: validate one optional transparency key for types 0/2 before IDAT
     - acTL, fcTL, fdAT: reject as the named unsupported APNG feature
     - IDAT: collect (must follow IHDR)
     - IEND: stop
     - otherwise: skip if ancillary (lowercase first letter), reject if critical
3. Require IHDR, IDAT and IEND to have been seen.
4. Concatenate all IDAT data. Validate the zlib header: method 8, the mod-31
   check, no preset dictionary.
5. Inflate, capped at exactly height * (width * channels + 1) bytes -- the size
   the header promises, so a bomb inside IDAT is stopped at the only size this
   image could need. Reject a result of any other length.
6. Verify the Adler-32 over the inflated bytes.
7. For each row: read its filter byte, undo the filter against the previous
   already-reconstructed row, then widen the row's channels into RGBA.
8. Widening: greyscale copies its one value into R, G and B; a missing alpha
   channel becomes 255.
```

### Required support

- **Encode:** 8-bit colour type 6, non-interlaced, one `IDAT`.
- **Decode:** 8-bit colour types 0, 2, 4 and 6, non-interlaced, any number of
  `IDAT` chunks, suggested `PLTE` for types 2/6, `tRNS` transparency for types
  0/2, and unknown non-semantic ancillary chunks skipped.

### Explicitly out of scope

Refused **by name**, never half-supported, because a decoder that silently
mis-reads a palette image is worse than one that says it cannot read it:

- palette images (colour type 3)
- bit depths other than 8
- Adam7 interlacing
- `APNG` animation (`acTL`, `fcTL`, and `fdAT`)

---

## Security requirements

PNG is a format a program reads from strangers, so these are normative.

1. **Bound the dimensions -- both edges AND the product.** `IHDR` is eight
   attacker-chosen bytes and `width * height * channels` is allocated on their
   word. A per-edge cap alone is not enough: 16384 x 16384 sits inside a 16384
   edge cap and is 268 million pixels, about 3 GiB once the container, the
   filtered buffer and the transient copy made while sizing it are counted.
   A port MUST cap the edges (16384, matching IC01) AND the total pixel count
   (32 mebipixels by default, a 128 MiB RGBA buffer), and SHOULD let the caller
   lower the latter and validate it where it is supplied. A caller-supplied
   ceiling MUST be a positive integer no larger than the 32 mebipixel default;
   it lowers the package ceiling and can never raise or fractionalize it.

   The pixel ceiling is a judgement, not a law, and a port should state its
   arithmetic: decoding costs roughly THREE times the pixel buffer -- the
   container, the filtered scanlines, and a transient copy while the latter is
   sized -- so 32 mebipixels admits about 400 MB of peak allocation. That is
   chosen to still read an ordinary camera photograph while sitting an order of
   magnitude below what a per-edge cap alone permits. An inflate that can return
   its buffer without a final trimming copy removes one of the three.

   This is where PNG parts company with BMP. BMP survives on an edge cap because
   its pixels have to BE in the file, so demanding a gigabyte of memory costs a
   gigabyte of upload. PNG compresses, and at DEFLATE's 1032:1 the same demand
   costs about one megabyte. The amplification is the whole difference.
2. **Cap decompression at the header's own promise.** The inflate call MUST be
   given `height * (width * channels + 1)` as its ceiling. DEFLATE's expansion
   ratio reaches 1032:1, so an unbounded inflate of a hostile `IDAT` is a
   denial-of-service with a few hundred kilobytes of input.
3. **Validate the chunk length before arithmetic on it**, not after.
4. **Verify every CRC**, and verify the Adler-32. A decoder that skips them
   silently produces wrong pixels.
5. **Malformed input MUST throw**, never return partial or approximate output.

---

## API

```
encode_png(pixels: PixelContainer) -> bytes
decode_png(bytes) -> PixelContainer
adler32(bytes) -> u32          # exported: it is testable on its own
PngCodec implements ImageCodec # mime type "image/png"
PngError(code, message)         # stable portable code plus explanatory text
```

The package MUST also expose the default edge and pixel ceilings and the closed
set of portable error identifiers. This lets an embedder and a neutral fixture
reason about the same boundaries without parsing prose or exception messages.

### Portable error taxonomy

Every malformed-input or invalid-option failure MUST use one of these stable,
payload-blind identifiers. Messages may add the bounded numbers or chunk type
called for by the error table, but consumers MUST branch on the identifier, not
the message.

| Identifier | Boundary |
|---|---|
| `invalid-max-pixels` | caller ceiling is non-integer, non-positive, non-finite, or above the default |
| `invalid-image-dimensions` | encoder dimensions are invalid or empty |
| `invalid-pixel-data-length` | encoder RGBA byte count disagrees with dimensions |
| `file-too-short` | input cannot contain the complete PNG signature |
| `invalid-signature` | signature differs |
| `truncated-chunk` | chunk header/data/CRC extends beyond input |
| `invalid-chunk-type` | type contains a non-letter or a lowercase reserved third letter |
| `chunk-crc-mismatch` | chunk CRC differs |
| `chunk-before-ihdr` | any chunk precedes IHDR |
| `duplicate-ihdr` | a second IHDR appears |
| `invalid-ihdr-length` | IHDR is not 13 bytes |
| `invalid-dimensions` | decoded width or height is zero |
| `dimension-limit` | decoded edge exceeds 16,384 |
| `pixel-limit` | decoded product exceeds the active ceiling |
| `unsupported-feature` | unsupported compression/filter method, depth, colour type, palette, or interlace |
| `invalid-plte` | PLTE is repeated, misplaced, malformed, or forbidden for the colour type |
| `invalid-trns` | tRNS is repeated, misplaced, malformed, out of range, or forbidden for the colour type |
| `nonconsecutive-idat` | an IDAT appears after the IDAT run ended |
| `invalid-iend` | IEND is non-empty |
| `trailing-data` | bytes follow IEND |
| `unknown-critical-chunk` | an unrecognised critical chunk appears |
| `missing-required-chunk` | IHDR, IDAT, or IEND is absent |
| `invalid-zlib-header` | zlib stream is too short, has the wrong method/CINFO, or fails FCHECK |
| `preset-dictionary` | zlib FDICT is set |
| `inflate-failed` | the ZIP-owned raw inflater rejects the DEFLATE stream |
| `inflated-length-mismatch` | decompressed byte count differs from IHDR's exact promise |
| `idat-cavity` | whole bytes remain between BFINAL and Adler-32 |
| `adler-mismatch` | Adler-32 differs |
| `invalid-filter` | a scanline filter byte is above 4 |

## Error Cases

| Condition | Behaviour |
|---|---|
| file shorter than the signature | error "too short" |
| signature mismatch | error "invalid signature" |
| chunk length exceeds file size | error, before allocating |
| chunk CRC mismatch | error naming the chunk type |
| unknown critical chunk | error naming the type |
| missing IHDR, IDAT or IEND | error naming which |
| second IHDR | error |
| IDAT before IHDR | error |
| colour type 3, depth != 8, interlace 1 | error naming the unsupported feature |
| `acTL`, `fcTL`, or `fdAT` | `unsupported-feature`, after type and CRC validation |
| invalid, repeated, misplaced, or forbidden `PLTE` / `tRNS` | stable typed error |
| dimension 0, or above the cap | error |
| zlib header not method 8, or failing mod 31 | error |
| zlib CINFO above 7 | error |
| zlib preset dictionary requested | error |
| inflated length != the header's promise | error |
| Adler-32 mismatch | error |
| filter byte above 4 | error naming the value |
| any chunk before `IHDR` | error naming the type |
| bytes after `IEND` | error naming the count |
| non-empty `IEND` | error |
| non-consecutive `IDAT` chunks | error |
| unused bytes between the DEFLATE stream and the Adler-32 | error naming the count |
| pixel count above the cap | error naming both numbers |
| encoding a 0-pixel image | error |
| encoding data of the wrong length | error naming both lengths |
| caller `maxPixels` is fractional or above 32 mebipixels | error |

## Round-Trip Property

For any `PixelContainer` with width and height >= 1:

```
decode_png(encode_png(c)) == c      # byte-for-byte, all four channels
```

## Test Requirements

Round-trip tests prove the encoder and decoder agree with each other, which is
necessary and nowhere near sufficient -- two halves of one misunderstanding round
-trip perfectly. A conforming port MUST also test against a **foreign**
implementation:

- inflate the encoder's `IDAT` with the platform's own zlib and check the
  scanline count and filter bytes;
- decode PNGs assembled by hand from this spec and compressed by the platform's
  zlib, covering **all five filter types** and each supported colour type;
- check `adler32` against a known vector (`"Wikipedia"` -> `0x11E60398`) and
  against the trailer the platform's zlib writes, including across the 5552-byte
  chunking boundary;
- confirm the written file is accepted by at least one real image tool.

The repository-owned portable corpus is
`code/specs/fixtures/image-codec-png-v1/cases.json`. Every port MUST consume the
same document through its public API and assert the exact stable error
identifier for rejection cases. Fixture vectors use deterministic stored and
fixed encoders plus a checked independent dynamic-Huffman stream; Python zlib
independently decodes them. A package's own round trip is never sufficient
evidence. Encode cases also pin the per-row filter choices without pinning the
compressed bytes.

Production is a pure in-memory byte transform. Every package MUST carry
`required_capabilities.json` with an empty `capabilities` array. Filesystem,
process, environment, time, entropy, network, and native image-tool access are
test-only evidence and are not part of the codec API.

---

## Package Layout

```
code/packages/<language>/image-codec-png/
  src/                  encoder, decoder, adler32
  tests/
  BUILD                 chain-installs pixel-container, lzss, zip, then self
  README.md
  CHANGELOG.md
  required_capabilities.json
```

**Dependencies:** `pixel-container` (IC00) for the pixel type, and the language's
`zip` (CMP09) for `raw_deflate`, `raw_inflate` and `crc32`. No others.
