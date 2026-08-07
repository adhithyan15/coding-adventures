# zstd (Haskell)

Pure Haskell implementation of the repository's CMP07 educational Zstandard
codec. It emits valid RFC 8878 frames and composes the native Haskell `lzss`
package for match finding; it does not call a system Zstandard library or use
FFI bindings.

## Supported format

- single-segment frames with an eight-byte content-size field;
- the standard 128 KiB block limit and multi-block frames;
- raw, RLE, and compressed blocks;
- raw literal sections;
- predefined literal-length, match-length, and offset FSE tables;
- strict validation of frame structure, reserved modes, truncated payloads,
  backreferences, trailing data, and decompressed-output limits.

The decoder also consumes the standard dictionary-id/content-size header forms
and optional checksums. Dictionary contents, compressed literal Huffman tables,
and custom/repeat FSE *table* modes are deliberately outside this educational
subset.

The decoder fully supports Repeated-Offset (R1/R2/R3) *sequences* (RFC 8878
§3.1.1.3.2.1.1) -- an Offset_Value of 1, 2, or 3 references one of three
recent offsets (frame-scoped, threaded through every block) rather than
encoding a literal distance, including the "Literals_Length == 0 shifts the
repeat-offset interpretation by one" special case. The ENCODER intentionally
never emits repeat-offset shortcuts -- every offset it writes is explicit --
but real `zstd`'s encoder uses them constantly, so the decoder must
understand them to interoperate with real-world `.zst` data. See
`lessons.md` Lesson 100.

The sequences-section FSE codec (table construction, per-sequence field order,
and the last-sequence state-initialisation special case) is cross-checked
against the real `zstd` CLI by `ZstdCliInteropSpec` (skipped, not failed, when
`zstd` isn't on `PATH`) -- a same-codebase round-trip test can never catch a
systematic, symmetric protocol deviation where the encoder and decoder simply
agree with each other on the wrong convention. See `lessons.md` Lesson 95 for
the three compounding bugs of exactly that shape this test line exists to
catch.

## API

- `compress :: ByteString -> Either String ByteString`
- `decompress :: ByteString -> Either String ByteString`
- `magic`, `maxBlockSize`, and `maxOutputSize` expose the wire and safety
  constants used by the implementation.

## Development

```sh
cabal test all
cabal test all --enable-coverage
```
