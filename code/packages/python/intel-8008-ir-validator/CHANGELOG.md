# Changelog

All notable changes to `coding-adventures-intel-8008-ir-validator` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-20

### Added

- **`IrValidator`** — single class with a `validate(program: IrProgram) -> list[IrValidationError]`
  method.  Runs all six checks and returns every violation at once.

- **`IrValidationError`** — dataclass/Exception with `rule: str` and `message: str`
  fields.  Inherits from `Exception` so it can be raised by the backend as well as
  collected into a list by the validator.

- **Six validation rules** (all accumulated, never fail-fast):

  1. **`no_word_ops`** — `LOAD_WORD` and `STORE_WORD` are forbidden.  The Intel 8008
     has an 8-bit data bus; all memory accesses are byte-wide via the `M`
     pseudo-register (memory at H:L).  WORD variants have no hardware instruction to
     lower them to.

  2. **`static_ram`** — Total size of all `IrDataDecl` entries must not exceed 8 191
     bytes.  The 8008 RAM region is 0x2000–0x3FFF (8 192 bytes); one guard byte is
     reserved, leaving 8 191 usable bytes for static variables.

  3. **`call_depth`** — Static call graph depth must not exceed 7.  The 8008 has an
     8-level hardware push-down stack.  Level 0 is always the current PC, leaving 7
     levels for `CAL` instructions.  Recursive call graphs (cycles) are always rejected
     before the depth measurement.

  4. **`register_count`** — No more than 6 distinct virtual register indices may appear
     across all instruction operands.  The Oct calling convention maps v0–v5 to the
     8008's B, A, C, D, E registers (H and L are reserved for memory addressing).
     Virtual registers v6 and above have no physical home without RAM spilling, which
     the code generator does not implement.

  5. **`imm_range`** — Every `LOAD_IMM` and `ADD_IMM` immediate must be in [0, 255].
     The 8008 immediate instructions (`MVI`, `ADI`, `ACI`, etc.) encode their operand
     in a single byte.  Values outside [0, 255] cannot be encoded.

  6. **`syscall_whitelist`** — Every `SYSCALL` instruction must use a number from the
     8008 intrinsic whitelist:
     - 3–4: `adc`/`sbb` (ADC r / SBB r)
     - 11–16: rotations (`rlc`, `rrc`, `ral`, `rar`), `carry()`, `parity()`
     - 20–27: `in(p)` for ports 0–7 (IN p)
     - 40–63: `out(p, v)` for ports 0–23 (OUT p)

     Each invalid SYSCALL number is reported individually, but the same number
     appearing multiple times is reported only once.

- **Comprehensive test suite** (`tests/test_intel_8008_ir_validator.py`):
  - 8 test classes, 60+ individual test cases
  - Each rule tested with: passing case, failing case, boundary values, edge cases
  - Full accumulation test (all six rules triggered simultaneously)
  - Clean-program test (zero errors on a well-formed IR)
  - `IrValidationError` type contract tests (equality, hash, raise, __str__)

### Design Notes

- **Structurally parallel to `intel-4004-ir-validator`**: same `IrValidator` /
  `IrValidationError` pattern, same test layout.  The 8008 validator extends the
  4004's 5 rules to 6 by splitting the old `operand_range` rule into `imm_range`
  (which covers both `LOAD_IMM` and `ADD_IMM`) and adding `syscall_whitelist`
  (specific to the 8008's port I/O model).

- **Accumulate, never abort**: the validator always runs all six checks.  This
  matches the design philosophy of the 4004 validator and gives the programmer a
  complete error report in a single compilation.

- **SYSCALL deduplication**: the same invalid SYSCALL number appearing N times in a
  program produces exactly one error.  Different invalid numbers each get their own
  error.  This prevents error floods in programs with many repeated bad syscalls.

- **Call graph cycle detection**: done via a two-phase DFS (visiting/visited sets).
  Cycles are caught before the depth DFS, because depth is undefined in a cyclic
  graph.  The 8008 stack wraps without detection — a recursive program would
  silently corrupt its own return addresses.
