# Changelog

All notable changes to the C `intel-8008-packager` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `intel-8008-packager` crate — an Intel
  HEX ROM image encoder/decoder for the Intel 8008.
- `pak_encode_hex` (binary → Intel HEX string, 16-byte data records + EOF, with
  correct per-record checksums) and `pak_decode_hex` (Intel HEX → `PakDecoded`
  origin + payload), plus `pak_decoded_free` and `pak_error_message`.
- `PakStatus` + out-parameter API in place of the Rust `Result` /
  `PackagerError(String)`; each error code maps to a representative static
  message containing the same keyword as the Rust text.
- Strict, hardened decoder: rejects missing `:`, non-hex/odd-length bodies,
  under-length records, checksum mismatches, unsupported record types,
  overlapping/duplicate data records, a missing EOF record, over-long lines,
  and any span exceeding the 8008's 16 KB address space — with overflow-guarded
  growable buffers and address arithmetic throughout.
- 67 checks mirroring the Rust crate's own unit tests (exact encode vectors,
  the checksum property, all encode/decode error paths, and round-trips
  including the full 16 KB image and the top of the address space), run under
  every available C compiler via the shared `iso-harness`; the suite also
  passes clean under AddressSanitizer + UndefinedBehaviorSanitizer.
