# Changelog — image-codec-orf

All notable changes to this crate are documented here.

---

## [0.1.0] — 2026-05-30

### Added

- Initial implementation of the Olympus ORF image codec (IC15).
- `decode_orf(bytes: &[u8]) -> Result<PixelContainer, String>` — decodes ORF
  byte streams (uncompressed, Compression=1) into RGBA8 pixel containers.
- `encode_orf(pixels: &PixelContainer) -> Vec<u8>` — encodes a PixelContainer
  as an uncompressed TIFF-based ORF stream, suitable for round-trip testing.
- `OrfCodec` struct implementing `paint_instructions::ImageCodec` with MIME
  type `"image/x-olympus-orf"`.
- IIRO magic normalisation: files with non-standard Olympus magic bytes
  (`0x52 0x4F` at positions 2–3) are patched to standard TIFF magic before
  parsing.
- Make tag validation: rejects files whose tag 271 is present but does not
  contain "OLYMPUS" or "OM DIGITAL" (prevents silent misidentification of
  Canon/Nikon/Sony files).
- CFA IFD selection: scans the IFD chain for the first IFD with
  `PhotometricInterpretation = 32803` (CFA/Bayer).
- Colour constants exported: `OLYMPUS_COLOR_MATRIX`, `OLYMPUS_BLACK_LEVEL`,
  `OLYMPUS_WHITE_LEVEL`.
- 12 unit tests covering: version, MIME type, round-trip (2×2, 4×4, trait),
  error cases (empty, short, big-endian), Make tag rejection predicate, IIRO
  normalisation, colour matrix shape, black/white level values.

### Not yet implemented

- Olympus proprietary RLE (Compression=32767) — returns a clear error message.
- MakerNote WB extraction (tags 0x1017/0x1018) — uses neutral [1.0, 1.0, 1.0]
  for now.
- Per-model colour matrix selection — hardcoded to E-M1 Mark II values.
