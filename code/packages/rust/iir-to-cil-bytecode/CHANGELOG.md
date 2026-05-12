# Changelog — iir-to-cil-bytecode

All notable changes to this crate are documented here.

## [0.3.0] — 2026-05-12

### Added (LANG35 — Closure Backend Integration)

#### Improved `ClosureOpcode` validator error

- `validate_iir_for_clr` now emits a dedicated `ClosureOpcode` error message
  (format: `"[fn_name] ClosureOpcode: alloc_closure/call_closure require the
  BEAM backend — CLR does not support heap-allocated closures"`) when it
  encounters `alloc_closure` or `call_closure`.
- Previously these fell through to the generic `UntypedInstruction` path;
  the closure check now runs first to give a more actionable error message.

#### Tests

- `lang35_alloc_closure_closure_opcode_error`: asserts `validate_iir_for_clr`
  returns an error containing "ClosureOpcode" for a module with `alloc_closure`.
- `lang35_call_closure_closure_opcode_error`: same for `call_closure`.
- `lang35_closure_opcode_error_not_untyped`: asserts the error does NOT
  contain "UntypedInstruction".

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O)

#### I/O support

- `io_out %v` → `ldloc <slot>; call System.Console.WriteLine(int64)`.
  Uses token `CONSOLE_WRITELINE_I64_TOKEN = 0x0A00_0002` (pre-defined
  member reference to `Console.WriteLine(long)`).

#### Global variables (LANG32b — deferred)

- `global_load` and `global_store` return `UnsupportedOp` with a clear
  LANG32b tracking note.  Full CLR static-field globals require extending
  `CILProgramArtifact` with a fields table and adding `ldsfld`/`stsfld`
  sequences; tracked in a follow-up PR.

#### Exhaustiveness fixes

- `Operand::Str` arms added to all `match` blocks in `lower.rs` (const,
  call argument loop).

---

## [0.1.0] — 2026-05-11

### Added

- Initial release.
- `validate_iir_for_clr(module: &IIRModule) -> Vec<String>` — pre-flight
  validator that checks for empty modules/functions, untyped instructions
  (`"any"` / `"polymorphic"` type hints), unsupported types (`"str"`,
  `"ref<…>"`), float constants (unsupported in CLR v1), and unsupported
  opcodes.
- `IIRClrConfig` — backend configuration struct (assembly name).
- `IIRClrError` — rich error enum with function-scoped context for all
  failure modes: `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`,
  `UndefinedLabel`, `UndefinedVariable`, `InvalidOperand`, `AssemblyError`.
- `lower_iir_to_cil(module: &IIRModule, config: &IIRClrConfig) -> Result<CILProgramArtifact, IIRClrError>`
  — two-pass register allocator + CIL emitter that lowers every IIRFunction
  to a `CILMethodArtifact` (assembled CIL body bytes).
- `IIRClrCodeGenerator` — `codegen_core::CodeGenerator<IIRModule, CILProgramArtifact>`
  adapter so the backend participates in the shared code-generator protocol.
- Opcode coverage: `const`, `add`, `sub`, `mul`, `div`, `mod`, `neg`,
  `and`, `or`, `xor`, `not`, `shl`, `shr`, `cmp_eq`, `cmp_ne`, `cmp_lt`,
  `cmp_le`, `cmp_gt`, `cmp_ge`, `label`, `jmp`, `jmp_if_true`,
  `jmp_if_false`, `ret`, `ret_void`, `call`, `load_reg`, `store_reg`,
  `type_assert`.
- 47 integration tests in `tests/test_backend.rs`.
