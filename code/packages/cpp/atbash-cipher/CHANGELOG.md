# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `atbash-cipher` crate, in
  namespace `ca::atbash`: the Atbash substitution cipher (A↔Z), preserving case
  and passing non-letters through unchanged.
- API: `atbash_char` (single-character substitution), `encrypt`, and `decrypt`
  (identical to encrypt, since Atbash is self-inverse), all over `std::string`.
- Byte-by-byte operation: only ASCII letters are substituted; every other byte
  passes through, matching the crate.
- Tests use the crate's own vectors — single-character mappings, full alphabet,
  case/punctuation handling, non-alpha passthrough, the self-inverse property,
  and that no letter maps to itself — under GCC and Clang via `iso-harness`.
