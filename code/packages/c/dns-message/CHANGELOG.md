# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `dns-message` crate: the transport-agnostic DNS
  wire-format codec (RFC 1035). No sockets, retries, caching, or nameserver
  selection.
- `DnsName` (ASCII labels) with `dns_name_from_ascii` / `_free` / `_clone` /
  `_is_root` / `_to_string` / `_equal`.
- Header flags, opcode/response-code/record-type/class tagged enums, questions,
  resource records, and a `DnsRecordData` tagged union (`A`, `AAAA`, `CNAME`,
  `PTR`, `SRV`, and verbatim `RAW`).
- `dns_build_query`, `dns_parse_message`, `dns_serialize_message`, plus
  `dns_message_is_success` / `_first_answer_of_type` / `_ipv4_answers` /
  `_ipv6_answers`. Errors surface as `DnsError { kind, message, detail }`.
- Name decompression with a visited-flag array and a 128-hop cap (rejects
  pointer loops, out-of-bounds pointers, over-long names/labels, non-ASCII
  labels, reserved prefixes) — safe against malformed input.
- 115 checks mirroring the crate's unit tests (the canonical info.cern.ch query
  and compressed A/CNAME/PTR/SRV responses, AAAA and unknown-record round-trips,
  and every decode/encode error path), run under every ISO C compiler via the
  shared `iso-harness`; also clean under ASan + UBSan.
