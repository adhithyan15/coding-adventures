# Changelog

All notable changes to the C++ `assembler` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `assembler` crate
  (namespace `ca::assembler`) — an ARM assembly parser and 32-bit binary
  encoder.
- `Assembler::parse` (source → `std::vector<ArmInstruction>`, recording labels)
  and `Assembler::encode` (→ `std::vector<std::uint32_t>`) for MOV(S), ADD(S),
  SUB(S), AND(S), ORR(S), EOR(S), RSB(S), CMP, LDR, STR, NOP, and labels.
- `ArmInstruction` is a `std::variant<DataProcessing, Load, Store, Nop, Label>`;
  `Option<u32>` → `std::optional`; errors throw `AssemblerError` (a
  `std::runtime_error`) whose `what()` reproduces the Rust `Display` text.
- 43 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
