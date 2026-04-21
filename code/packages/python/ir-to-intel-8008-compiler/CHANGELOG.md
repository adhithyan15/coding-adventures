# Changelog — ir-to-intel-8008-compiler

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-20

### Added

- `CodeGenerator` — one-pass IR-to-8008 translator for all IR opcodes:
  - `LABEL` → column-0 label definition
  - `LOAD_IMM` → `MVI Rdst, imm`
  - `LOAD_ADDR` → `MVI H, hi(sym); MVI L, lo(sym)` (sets H:L for memory ops)
  - `LOAD_BYTE` → `MOV A, M; MOV Rdst, A` (reads from H:L)
  - `STORE_BYTE` → `MOV A, Rsrc; MOV M, A` (writes to H:L)
  - `ADD` → `MOV A, Ra; ADD Rb; MOV Rdst, A`
  - `ADD_IMM` → `MOV A, Ra; ADI imm; MOV Rdst, A`; imm=0 optimised to pure
    register copy (2 instructions, no ADI)
  - `SUB` → `MOV A, Ra; SUB Rb; MOV Rdst, A`
  - `AND` → `MOV A, Ra; ANA Rb; MOV Rdst, A`
  - `OR` → `MOV A, Ra; ORA Rb; MOV Rdst, A`
  - `XOR` → `MOV A, Ra; XRA Rb; MOV Rdst, A`
  - `NOT` → `MOV A, Ra; XRI 0xFF; MOV Rdst, A` (8008 has no NOT; XOR-0xFF
    is the canonical complement)
  - `CMP_EQ` → 6-instruction optimistic-load/JTZ sequence with unique label
  - `CMP_NE` → 6-instruction optimistic-load/JTZ sequence (inverted)
  - `CMP_LT` → 6-instruction sequence using JTC (carry = unsigned borrow)
  - `CMP_GT` → operand-swap trick: load Rb→A, CMP Ra; then JTC sequence
  - `BRANCH_Z` → `MOV A, Rcond; CPI 0; JTZ label`
  - `BRANCH_NZ` → `MOV A, Rcond; CPI 0; JFZ label`
  - `JUMP` → `JMP label`
  - `CALL` → `CAL label`
  - `RET` → `MOV A, C; RFC` (copies return value from C to A, then returns)
  - `HALT` → `HLT`
  - `NOP` → comment (8008 has no dedicated NOP)
  - `COMMENT` → `; text`
  - `SYSCALL` → inline 8008 hardware intrinsics:
    - 3 → `ADC` (add with carry, D+E+CY → C)
    - 4 → `SBB` (subtract with borrow, D−E−CY → C)
    - 11 → `RLC` (rotate A left circular)
    - 12 → `RRC` (rotate A right circular)
    - 13 → `RAL` (rotate left through carry)
    - 14 → `RAR` (rotate right through carry)
    - 15 → `MVI A, 0; ACI 0` (materialise carry flag into C)
    - 16 → `ORA A; JFP` parity materialisation into C
    - 20–27 → `IN p; MOV C, A` (read input port p)
    - 40–63 → `MOV A, D; OUT p` (write output port p)
  - Unknown opcodes → safe comment fallback (no crash)

- `IrToIntel8008Compiler` (`Intel8008Backend` alias) — validate-then-generate
  facade: runs `IrValidator` first; on error raises `IrValidationError` with
  all violations concatenated; on success returns assembly text.

- Module-level convenience functions:
  - `validate(program)` → `list[IrValidationError]`
  - `generate_asm(program)` → `str` (skips validation)

- Physical register table (`_VREG_TO_PREG`): v0→B, v1→C, v2→D, v3→E, v4→H,
  v5→L (matches `intel-8008-ir-validator` 6-register limit).

- Unique label counter (`_label_count` on `CodeGenerator`) for comparison
  materialisation labels (`cmp_0`, `cmp_1`, …) — prevents collisions in
  programs with multiple comparisons or parity calls.

- `tests/test_ir_to_intel_8008_compiler.py` — 130+ tests covering every
  opcode, all 10 SYSCALL intrinsics, edge cases, and the compile facade.
  Coverage > 95%.
