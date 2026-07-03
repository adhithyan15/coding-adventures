## [0.1.0] - 2026-05-25

### Added

- Initial release: baseline JPEG (JFIF SOF0) encoder and decoder
- YCbCr colour space conversion (BT.601 / JFIF coefficients) in `color.rs`
- 8×8 block DCT using the `dsp-dct` crate (2-D separable DCT-II / IDCT-III)
- Standard Annex K quantization tables (luma + chroma) with quality factor 1–100
  using the libjpeg-turbo quality scaling formula
- Standard Annex K Huffman tables for DC/AC luma/chroma (all 4 tables)
- MSB-first bit I/O with 0xFF byte stuffing / un-stuffing (`BitWriter`, `BitReader`)
- JFIF container assembly: SOI, APP0, DQT×2, SOF0, DHT×4, SOS, scan data, EOI
- JFIF container parser: handles SOF0, DQT, DHT, SOS segments; skips APP/other
- `JpegCodec` struct implementing the `pixel-container::ImageCodec` trait
- `encode_jpeg` / `decode_jpeg` convenience functions (quality 75 default)
- 4:4:4 chroma sampling (no downsampling; one Y/Cb/Cr block per 8×8 MCU)
- Edge replication for images with dimensions not multiples of 8
- DC differential coding (inter-block prediction per component)
- AC run-length + Huffman coding with EOB and ZRL symbols
- 56 unit tests covering all modules; all tests pass
