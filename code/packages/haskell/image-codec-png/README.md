# CodingAdventures.ImageCodecPng

Portable, bounded, zero-external-dependency RFC 2083 PNG encoder and decoder in Haskell (IC18).

## Overview

`CodingAdventures.ImageCodecPng` implements an RFC 2083 compliant PNG codec for 8-bit non-interlaced truecolour and greyscale images. It is engineered for pure-computation memory safety, bounded input processing, and deterministic RGBA8 representation.

### Architectural Principles

1. **Zero Duplicated Compression Primitives**: Reuses the repository `Zip` package for CRC-32 checksums (`Zip.crc32`), raw RFC 1951 DEFLATE compression (`Zip.rawDeflate`), and bounded decompression (`Zip.rawInflateCounted`). Zero external Hackage `zlib` dependency in library code.
2. **Buffer Interchange**: Integrates seamlessly with `PixelContainer` (IC00) and implements the `ImageCodec` type class interface.
3. **Payload-Blind Error Handling (Standing Rule #20)**: 100% of error messages, diagnostic strings, and placeholders are isolated in `Messages.hs`. The closed sum type `PngError` contains 29 constructors without payload strings; exception and `Show` messages never construct or interpolate untrusted caller or file data.
4. **Memory-Bounded Streaming**: Multi-part `IDAT` chunks are accumulated and decompressed against an exact expected byte count, strictly capping memory allocation.

## CVE Resilience Showcase

| Vulnerability Class | Relevant CVE | Mitigation in `CodingAdventures.ImageCodecPng` |
|:---|:---|:---|
| **Dimension & Integer Overflows** | CVE-2004-0597, CVE-2019-7317 | Strict edge cap at 16,384 (`dimension-limit`) and product limit at 33,554,432 pixels (`pixel-limit`). Caller pixel ceiling validated eagerly before file parsing (`invalid-max-pixels`). PixelContainer dimensions validated on encode using 64-bit integer arithmetic. |
| **Decompression & Zip Bombs** | CVE-2002-1363, CVE-2023-4863 | Decompression strictly bounded by promised scanline length `height * (stride + 1)` (`inflated-length-mismatch`). |
| **Chunk-Length Truncation & Pre-Allocation OOM** | CVE-2018-14550 | Validates 32-bit chunk length against available remaining file bytes before allocating or slicing payloads (`truncated-chunk`). |
| **Filter Signedness & Modulo Arithmetic Traps** | CVE-2015-8126 | Enforces unsigned 8-bit modulo arithmetic (`.&. 0xFF`) and normative Paeth predictor tie-breaking (preferring $a$, then $b$, then $c$). Validates filter byte $\le 4$ (`invalid-filter`). |
| **Checksum & Integrity Tampering, IDAT Cavities** | CVE-2010-1205 | Recomputes CRC-32 on all chunks including IEND (`chunk-crc-mismatch`). Recomputes Adler-32 over uncompressed scanlines (`adler-mismatch`). Verifies DEFLATE stream consumes exact bytes before trailer (`idat-cavity`). |
| **Chunk Ordering, Duplication & APNG Spoofing** | CVE-2011-3045, CVE-2021-30860 | Strict state machine: IHDR must be first (`chunk-before-ihdr`), unique (`duplicate-ihdr`), followed by optional PLTE/tRNS before IDAT, consecutive IDAT chunks (`nonconsecutive-idat`), and clean empty IEND (`invalid-iend`, `trailing-data`). Rejects APNG chunks (`acTL`, `fcTL`, `fdAT`) with `unsupported-feature`. |
| **Payload Data Leakage in Exceptions** | Standing Rule #20 | Diagnostic messages are statically mapped from closed error constructors in `Messages.hs`. Input fragments or file contents are never interpolated into error strings. |

## Usage

### Functional API

```haskell
import qualified Data.ByteString as BS
import PixelContainer (PixelContainer(..))
import CodingAdventures.ImageCodecPng
    ( encodePng
    , decodePng
    , decodePngBounded
    , adler32
    )

-- Encode a PixelContainer to PNG bytes
let container = PixelContainer 2 2 (BS.replicate (2 * 2 * 4) 255)
case encodePng container of
    Left err      -> putStrLn ("Encode error: " ++ show err)
    Right pngData -> BS.writeFile "output.png" pngData

-- Decode PNG bytes
pngBytes <- BS.readFile "input.png"
case decodePng pngBytes of
    Left err -> putStrLn ("Decode error: " ++ show err)
    Right pc -> putStrLn ("Decoded: " ++ show (pcWidth pc) ++ "x" ++ show (pcHeight pc))

-- Bounded decode with custom pixel ceiling
case decodePngBounded 100000 pngBytes of
    Left err -> putStrLn ("Decode error: " ++ show err)
    Right pc -> putStrLn "Decoded within limit"
```

### Type Class Codec Adapter

```haskell
import PixelContainer (ImageCodec(..))
import CodingAdventures.ImageCodecPng (PngCodec(..), defaultPngCodec)

let codec = defaultPngCodec
putStrLn (mimeType codec) -- "image/png"

let encoded = encode codec container
let decoded = decode codec encoded
```

## Error Taxonomy

The package defines a closed sum type of 29 portable error identifiers:

- `InvalidMaxPixels` (`invalid-max-pixels`)
- `InvalidImageDimensions` (`invalid-image-dimensions`)
- `InvalidPixelDataLength` (`invalid-pixel-data-length`)
- `FileTooShort` (`file-too-short`)
- `InvalidSignature` (`invalid-signature`)
- `TruncatedChunk` (`truncated-chunk`)
- `InvalidChunkType` (`invalid-chunk-type`)
- `ChunkCrcMismatch` (`chunk-crc-mismatch`)
- `ChunkBeforeIhdr` (`chunk-before-ihdr`)
- `DuplicateIhdr` (`duplicate-ihdr`)
- `InvalidIhdrLength` (`invalid-ihdr-length`)
- `InvalidDimensions` (`invalid-dimensions`)
- `DimensionLimit` (`dimension-limit`)
- `PixelLimit` (`pixel-limit`)
- `UnsupportedFeature` (`unsupported-feature`)
- `InvalidPlte` (`invalid-plte`)
- `InvalidTrns` (`invalid-trns`)
- `NonconsecutiveIdat` (`nonconsecutive-idat`)
- `InvalidIend` (`invalid-iend`)
- `TrailingData` (`trailing-data`)
- `UnknownCriticalChunk` (`unknown-critical-chunk`)
- `MissingRequiredChunk` (`missing-required-chunk`)
- `InvalidZlibHeader` (`invalid-zlib-header`)
- `PresetDictionary` (`preset-dictionary`)
- `InflateFailed` (`inflate-failed`)
- `InflatedLengthMismatch` (`inflated-length-mismatch`)
- `IdatCavity` (`idat-cavity`)
- `AdlerMismatch` (`adler-mismatch`)
- `InvalidFilter` (`invalid-filter`)

## License

MIT
