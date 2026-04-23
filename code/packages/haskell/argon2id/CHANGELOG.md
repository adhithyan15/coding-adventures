# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-20

### Added

- Initial pure-Haskell Argon2id implementation following RFC 9106.
- `argon2id` / `argon2idHex` high-level API.
- `argon2Version` constant (`0x13`).
- Hybrid `fillSegment` that picks data-INDEPENDENT addressing during
  `r == 0 && sl < 2` and data-DEPENDENT addressing everywhere else,
  matching the Argon2id specification.
- Shared G-mixer, permutation P, compression G, H' variable-length
  hash, `index_alpha`, and input validation identical to the sibling
  `argon2d` / `argon2i` packages.
- Hspec test suite matching the RFC 9106 §5.3 canonical vector plus
  validation, determinism, input-binding, tag-length variants, and a
  cross-variant distinctness check.
