# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Added a transport-agnostic DNS message model for headers, flags, questions,
  resource records, record classes, record types, and record data.
- Added query construction and wire-format serialization for standard DNS
  messages.
- Added DNS response parsing with compressed-name pointer support and loop /
  bounds validation.
- Added parser hardening for attacker-controlled section counts and excessive
  compression-pointer chains.
- Added typed decoding for `A`, `AAAA`, and `CNAME` records.
- Added raw preservation for unknown record types so future DNS extensions can
  be parsed without losing bytes.
- Added unit coverage for query round trips, compressed names, CNAMEs,
  `NXDOMAIN`, truncation flags, unknown records, and malformed input.
