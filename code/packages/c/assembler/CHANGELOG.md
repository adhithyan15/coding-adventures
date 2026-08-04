# Changelog

All notable changes to the C `assembler` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `assembler` crate — an ARM assembly
  parser and 32-bit binary encoder.
- `asm_parse` (source → `ArmInstruction` array, recording labels) and
  `asm_encode` (instructions → `uint32_t` machine-code words) for MOV(S),
  ADD(S), SUB(S), AND(S), ORR(S), EOR(S), RSB(S), CMP, LDR, STR, NOP, and
  labels; plus `asm_init`/`asm_free`, `asm_label_lookup`, and
  `asm_instructions_free`.
- `AsmStatus` + optional `AsmError` out-parameter in place of the Rust
  `Result`/`AssemblerError`; each error message reproduces the Rust `Display`
  text (e.g. `"Unknown mnemonic: BLAH"`, `"ADD: expected 3 operands, got 2"`).
- Only `ASM_INSTR_LABEL` owns a heap string (freed by `asm_instructions_free`);
  growable arrays and the label table are overflow-guarded; the parser
  tokenises a per-line scratch copy so the source is never mutated.
- 66 checks mirroring the Rust crate's own unit tests (register/immediate
  parsing, every instruction form, exact binary encodings, label handling, and
  error paths), run under every available C compiler via the shared
  `iso-harness`; the suite also passes clean under AddressSanitizer +
  UndefinedBehaviorSanitizer.
