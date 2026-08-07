# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `vigenere-cipher` crate, in
  namespace `ca::vigenere`: the Vigenere polyalphabetic cipher plus
  Kasiski/Friedman cryptanalysis.
- Cipher: `encrypt`, `decrypt` (returning `std::optional<std::string>`;
  `std::nullopt` on an invalid key) and `key_valid`. Case is preserved,
  non-alphabetic characters pass through and do not advance the key.
- Cryptanalysis: `find_key_length` (Index of Coincidence), `find_key`
  (chi-squared frequency analysis), and `break_cipher` returning a
  `BreakResult { key, plaintext }`.
- No libm — the statistics use only `+ - * /`;
  `std::numeric_limits<double>::max()` stands in for the crate's
  `f64::INFINITY`.
- Tests use the crate's own vectors, including recovering the key lengths 3/5/6
  and the keys `KEY`/`LEMON`/`SECRET`, under GCC and Clang via `iso-harness`.
