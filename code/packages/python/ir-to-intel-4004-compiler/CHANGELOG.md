# Changelog - coding-adventures-ir-to-intel-4004-compiler

All notable changes to this package will be documented in this file.

## [0.2.0] — 2026-04-27

### Added — LANG20: `Intel4004CodeGenerator` — `CodeGenerator[IrProgram, str]` adapter

**New module: `ir_to_intel_4004_compiler.generator`**

- `Intel4004CodeGenerator` — thin adapter satisfying the
  `CodeGenerator[IrProgram, str]` structural protocol (LANG20).

  ```
  [Optimizer] → [Intel4004CodeGenerator] → str (text assembly)
                                             ├─→ assembler → bytes  (AOT)
                                             └─→ Intel 4004 simulator
  ```

  - `name = "intel4004"` — unique backend identifier.
  - `validate(ir) -> list[str]` — delegates to `IrValidator().validate()`,
    which returns `list[IrValidationError]`; the adapter converts each error
    to its `.message` string so callers receive a plain `list[str]`.  Never
    raises.  Two rules: register count ≤ 12, static RAM ≤ 160 bytes.
  - `generate(ir) -> str` — delegates to `IrToIntel4004Compiler().compile(ir)`.
    Returns Intel 4004 text assembly (`ORG …`, `MVI …`, `HLT`).  Raises
    `IrValidationError` on invalid IR.

- `Intel4004CodeGenerator` exported from `ir_to_intel_4004_compiler.__init__`
  alongside the existing (internal) `CodeGenerator` assembly class.

**New tests: `tests/test_codegen_generator.py`** — 14 tests covering: `name`,
`isinstance(gen, CodeGenerator)` structural check, `validate()` on valid /
too-many-registers / too-much-RAM IR, `validate()` returns `list[str]` (not
`list[IrValidationError]`), `generate()` returns `str`, output contains `ORG`
directive, output contains `HLT`, `generate()` raises on invalid IR,
round-trip, export check.

---

## [0.1.0] - 2026-04-14

### Added

- Renamed the old Intel 4004 backend package to `ir-to-intel-4004-compiler`
- Split hardware feasibility checks into `coding-adventures-intel-4004-ir-validator`
- Kept a facade class that validates IR then emits Intel 4004 assembly
