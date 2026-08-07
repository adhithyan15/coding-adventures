# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `arm1-simulator` crate in
  namespace `ca::arm1_simulator`: a complete behavioral simulator for the ARM1
  (1985), the first ARM chip.
- The full ARMv1 instruction set — 16 data-processing ops through the inline
  barrel shifter, load/store, block transfer (LDM/STM), branch (B/BL), SWI,
  conditional execution, and 4 banked processor modes with the shared PC+status
  R15.
- `ARM1` class (`reset`, register/flag/mode/memory accessors, `step`, `run`
  returning `std::vector<Trace>`), and free functions `evaluate_condition`,
  `barrel_shift`, `decode_immediate`, `alu_execute`, `decode`,
  `DecodedInstruction::disassemble`, plus the `encode_*` helpers. RAII throughout
  (`std::vector` memory, `std::string` mnemonics, `std::vector<MemoryAccess>`
  traces). Verified clean under ASan + UBSan.
- 77 checks mirroring the crate's unit tests run under every ISO C++ compiler via
  the shared `iso-harness`.
