# iir-to-cil-bytecode

Lowers an `IIRModule` (from `interpreter-ir`) directly to a `CILProgramArtifact`
(from `ir-to-cil-bytecode`) **without going through the deprecated `compiler-ir`
layer**.

## What is CIL?

Common Intermediate Language (CIL) is the stack-based bytecode format of the
Common Language Runtime (CLR), the virtual machine powering .NET, Mono, and
Xamarin.  Unlike the JVM — which encodes types in the opcode (`iadd` for
integers, `fadd` for floats) — CIL infers operand types from the evaluation
stack at JIT time:

```text
JVM:  iadd          ← "i" means int32, encoded in the opcode
CIL:  add           ← type is inferred at JIT time from the stack
```

## Pipeline

```text
IIRModule
  → validate_iir_for_clr()      — pre-flight validation
  → lower_iir_to_cil()          — emit CIL body bytes per function
  → CILProgramArtifact          — structured multi-method artifact
       ↓ (future) CLR packager  — wrap in PE/COFF .dll/.exe
       ↓ CLR simulator          — run directly
```

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
use iir_to_cil_bytecode::{IIRClrConfig, lower_iir_to_cil, validate_iir_for_clr};

// Build: add(a: i32, b: i32) -> i32  { ret a + b }
let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
    "i32",
    vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None,
            vec![Operand::Var("v0".into())], "i32"),
    ],
);
let mut module = IIRModule::new("example", "tetrad");
module.entry_point = Some("add".into());
module.add_or_replace(fn_);

let errors = validate_iir_for_clr(&module);
assert!(errors.is_empty());

let config = IIRClrConfig::default();
let artifact = lower_iir_to_cil(&module, &config).unwrap();
assert!(!artifact.methods[0].body.is_empty());
assert!(artifact.methods[0].body.contains(&0x2A)); // ret
```

## Closure lowering (LANG37)

Since LANG37 the CLR backend supports first-class closures via an `int32[]`
dispatch table.

```rust
// alloc_closure: build a closure over captured i32 values
// call_closure: call a closure at runtime via __callClosure dispatch
```

| Closure type | Support |
|---|---|
| `i32` / `bool` captures | ✅ LANG37 |
| `i64` / `u64` / `f32` / `f64` captures | ❌ LANG38 (ClosureOpcode error) |

A closure is an `int32[]` array:
- `[0]` = function dispatch index (alphabetical order, deterministic)
- `[1..n]` = captured values as `int32`

A synthetic `__callClosure(int32[], int32[]) → int32` method is appended to
the program artifact whenever any `alloc_closure` instruction appears.

## Validation errors

| Error | Condition |
|-------|-----------|
| `EmptyModule` | Module has no functions |
| `EmptyFunction` | A function has no instructions |
| `ClosureOpcode` | `alloc_closure` with i64/u64/f32/f64 capture — deferred to LANG38 |
| `UntypedInstruction` | `type_hint` is `"any"` or `"polymorphic"` (except `call_closure` which is exempt) |
| `UnsupportedType` | `type_hint` is `"str"` or starts with `"ref<"` (except `ref<LispyPair>`) |
| `UnsupportedOp` | Any unsupported opcode (see below) |

## Supported IIR opcodes

| Category | Opcodes |
|----------|---------|
| Constants | `const` (Int, Bool; Float **unsupported** in v1) |
| Arithmetic | `add`, `sub`, `mul`, `div`, `mod`, `neg` |
| Bitwise | `and`, `or`, `xor`, `not`, `shl`, `shr` |
| Comparison | `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge` |
| Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
| Return | `ret`, `ret_void` |
| Calls | `call`, `call_closure` (LANG37) |
| Closures | `alloc_closure` (LANG37, i32/bool captures only) |
| Heap | `alloc` (`ref<LispyPair>` only), `field_load`, `field_store`, `is_null` |
| Register | `load_reg`, `store_reg` |
| Coercion | `type_assert` (becomes `nop`) |

## How CIL synthesis works for derived operations

CIL lacks native opcodes for some logical operations, so we synthesize them
from primitives:

| IIR op | CIL synthesis |
|--------|---------------|
| `mod` | `rem` opcode (0x5D — not in the `CILOpcode` enum, emitted as raw byte) |
| `neg` | `neg` opcode (0x65 — raw byte) |
| `not` | `not` opcode (0x66 — bitwise complement, raw byte) |
| `cmp_ne r1, r2` | `ceq; ldc.i4.0; ceq` (NOT of equal) |
| `cmp_le r1, r2` | `cgt; ldc.i4.0; ceq` (NOT of greater-than) |
| `cmp_ge r1, r2` | `clt; ldc.i4.0; ceq` (NOT of less-than) |

## Register model

IIR uses named SSA variables.  This backend maps them to CIL locals and
method arguments:

- **Parameters** → `ldarg`/`starg` (indices 0..N-1).
- **Locals** → `ldloc`/`stloc` (indices 0..M-1 where M = unique vars − params).

A two-pass scan assigns each distinct variable name a stable slot index.

## Module structure

| Module | Contents |
|--------|----------|
| `validate` | `validate_iir_for_clr` — pre-flight checks |
| `lower` | `IIRClrConfig`, `IIRClrError`, `lower_iir_to_cil` — main lowering pass |
| `codegen` | `IIRClrCodeGenerator` — `CodeGenerator` protocol adapter |

## How it fits in the stack

```text
Language Frontend
     │  IIRModule
     ▼
iir-to-cil-bytecode   ← this crate
     │  CILProgramArtifact
     ▼
CLR simulator / PE packager
```
