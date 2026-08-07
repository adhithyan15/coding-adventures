# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `des` crate, in namespace
  `ca::des`: the DES block cipher (FIPS 46) and Triple DES (NIST SP 800-67).
- `encrypt_block` / `decrypt_block` (→ `std::array<std::uint8_t, 8>`),
  `ecb_encrypt` / `ecb_decrypt` (ECB with PKCS#7 padding; `ecb_decrypt` returns
  `std::optional<std::vector<std::uint8_t>>`), `tdea_encrypt_block` /
  `tdea_decrypt_block` (Triple DES EDE).
- Tables as `inline constexpr`; the same bit-array algorithm as the C sibling.
- Tests pinned to the FIPS 46 worked example and NIST SP 800-20 known-answer
  vectors, plus round-trips, ECB, and Triple DES, under GCC and Clang via
  `iso-harness`.
