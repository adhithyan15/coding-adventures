# Changelog — coding_adventures_zstd

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-24

### Added

- **`CodingAdventures.Zstd.compress/1`** — Compress any binary to a valid ZStd
  frame (RFC 8878). Supports Raw, RLE, and Compressed block types. Falls back
  to Raw when LZ77 + FSE does not reduce size.

- **`CodingAdventures.Zstd.decompress/1`** — Decompress a ZStd frame, returning
  `{:ok, binary}` or `{:error, reason}`. Supports frames produced by our encoder
  as well as handcrafted Raw-block frames. Guards against decompression bombs
  (256 MB output cap).

- **FSE (Finite State Entropy) codec** — Full encoder and decoder using the
  predefined distributions from RFC 8878 Appendix B. No per-frame table
  transmission needed. Both encoder and decoder share the same spread function
  (step = (sz >> 1) + (sz >> 3) + 3) for exact symmetry.

- **RevBitWriter / RevBitReader** — Reverse bit-stream codec for the FSE sequence
  bitstream. The writer prepends bytes (then reverses at flush) and attaches a
  sentinel bit so the reader can locate the start without side-channel data.

- **Multi-block support** — Inputs larger than 128 KB are split into multiple
  blocks. Back-references correctly span block boundaries (the output accumulator
  is threaded through all block decompressors).

- **Sequence count encoding fix** — Follows RFC 8878 §3.1.1.1.3: 2-byte counts
  use `byte0 = (count >> 8) | 0x80` (high byte first, bit-7 set) rather than
  a raw little-endian u16. This ensures the decoder can distinguish 1-byte and
  2-byte encodings by inspecting byte0 alone.

- **24 unit + integration tests** — TC-1 through TC-9 from the spec, plus
  additional round-trip, compression-ratio, wire-format, and edge-case tests.
  Coverage: 90% (threshold: 80%).

- **Literate inline documentation** — All internals explained with diagrams,
  bit-layout examples, and algorithm justifications for each phase of the FSE
  table construction.

### Dependencies

- `coding_adventures_lzss` (local path `../lzss`) — provides LZ77 tokenisation.
