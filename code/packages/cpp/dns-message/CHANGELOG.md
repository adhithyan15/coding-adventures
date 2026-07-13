# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `dns-message` crate in namespace
  `ca::dns_message`: the transport-agnostic DNS wire-format codec (RFC 1035).
- `DnsName`, header flags, opcode/response-code/record-type/class types,
  questions, resource records, and a tagged `RecordData` (`A`/`AAAA`/`CNAME`/
  `PTR`/`SRV`/`RAW`) — all with value equality.
- `build_query`, `parse_message`, `serialize_message`, plus `Message`
  accessors (`is_success`, `first_answer_of_type`, `ipv4_answers`,
  `ipv6_answers`). Malformed input throws `ca::dns_message::Error` (carrying an
  `ErrorKind` + `detail()`).
- Name decompression with a visited-set and a 128-hop cap; ownership is
  automatic via `std::vector` / `std::string`.
- 57 checks mirroring the crate's unit tests, run under every ISO C++ compiler
  via the shared `iso-harness`; also clean under ASan + UBSan.
