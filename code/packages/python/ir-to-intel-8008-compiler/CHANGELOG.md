# Changelog — ir-to-intel-8008-compiler

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — 2026-04-27

### Added — LANG20: `Intel8008CodeGenerator` — `CodeGenerator[IrProgram, str]` adapter

**New module: `ir_to_intel_8008_compiler.generator`**

- `Intel8008CodeGenerator` — thin adapter satisfying the
  `CodeGenerator[IrProgram, str]` structural protocol (LANG20).

  ```
  [Optimizer] → [Intel8008CodeGenerator] → str (text assembly)
                                             ├─→ assembler → bytes  (AOT)
                                             └─→ Intel 8008 simulator
  ```

  - `name = "intel8008"` — unique backend identifier.
  - `validate(ir) -> list[str]` — delegates to `IrValidator().validate()`,
    converting each `IrValidationError` to its `.message` string.  Never
    raises.  8008 rules: register count ≤ 6 (B, C, D, E, H, L), supported
    opcode set (no LOAD_WORD, STORE_WORD, etc.).
  - `generate(ir) -> str` — delegates to `IrToIntel8008Compiler().compile(ir)`.
    Returns Intel 8008 text assembly (`ORG …`, `MVI …`, `HLT`).  Raises
    `IrValidationError` on invalid IR.

- `Intel8008CodeGenerator` exported from `ir_to_intel_8008_compiler.__init__`
  alongside the existing (internal) `CodeGenerator` assembly class.

**New tests: `tests/test_codegen_generator.py`** — 14 tests covering: `name`,
`isinstance(gen, CodeGenerator)` structural check, `validate()` on valid /
too-many-registers / unsupported-opcode IR, `validate()` returns `list[str]`
(not `list[IrValidationError]`), `generate()` returns `str`, output contains
`ORG` directive, output contains `HLT`, `generate()` raises on invalid IR,
round-trip, export check.

---

## [0.1.1] — 2026-04-21

### Fixed

#### Critical code generation bugs discovered during end-to-end pipeline testing

The Intel 8008 opcode space has three "dangerous" register slots in Group-01
(MOV) where the destination-is-A instruction is NOT a register copy — those
bit patterns are occupied by other instructions that cause catastrophic behavior
at runtime.  The original code generator was unaware of this and emitted
`MOV A, {reg}` unconditionally.

The three dangerous cases (all in Group-01, where SSS field conflicts occur):

| Instruction emitted | Actual opcode | What the 8008 executes           | Impact                        |
|---------------------|--------------|-----------------------------------|-------------------------------|
| `MOV A, C`          | `0x79`       | `IN 7` (read input port 7)        | Reads wrong value into A      |
| `MOV A, H`          | `0x7C`       | `JMP` (3-byte unconditional jump) | Jumps to garbage address!     |
| `MOV A, M`          | `0x7E`       | `CAL` (3-byte subroutine call)    | Calls arbitrary subroutine!   |

The JMP and CAL cases are particularly catastrophic: the next 2 bytes in the
instruction stream are consumed as a 14-bit address, causing the CPU to jump
into arbitrary memory and execute garbage code.

**Fix: `_load_a(reg)` helper function** (new in this release)

A new module-level helper `_load_a(reg: str) -> list[str]` emits the shortest
safe instruction sequence to load a physical register into the accumulator A:

- For `A`: no instructions needed (already in accumulator).
- For `C`, `H`, `M` (dangerous): emits `MVI A, 0; ADD {reg}` (2 instructions).
  In Group-10 (ALU), SSS=001/100/110 correctly reads C/H/M — no conflict.
  The `MVI A, 0` ensures A=0 so that `ADD {reg}` gives exactly `A = {reg}`.
  Since 0 + reg ≤ 255, the carry flag is always 0 (RFC still fires correctly).
- For all other registers (B, D, E, L): emits `MOV A, {reg}` (1 instruction,
  safe in Group-01 because their SSS codes have no hardware conflict).

**Affected emit methods** — all updated to use `_load_a`:

- `_emit_ret`: `MOV A, C; RFC` → `MVI A, 0; ADD C; RFC` (3 lines)
- `_emit_store_byte`: `MOV A, Rsrc; MOV M, A` → `_load_a(Rsrc); MOV M, A`
- `_emit_add`: `MOV A, Ra; ADD Rb; MOV Rdst, A` → `_load_a(Ra); ADD Rb; MOV Rdst, A`
- `_emit_add_imm`: same pattern for both imm=0 (copy) and imm≠0 cases
- `_emit_sub`, `_emit_and`, `_emit_or`, `_emit_xor`, `_emit_not`: same pattern
- `_emit_cmp_eq`, `_emit_cmp_ne`, `_emit_cmp_lt`: `_load_a(Ra)` before CMP
- `_emit_cmp_gt`: `_load_a(Rb)` (operand-swap trick: Rb into A, then CMP Ra)
- `_emit_branch_z`, `_emit_branch_nz`: `_load_a(Rcond)` before CPI/JTZ/JFZ
- `_emit_load_byte`: `MOV A, M; MOV Rdst, A` → `MVI A, 0; ADD M; MOV Rdst, A`

**Consequence for line counts** — emit methods that previously produced N lines
now produce N+1 lines when the source register is C, H, or M.  All tests
updated accordingly.

**End-to-end verification** — after these fixes, the full Oct → 8008 pipeline
correctly computes `3 + 7 = 10` and writes it to output port 17 on the Intel
8008 simulator.

### Changed

- Updated docstring for `_emit_load_byte` and the top-of-file architecture
  comment to document the `MOV A, M = CAL` hardware conflict.
- Updated all affected tests in `test_ir_to_intel_8008_compiler.py`:
  - `TestEmitRet` — expects 3-line sequence (MVI A, 0; ADD C; RFC)
  - `TestEmitStoreByte` — split into safe-register and dangerous-register cases
  - `TestEmitAdd`, `TestEmitAddImm`, `TestEmitAnd`, `TestEmitNot` — updated
    line counts and exact sequences when Ra=C
  - `TestEmitCmpEq`, `TestEmitCmpNe`, `TestEmitCmpLt` — 7-line sequences
    when Ra=C (was 6)
  - `TestEmitBranchZ` — 4-line sequence when Rcond=C (was 3)
  - `TestEmitLoadByte` — completely rewritten: expects `MVI A, 0; ADD M;
    MOV Rdst, A` (3 lines) instead of `MOV A, M; MOV Rdst, A` (2 lines);
    added `test_uses_add_m_not_mov_a_m` to explicitly guard against regression
  - `TestFullProgram.test_load_and_return` — asserts `MOV A, C` absent
  - `TestFullProgram.test_memory_access_sequence` — asserts `MOV A, M` absent

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
