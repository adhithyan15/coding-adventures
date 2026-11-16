# CodingAdventures::ImageCodecPNG

Portable, bounded, zero-external-dependency RFC 2083 PNG encoder and decoder in Perl (IC18).

## Overview

`CodingAdventures::ImageCodecPNG` implements an RFC 2083 compliant PNG codec for 8-bit non-interlaced truecolour and greyscale images. It is engineered for pure-computation memory safety, bounded input processing, and deterministic RGBA8 representation.

### Architectural Principles

1. **Zero Duplicated Compression Primitives**: Reuses `CodingAdventures::Zip` (`v0.2.0`) for CRC-32 checksums (`CodingAdventures::Zip::crc32`), DEFLATE compression (`CodingAdventures::Zip::raw_deflate`), and bounded decompression (`CodingAdventures::Zip::raw_inflate_counted`). Zero standard library `Compress::Raw::Zlib` or `IO::Compress` in `lib/`.
2. **Buffer Interchange**: Integrates seamlessly with `CodingAdventures::PixelContainer` (IC00) and implements the `CodingAdventures::ImageCodec` OO interface.
3. **Payload-Blind Error Handling (Standing Rule #20)**: 100% of error messages and diagnostic strings are isolated in `Messages.pm`. `PngError->new($code)` accepts an error identifier only; messages are never constructed with untrusted caller or payload data.
4. **Single-Copy Stream Accumulation**: Multiparts `IDAT` chunks are accumulated directly into a binary scalar buffer, halving peak pre-decompression memory usage.

## CVE Resilience Showcase

| Vulnerability Class | Relevant CVE | Mitigation in `CodingAdventures::ImageCodecPNG` |
|:---|:---|:---|
| **Dimension & Integer Overflows** | CVE-2004-0597, CVE-2019-7317 | Strict edge cap at 16,384 (`dimension-limit`) and product limit at 33,554,432 pixels (`pixel-limit`). Caller pixel ceiling validated eagerly before file parsing (`invalid-max-pixels`). |
| **Decompression & Zip Bombs** | CVE-2002-1363, CVE-2023-4863 | Decompression strictly bounded by promised scanline length `height * (stride + 1)` (`inflated-length-mismatch`). |
| **Chunk-Length Truncation & Pre-Allocation OOM** | CVE-2018-14550 | Validates chunk length against available remaining file bytes before allocating or slicing payloads (`truncated-chunk`). |
| **Filter Signedness & Modulo Arithmetic Traps** | CVE-2015-8126 | Enforces unsigned 8-bit modulo arithmetic (`& 0xFF`) and normative Paeth predictor tie-breaking (preferring $a$, then $b$, then $c$). Validates filter byte $\le 4$ (`invalid-filter`). |
| **Checksum & Integrity Tampering, IDAT Cavities** | CVE-2010-1205 | Recomputes CRC-32 on all chunks including IEND (`chunk-crc-mismatch`). Recomputes Adler-32 over uncompressed scanlines (`adler-mismatch`). Verifies DEFLATE stream consumes exact bytes before trailer (`idat-cavity`). |
| **Chunk Ordering, Duplication & APNG Spoofing** | CVE-2011-3045, CVE-2021-30860 | Strict state machine: IHDR must be first (`chunk-before-ihdr`), unique (`duplicate-ihdr`), followed by optional PLTE/tRNS before IDAT, consecutive IDAT chunks (`nonconsecutive-idat`), and clean empty IEND (`invalid-iend`, `trailing-data`). Rejects APNG chunks (`acTL`, `fcTL`, `fdAT`) with `unsupported-feature`. |
| **Payload Data Leakage in Exceptions** | Standing Rule #20 | Exception messages are statically mapped from closed error codes in `Messages.pm`. Input fragments or file contents are never interpolated into error strings. |

## Usage

### Functional API

```perl
use CodingAdventures::ImageCodecPNG qw(encode_png decode_png adler32);
use CodingAdventures::PixelContainer;

# Encode
my $container = CodingAdventures::PixelContainer->new(2, 2);
$container->set_pixel(0, 0, 255, 0, 0, 255); # Red
my $png_bytes = encode_png($container);

# Decode
my $decoded = decode_png($png_bytes);
my ($r, $g, $b, $a) = $decoded->pixel_at(0, 0);

# Bounded decode
my $safe = decode_png($png_bytes, max_pixels => 100_000);
```

### Object-Oriented Codec Adapter

```perl
use CodingAdventures::ImageCodecPNG::Codec;

my $codec = CodingAdventures::ImageCodecPNG::Codec->new(max_pixels => 500_000);
print $codec->mime_type; # image/png

my $bytes = $codec->encode($container);
my $image = $codec->decode($bytes);
```

## Error Taxonomy

The module defines a closed set of 29 portable error identifiers:

- `invalid-max-pixels`
- `invalid-image-dimensions`
- `invalid-pixel-data-length`
- `file-too-short`
- `invalid-signature`
- `truncated-chunk`
- `invalid-chunk-type`
- `chunk-crc-mismatch`
- `chunk-before-ihdr`
- `duplicate-ihdr`
- `invalid-ihdr-length`
- `invalid-dimensions`
- `dimension-limit`
- `pixel-limit`
- `unsupported-feature`
- `invalid-plte`
- `invalid-trns`
- `nonconsecutive-idat`
- `invalid-iend`
- `trailing-data`
- `unknown-critical-chunk`
- `missing-required-chunk`
- `invalid-zlib-header`
- `preset-dictionary`
- `inflate-failed`
- `inflated-length-mismatch`
- `idat-cavity`
- `adler-mismatch`
- `invalid-filter`

## License

MIT
