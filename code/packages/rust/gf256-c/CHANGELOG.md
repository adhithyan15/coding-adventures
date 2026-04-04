# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-03

### Added

- `gf256_c_add` — Add two GF(256) elements (bitwise XOR). No error cases.
- `gf256_c_subtract` — Subtract two GF(256) elements (= add in char-2).
- `gf256_c_multiply` — Multiply using log/antilog table lookup (O(1)).
- `gf256_c_divide` — Divide; returns 0xFF sentinel and sets error flag if
  divisor is zero.
- `gf256_c_power` — Raise to non-negative integer power via log scaling.
- `gf256_c_inverse` — Multiplicative inverse; returns 0xFF sentinel and sets
  error flag if input is zero.
- `gf256_c_had_error` — Returns 1 if the most recent call on this thread
  encountered an error. The flag is per-thread via `thread_local!`.
- `gf256_c_primitive_polynomial` — Returns the field's defining polynomial
  (285 = 0x11D).
- `include/gf256_c.h` — C header declaring all exported functions.
- `Cargo.toml` with `crate-type = ["staticlib", "cdylib"]` and LTO enabled.
- `BUILD` file for the coding-adventures build tool.
- Literate programming style with extensive inline documentation explaining
  GF(256) arithmetic, the error flag design, and the primitive polynomial.
