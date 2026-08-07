# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `scytale-cipher` crate, in
  namespace `ca::scytale`: the Scytale transposition cipher plus brute force.
- API: `encrypt`, `decrypt` (returning `std::optional<std::string>`; `""` for
  empty text, `std::nullopt` on an invalid key) and `brute_force` returning a
  `std::vector<BruteForceResult>`.
- Transposes whole UTF-8 characters (not bytes), matching the crate's
  `char`-based behaviour; multibyte characters stay intact through a round trip.
- Tests use the crate's own vectors — encrypt/decrypt cases, key validation,
  padding stripping, round trips over all valid keys, brute force, and a
  multibyte UTF-8 round trip — under GCC and Clang via `iso-harness`.
