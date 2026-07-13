# Changelog

All notable changes to the C++ `intel-8008-packager` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `intel-8008-packager`
  crate (namespace `ca::intel_8008_packager`) — an Intel HEX ROM image
  encoder/decoder for the Intel 8008.
- `encode_hex(binary, origin)` returning `std::string` (16-byte data records +
  EOF, correct checksums) and `decode_hex(text)` returning
  `DecodedHex { origin, binary }`; both throw `PackagerError` (a
  `std::runtime_error`) in place of the Rust `Result::Err`.
- Strict, hardened decoder over a `std::map<address, bytes>` (mirroring the Rust
  `BTreeMap`): rejects missing `:`, non-hex/odd-length bodies, under-length
  records, checksum mismatches, unsupported record types, overlapping/duplicate
  records, a missing EOF record, over-long lines, and any span exceeding the
  8008's 16 KB address space.
- 39 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
