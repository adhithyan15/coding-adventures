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
and custom/repeat FSE tables are deliberately outside this educational subset.

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
