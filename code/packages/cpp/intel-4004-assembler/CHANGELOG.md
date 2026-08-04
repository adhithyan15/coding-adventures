# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `intel-4004-assembler`
  crate, in namespace `ca::intel4004`: a two-pass assembler for the Intel 4004.
- `assemble(text)` returning `std::vector<std::uint8_t>`, throwing
  `ca::intel4004::AssemblerError` on any error.
- Lexer, pass-1 symbol table (`std::unordered_map`) with `ORG`, pass-2 encoding
  with forward-`ORG` zero padding, and the full instruction table.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): the Rust reference
  vector plus hand-computed encodings for every instruction, labels, ORG
  padding, comments, and error cases.
