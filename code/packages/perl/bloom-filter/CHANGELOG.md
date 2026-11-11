# Changelog

## [Unreleased]

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## 0.1.0

- Initial Perl Bloom filter with automatic and explicit sizing.
- Add deterministic double hashing, fill and false-positive statistics,
  over-capacity detection, and stable composite-value encoding.
