# Changelog — iir-to-cil-bytecode

All notable changes to this crate are documented here.

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
