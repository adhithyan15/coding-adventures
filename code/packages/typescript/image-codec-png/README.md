# @coding-adventures/image-codec-png

PNG encoder and decoder implemented from scratch in TypeScript — **IC18** in the
image-codec series, specified in
[`IC18`](../../../specs/IC18-image-codec-png.md).

## What it does

Turns a `PixelContainer` into a real `.png` file, and back. The output is
accepted by `file`, macOS Preview, browsers, and XeLaTeX; the input may be any
non-interlaced 8-bit PNG.

```typescript
import { createPixelContainer, setPixel } from "@coding-adventures/pixel-container";
import { encodePng, decodePng } from "@coding-adventures/image-codec-png";

const c = createPixelContainer(2, 1);
setPixel(c, 0, 0, 255, 0, 0, 255);   // red
setPixel(c, 1, 0, 0, 0, 255, 255);   // blue

const png = encodePng(c);
decodePng(png);                       // the same pixels, byte for byte
```

## Why PNG is three formats in a trench coat

BMP is a header followed by pixels, and that is the whole story. PNG is the
opposite — almost nothing in a PNG file is pixels:

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

**The hardest layer is not in this package.** RFC 1951 DEFLATE lives in
[`@coding-adventures/zip`](../zip), which needed the identical bit stream for ZIP
entries and exports it as `rawDeflate`/`rawInflate`. The CRC-32 comes from there
too, because PNG chunks and ZIP entries use the same polynomial — the doc comment
on `crc32` has said "ZIP/gzip/PNG/zlib" since before this package existed. So
what is here is the two wrappers around DEFLATE, plus the filtering.

### Where it fits

```
IC00 (pixel-container)   — the RGBA8 buffer            <- dependency
CMP09 (zip)              — RFC 1951 DEFLATE + CRC-32   <- dependency
IC18 (this package)      — chunks + zlib + filters
```

## The signature is eight bytes of scar tissue

```
89  P  N  G  \r  \n  1A  \n
```

The high bit in `0x89` catches a transfer that stripped the eighth bit. The
`\r\n` catches one that "helpfully" converted line endings — and the trailing
`\n` catches the reverse conversion. The `1A` is DOS end-of-file, so `TYPE
image.png` stops there instead of spraying binary at a terminal. Every byte is a
bug someone had to debug first.

## Filtering, which is where the compression comes from

One idea: **a pixel usually resembles the pixel to its left and the pixel above
it.** So store the difference from a prediction rather than the value. A smooth
gradient becomes a run of zeroes, and DEFLATE compresses runs of zeroes
extremely well. That is the whole reason a PNG beats a zipped BMP.

Each row picks its own predictor and names it in a leading byte:

| # | Name | Stores | Good at |
|---|---|---|---|
| 0 | None | the byte | already-random data |
| 1 | Sub | byte − left | horizontal runs |
| 2 | Up | byte − above | vertical runs |
| 3 | Average | byte − ⌊(left + above)/2⌋ | smooth gradients |
| 4 | Paeth | byte − nearest of left/above/upper-left | edges and diagonals |

Three details that produce a *plausible but wrong* image rather than an error:

1. **Filters work on bytes, not pixels.** "The byte to the left" means byte
   `i − bpp`, not `i − 1`. Off the left edge, zero.
2. **The row above row 0 is all zeroes**, which is what lets Up and Paeth apply
   to the first row with no special case.
3. **The direction is asymmetric.** The encoder subtracts using *original*
   neighbouring bytes; the decoder adds using bytes it has *already restored*.
   They agree only because the decoder goes strictly left to right, top to
   bottom.

Filter choice per row uses the PNG spec's own heuristic: apply all five, sum the
results read as **signed** bytes, keep the smallest. The signed reading is the
point — a filtered byte of 255 means −1, a tiny correction, and reading it as 255
would rank the best filter worst.

On a 200×120 gradient with a circle, that picks Paeth for 101 rows, Up for 14 and
Sub for 5, and compresses 96,000 bytes to 2,123 — **45:1**.

## Supported

|  | Encode | Decode |
|---|---|---|
| 8-bit truecolour + alpha (type 6) | ✅ | ✅ |
| 8-bit truecolour (type 2) | — | ✅ |
| 8-bit greyscale (type 0) | — | ✅ |
| 8-bit greyscale + alpha (type 4) | — | ✅ |
| multiple `IDAT` chunks | writes one | ✅ reads any number |
| unknown **ancillary** chunks (`gAMA`, `tEXt`, …) | — | ✅ skipped |
| unknown **critical** chunks | — | ❌ refused, as the spec requires |
| suggested `PLTE` on truecolour types 2/6 | — | ✅ validated and ignored |
| `tRNS` transparency on types 0/2 | — | ✅ applied to output alpha |

Encoding always writes colour type 6, because that is exactly what a
`PixelContainer` holds: no channel is dropped and no palette is guessed at, so
the round trip is lossless by construction rather than by luck.

**Refused by name, never half-supported:** palette images (type 3), bit depths
other than 8, Adam7 interlacing, APNG animation. A decoder that silently
mis-reads a palette image is worse than one that says it cannot read it.

## Reading bytes you did not write

`decodePng` parses hostile input, so:

- **Malformed input always throws** — never partial or approximate output.
- Every chunk CRC and the trailing Adler-32 are verified.
- A chunk's declared length is checked against the file size **before** any
  arithmetic uses it.
- Each edge is capped at 16,384 pixels **and the total at 32 mebipixels**,
  because `IHDR` is eight attacker-chosen bytes and `width × height × 4` is
  allocated on their word. An edge cap alone is not enough: 16384 × 16384 sits
  inside it and is 268 million pixels, roughly 3 GiB of peak allocation — which
  at DEFLATE's 1032:1 costs the attacker about a megabyte. BMP survives on an
  edge cap because its pixels have to *be* in the file; PNG compresses, and that
  amplification is the whole difference. Lower it with
  `decodePng(bytes, { maxPixels })` or `new PngCodec({ maxPixels })` — decoding
  costs about three times the pixel buffer, so the default admits roughly
  400 MB of peak allocation, and a caller who knows its images are small should
  say so. The supplied ceiling must be a positive safe integer no larger than
  the 32 mebipixel default: callers may lower the budget, never raise or
  fractionalize it.
- **`IHDR` must be first, `IEND` must be empty and last, `IDAT` chunks must be
  consecutive, and the compressed data must end exactly where the Adler-32
  begins.** Each of those
  describes a file that decodes to precisely the right image while carrying
  bytes the image does not need. The picture is identical either way, which is
  why tolerating them turns a valid-looking PNG into free carriage: a scanner
  that renders the image sees nothing wrong. The last one is the *`IDAT`
  cavity* — DEFLATE announces its own end with BFINAL, so a stream can stop
  early and everything up to the checksum is dead space a decoder asking only
  for pixels never looks at.
- Inflation is capped at **exactly** the size `IHDR` promises. DEFLATE's
  expansion ratio reaches 1032:1, so an uncapped inflate of a hostile `IDAT` is
  a denial-of-service with a few hundred kilobytes of input.

## Testing

Round-trip tests prove the encoder and decoder agree with each other, which is
necessary and nowhere near sufficient — two halves of one misunderstanding
round-trip perfectly. So the suite also:

- inflates the encoder's `IDAT` with **Node's own zlib** and checks the scanline
  count and filter bytes;
- decodes PNGs assembled **by hand from RFC 2083** and compressed by Node's zlib,
  covering all five filter types in one image and each supported colour type;
- checks `adler32` against the RFC's worked example and against the trailer zlib
  itself writes, including across the 5552-byte chunking boundary.
- consumes all 82 cases in the
  [`image-codec-png-v1`](../../../specs/fixtures/image-codec-png-v1/README.md)
  language-neutral corpus through the public API, including every stable error
  code, all supported colour forms and filters, and stored/fixed/dynamic
  DEFLATE;
- decodes the encoder's output with test-only `pngjs`, a foreign PNG
  implementation, rather than trusting this package's own decoder.

The written file has been confirmed readable by `file`, macOS `sips`, and
Python's `zlib`.

```bash
npm install
npx vitest run --coverage
```

## API

| Symbol | Description |
|---|---|
| `encodePng(pixels)` | `PixelContainer` → 8-bit RGBA PNG bytes |
| `decodePng(bytes, opts?)` | PNG bytes → `PixelContainer`. `opts.maxPixels` lowers the ceiling. |
| `PngCodec` | `ImageCodec` implementation, mime type `image/png`. Takes the same options. |
| `adler32(data)` | RFC 1950 checksum, exported because it is testable alone |
| `PngError` / `PngErrorCode` | Stable portable failure code plus explanatory message |
| `PNG_ERROR_CODES` | Closed IC18 error-code list shared with the neutral corpus |
| `PNG_MAX_DIMENSION` / `PNG_MAX_PIXELS` | Public fixed default resource ceilings |
