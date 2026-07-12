# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `vigenere-cipher` crate: the Vigenere
  polyalphabetic cipher plus Kasiski/Friedman cryptanalysis.
- Cipher: `vigenere_encrypt`, `vigenere_decrypt`, `vigenere_key_valid`. Case is
  preserved, non-alphabetic characters pass through and do not advance the key,
  and an empty or non-alphabetic key is rejected (returns `NULL`).
- Cryptanalysis: `vigenere_find_key_length` (Index of Coincidence),
  `vigenere_find_key` (chi-squared frequency analysis), and `vigenere_break`
  (automatic) with `vigenere_break_free`.
- No libm — the statistics use only `+ - * /`; `DBL_MAX` from `<float.h>` stands
  in for the crate's `f64::INFINITY`. Allocations return `NULL` on failure;
  `find_key` guards `key_length + 1` against overflow and `find_key_length` uses
  `calloc`'s checked multiply.
- Tests use the crate's own vectors, including recovering the key lengths 3/5/6
  and the keys `KEY`/`LEMON`/`SECRET`, under GCC and Clang via `iso-harness`.
