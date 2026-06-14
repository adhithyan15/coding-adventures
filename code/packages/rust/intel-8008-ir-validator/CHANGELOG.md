# Changelog — intel-8008-ir-validator

## [0.1.0] — 2026-04-28

### Added

- **`IrValidator`** — hardware-constraint validator for Intel 8008 IR programs.
  - `validate(program) -> Vec<ValidationDiagnostic>` — runs all six checks in
    a single pass and accumulates every violation.

- **`ValidationDiagnostic`** — typed validation error with `rule` and `message`
  fields; implements `Display` as `"[rule] message"`.

- **Six validation rules:**
  1. **`no_word_ops`** — rejects `LOAD_WORD` and `STORE_WORD`; the 8008 has an
     8-bit data bus and no 16-bit memory instruction.  At most one error per
     forbidden opcode type.
  2. **`static_ram`** — sum of all `IrDataDecl.size` values must not exceed
     8 191 bytes (RAM region 0x2000–0x3FFE minus one guard byte).
  3. **`call_depth`** — static call-graph DFS depth must not exceed 7 (8-level
     hardware stack; level 0 = current PC).  Implements three-colour DFS for
     cycle detection (recursive programs always rejected) followed by a
     max-depth DFS with per-branch visited-set copies.
  4. **`register_count`** — distinct virtual register indices must not exceed 6
     (v0–v5 map to B, A, C, D, E, and one spare; H and L are reserved for
     memory addressing).
  5. **`imm_range`** — every `LOAD_IMM` and `ADD_IMM` immediate must be in
     [0, 255] (fits in the 8-bit immediate field of MVI/ADI).  One error per
     out-of-range occurrence.
  6. **`syscall_whitelist`** — `SYSCALL` numbers must be in
     {3, 4} ∪ {11–16} ∪ {20–27} ∪ {40–63} (hardware intrinsics with a defined
     8008 lowering).  Each unique invalid number reported once.

- **Helper functions** (module-private):
  - `valid_syscalls() -> HashSet<i64>` — returns the whitelist set.
  - `find_cycle()` — three-colour DFS cycle detection; returns cycle path.
  - `dfs_depth()` — per-branch copy-on-extend depth measurement.

- **Tests** — 25 tests covering all six rules, boundary values, and error
  accumulation.  Tests mirror the Python `intel_8008_ir_validator` test suite.
