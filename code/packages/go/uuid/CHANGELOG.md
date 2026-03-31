# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-03-31

### Changed

- **Operations system integration**: All public functions and methods (`Parse`,
  `IsValid`, `V4`, `V5`, `V3`, `V1`, `V7`, `Compare`, `String`, `Bytes`,
  `ToInt`, `Version`, `Variant`, `IsNil`, `IsMax`) are now wrapped with
  `StartNew[T]` from the package's Operations infrastructure. Multi-return
  functions (`ToInt`) use inline helper structs. Each call gains automatic
  timing, structured logging, and panic recovery.

## [0.1.0] - 2026-03-23

### Added

- Full UUID v1/v3/v4/v5/v7 implementation from scratch (RFC 4122 + RFC 9562)
- `UUID` type ([16]byte) with `String()`, `Bytes()`, `ToInt()`, `Version()`,
  `Variant()`, `IsNil()`, `IsMax()`, `Compare()` methods
- `Parse()`: accepts standard (8-4-4-4-12), compact, braced, URN formats
- `IsValid()`: non-panicking string validation
- Namespace constants: `NamespaceDNS`, `NamespaceURL`, `NamespaceOID`,
  `NamespaceX500` (RFC 4122 Appendix C)
- `Nil` and `Max` sentinel values
- `V4()`: 122 bits of `crypto/rand`
- `V5(namespace, name)`: SHA-1 name-based via ca_sha1.Sum1();
  RFC vector: V5(NamespaceDNS, "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"
- `V3(namespace, name)`: MD5 name-based via ca_md5.SumMD5();
  RFC vector: V3(NamespaceDNS, "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"
- `V1()`: 60-bit 100-ns Gregorian timestamp + 14-bit random clock sequence +
  48-bit random node (multicast bit set to signal random generation)
- `V7()`: 48-bit Unix millisecond timestamp (sortable) + 74 random bits
- Package declaration `package ca_uuid` for consistent ca_ naming
- 47 tests, 91% coverage
- Knuth-style literate programming comments throughout
