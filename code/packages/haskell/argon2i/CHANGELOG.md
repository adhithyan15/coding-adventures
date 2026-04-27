# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-20

### Added

- Initial pure-Haskell Argon2i implementation following RFC 9106.
- `argon2i` / `argon2iHex` high-level API.
- `argon2Version` constant (`0x13`).
- Data-independent `fillSegment` driven by a deterministic address
  stream: `double-G(0, compress(0, (r, lane, slice, m', t_total,
  TYPE_I, counter)))` with a 128-word chunk refreshed lazily.
- Shared G-mixer, permutation P, compression G, H' variable-length
  hash, `index_alpha`, and input validation identical to the sibling
  `argon2d` package.
- Hspec test suite matching the RFC 9106 §5.2 canonical vector and
  covering validation, determinism, input-binding, tag-length variants,
  and a side-channel sanity check.
