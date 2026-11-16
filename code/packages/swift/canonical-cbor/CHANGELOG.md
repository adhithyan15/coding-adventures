# Changelog

All notable changes to `canonical-cbor` will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Pure Swift implementation of RFC 8949 canonical CBOR (CBR01 profile).
- Zero `Foundation` runtime dependency in library code.
- Checked canonical encoder (`encodeChecked`, `encodeIntoChecked`) with strict depth (128) and size (1,048,576 bytes) limits.
- Strict canonical decoder (`decode`) enforcing shortest-form integer headers, canonical map sorting, duplicate key rejection, and scalar-valid UTF-8.
- Map span extraction (`decodeMapSpans`) returning spanned map entries with byte ranges.
- Comprehensive test suite covering the 55-case shared conformance corpus, CVE resilience, deterministic property tests, and adversarial mutation fuzzing.
