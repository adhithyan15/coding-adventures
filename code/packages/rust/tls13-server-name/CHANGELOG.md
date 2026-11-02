# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Allocation-free RFC 6066 ClientHello `server_name` encoding and strict
  one-`host_name` decoding.
- Bounded ASCII DNS hostname validation that rejects trailing dots, wildcards,
  malformed labels, and IP literals, including legacy or ambiguous numeric IPv4
  resolver spellings.
- TLS 1.3 EncryptedExtensions empty-acknowledgement validation.
- Stable payload-free errors, failure-atomic output, zero dependencies, and no
  external effects.
