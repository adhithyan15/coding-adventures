# Changelog — iir-to-beam

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-05-11

### Added

- `validate::validate_for_beam(module: &IIRModule) -> Vec<String>` — pre-flight
  validation pass that rejects modules containing BEAM-incompatible instructions
  or types before any lowering starts. Catches:
  - Empty module (no functions)
  - Empty function (function with no instructions)
  - Untyped instructions (`type_hint == "any"` or `"polymorphic"`)
  - Unsupported types (`"str"`, `ref<…>`, float constants)
  - Unsupported opcodes (`call_builtin`, `io_in`, `io_out`, `cast`, memory ops,
    GC ops, `safepoint`)

- `lower::IIRBeamConfig` — lowering configuration, currently just `module_name`.
  Implements `Default` (uses `"iir_module"`) and `new(module_name)`.

- `lower::IIRBeamError` — typed error variants:
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`, `UndefinedLabel`,
  `UndefinedVariable`, `InvalidOperand`. Implements `Display` and `std::error::Error`.

- `lower::lower_iir_to_beam(module: &IIRModule, config: &IIRBeamConfig) -> Result<BEAMModule, IIRBeamError>` —
  two-pass lowering algorithm:
  - Pass 1 per function: assign x-registers to params and variable names, scan
    `label` instructions and assign globally-unique BEAM label numbers.
  - Emit `func_info` preamble for each function (`{label,N}`, `{func_info,...}`,
    `{label,N+1}`).
  - Pass 2 per function: translate each `IIRInstr` to BEAM instructions.
  - Build atom table, import table, exports, and final `BEAMModule`.

- Supported IIR opcodes:
  `const` (Int + Bool), `add`, `sub`, `mul`, `div`, `mod`, `neg`,
  `and`, `or`, `xor`, `not`, `shl`, `shr`,
  `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge`,
  `label`, `jmp`, `jmp_if_true`, `jmp_if_false`,
  `ret`, `ret_void`, `call`, `load_reg`, `store_reg`, `type_assert`.

- `codegen::IIRBeamCodeGenerator` — thin adapter that wires `validate_for_beam`
  and `lower_iir_to_beam` behind the `name()` / `validate()` / `generate()` API.

- Re-exported `BEAMModule` and `encode_beam` from `ir-to-beam` for convenience.

- 45 integration tests in `tests/test_backend.rs` covering validation, lowering,
  instruction emission, register allocation, export table, multi-function modules,
  call sequences, and comparison synthesis.
