# Changelog

## [0.1.0] - 2026-04-25

### Added
- Initial implementation of ZStd (RFC 8878) compression/decompression
- Full FSE (Finite State Entropy) encode/decode with predefined tables
- RevBitWriter/RevBitReader for ZStd's backward bitstream format
- Raw, RLE, and Compressed block types
- 256 MB decompression bomb protection
- 9 test cases covering round-trips, compression ratios, and error handling
