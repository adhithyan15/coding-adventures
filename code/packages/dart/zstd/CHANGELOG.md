# Changelog

## 0.1.0

- Added the initial Dart implementation of the CMP07 ZStd compression package.
- Added `compress` function that encodes data into a valid RFC 8878 ZStd frame.
- Added `decompress` function that decodes any RFC 8878 ZStd frame with Raw, RLE, or Compressed blocks.
- Implemented predefined FSE (Finite State Entropy) tables for Literal Length, Match Length, and Offset coding.
- Implemented the reversed bitstream encoder (`RevBitWriter`) and decoder (`RevBitReader`) used by ZStd's sequence section.
- Implemented raw literals section encoding and decoding with 1/2/3-byte header variants.
- Implemented ZStd sequence count encoding and decoding with 1/2/3-byte forms.
- Integrated the `coding_adventures_lzss` package for LZ77 back-reference generation with a 32 KB window.
- Added decompression bomb guard: output capped at 256 MB.
- Added support for multi-block frames (inputs > 128 KB split across blocks).
- Added RLE block detection: runs of a single byte value emit a 4-byte block instead of full compression overhead.
