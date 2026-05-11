# Changelog — iir-to-jvm-class-file

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-05-11

### Added

- `validate::validate_for_jvm(module: &IIRModule) -> Vec<String>` — pre-flight
  validation pass that rejects modules containing JVM-incompatible instructions
  or types before any lowering starts. Catches:
  - Empty module (no functions)
  - Empty function (function with no instructions)
  - Untyped instructions (`type_hint == "any"` or `"polymorphic"`)
  - Unsupported types (`"str"`, `ref<…>`)
  - Unsupported opcodes (`call_builtin`, `io_in`, `io_out`, `cast`, memory ops,
    GC ops, `safepoint`)
  - Float type hints and float constants are **supported** (unlike the BEAM
    backend), since the JVM has native `fload`/`dload`/`fadd`/`dadd` opcodes.

- `lower::IIRJvmConfig` — lowering configuration: `class_name` String.
  Implements `Default` (uses `"IIRModule"`) and `new(class_name)`.

- `lower::IIRJvmError` — typed error variants:
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`, `UndefinedLabel`,
  `UndefinedVariable`, `InvalidOperand`. Implements `Display` and `std::error::Error`.

- `lower::lower_iir_to_jvm(module: &IIRModule, config: &IIRJvmConfig) -> Result<JvmClassFile, IIRJvmError>` —
  two-pass lowering algorithm:
  - Pass 1 per function: assign JVM local variable slots to params (0..N-1)
    then walk dests and src Var operands in order for locals (N..).
  - Pass 2: emit raw JVM bytecode (Vec<u8>) per method using emit_* helpers.
  - Build `JvmClassFile` directly (Java 8, version 52.0).
  - Two-pass backpatching for forward label/jump references.

- Supported IIR opcodes:
  `const` (Int, Float, Bool), `add`, `sub`, `mul`, `div`, `mod`, `neg`,
  `and`, `or`, `xor`, `not`, `shl`, `shr`,
  `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge`,
  `label`, `jmp`, `jmp_if_true`, `jmp_if_false`,
  `ret`, `ret_void`, `call`, `load_reg`, `store_reg`, `type_assert`.

- Type mapping: `i8/i16/i32/u8/u16/u32/bool → int (I)`, `i64/u64 → long (J)`,
  `f32 → float (F)`, `f64 → double (D)`, `void → void (V)`.

- `codegen::IIRJvmCodeGenerator` — thin adapter that wires `validate_for_jvm`
  and `lower_iir_to_jvm` behind the `name()` / `validate()` / `generate()` API.

- 40+ integration tests in `tests/test_backend.rs` covering validation, lowering,
  instruction emission, register allocation, multi-function modules, float support,
  comparison synthesis, and bytecode non-emptiness checks.
