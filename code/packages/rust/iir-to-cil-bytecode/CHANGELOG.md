# Changelog — iir-to-cil-bytecode

All notable changes to this crate are documented here.

## [0.4.0] — 2026-05-12

### Added (LANG37 — CLR Closure Lowering)

#### `int32[]`-based closure representation

The CLR backend now supports first-class closures (LANG34 `alloc_closure` /
`call_closure` opcodes) using an `int32[]` dispatch-table approach.

A closure is represented as an `int32[]` array:
- `closure[0]` — function dispatch index (alphabetical among closure targets)
- `closure[1..n]` — captured values (all stored as `int32`)

#### `alloc_closure` lowering

`alloc_closure(Str("fn_name"), Var(cap0), …) : "closure"` lowers to:

```cil
ldc.i4 {n+1}              ; array size = 1 (idx) + n (captures)
newarr [System.Int32]     ; int32[] closure_arr = new int32[n+1]
dup
ldc.i4.0
ldc.i4 {dispatch_idx}
stelem.i4                 ; closure_arr[0] = dispatch_idx
dup
ldc.i4.1
ldloc cap0_slot
stelem.i4                 ; closure_arr[1] = cap0
…
stloc dest_slot           ; dest = closure_arr
```

#### `call_closure` lowering

`call_closure(Var(handle), Var(arg0), …) : "any"` lowers to:

```cil
ldloc handle_slot         ; push closure handle (int32[])
ldc.i4 {n_args}           ; args array size
newarr [System.Int32]     ; int32[] args_arr = new int32[n_args]
dup
ldc.i4.0
ldloc arg0_slot
stelem.i4                 ; args_arr[0] = arg0
…
call int32 ClassName::__callClosure(int32[], int32[])
stloc dest_slot           ; dest = result
```

#### Synthetic `__callClosure` dispatch method

When any `alloc_closure` instruction is present in the module,
`lower_iir_to_cil` appends a synthetic `__callClosure(int32[], int32[]) →
int32` static method.  It reads `closure[0]` and dispatches to the correct
user function via a chain of `ldc.i4 N; beq case_N` branches.

Token: `0x0600_0001 + module.functions.len()` (the next slot after all user
functions in the MethodDef table).

#### New token

- `INT32_ARRAY_TYPE_TOKEN = 0x0100_0002` added to `ir-to-cil-bytecode`
  alongside the existing `OBJECT_ARRAY_TYPE_TOKEN = 0x0100_0001`.  Used with
  `newarr` to allocate `int32[]` closure and argument arrays.

#### Validator changes

`validate_iir_for_clr` now:
- **Accepts** `alloc_closure` with `i32`/`bool` captures (LANG37 early-accept).
- **Accepts** `call_closure` unconditionally (type_hint `"any"` is fine here).
- **Rejects** `alloc_closure` with `i64`/`u64`/`f32`/`f64` captures with a
  `ClosureOpcode` error: `"only i32/bool captures are supported by the CLR
  backend in v1 — use integer types or upgrade to LANG38"`.

#### Tests

- `lang37_alloc_closure_i32_cap_accepted_by_clr_validator`: i32 capture passes.
- `lang37_call_closure_accepted_by_clr_validator`: call_closure passes.
- `lang37_i64_capture_still_rejected`: i64 capture → ClosureOpcode.
- `lang37_float_capture_still_rejected`: f32 capture → ClosureOpcode.
- `lang37_alloc_closure_emits_newarr`: alloc_closure emits `newarr` (0x8D).
- `lang37_alloc_closure_emits_stelem_i4`: alloc_closure emits `stelem.i4` (0x9E).
- `lang37_call_closure_emits_call_dispatch`: call_closure emits `call` (0x28).
- `lang37_dispatch_method_generated`: artifact contains `__callClosure` method.
- `lang37_dispatch_method_contains_ldelem_i4`: dispatch body has `ldelem.i4` (0x94).

#### Deferred

- i64/f32/f64 closure captures — LANG38.
- WASM closure lowering — LANG38.
- Real .NET round-trip test — LANG39.

---

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
