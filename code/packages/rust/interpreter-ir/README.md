# interpreter-ir

**InterpreterIR (IIR)** — the bytecode representation for the LANG JIT/AOT pipeline.

Language frontends (Tetrad, Nib, Python, Ruby, …) compile to `IIRModule`; the
`vm-core` crate interprets it; `jit-core` specialises hot functions into `CIRInstr`
sequences for native code generation.

---

## Position in the stack

```
Source
  → Frontend (lexer → parser → type-checker → compiler)
  → IIRModule  ← this crate
  → vm-core (interpreter tier)
  → jit-core  (JIT specialiser)
  → CIRInstr → Backend (WASM / JVM / CIL / Intel 4004 / …)
```

---

## Core types

| Type | Description |
|------|-------------|
| `IIRModule` | Top-level container: functions + entry point + language tag |
| `IIRFunction` | Named, parameterised sequence of `IIRInstr` |
| `IIRInstr` | One instruction: `op`, `dest`, `srcs`, `type_hint`, profiling fields |
| `Operand` | Instruction operand: `Var(name)`, `Int`, `Float`, or `Bool` |
| `SlotState` | Per-instruction type-feedback slot (V8 Ignition–style) |
| `FunctionTypeStatus` | `FullyTyped` / `PartiallyTyped` / `Untyped` |

---

## Opcodes

Standard opcodes (handled by `vm-core`):

| Category | Opcodes |
|----------|---------|
| Constant load | `const` |
| Integer arithmetic | `add sub mul div mod neg` |
| Bitwise | `and or xor not shl shr` |
| Integer comparison | `cmp_eq cmp_ne cmp_lt cmp_le cmp_gt cmp_ge` |
| Control flow | `label jmp jmp_if_true jmp_if_false ret ret_void` |
| Register memory | `load_reg store_reg` |
| Flat memory / I/O | `load_mem store_mem io_in io_out` |
| Calls | `call call_builtin` |
| Type system | `cast type_assert` |

Type strings: `u8 u16 u32 u64 i8 i16 i32 i64 bool f32 f64 str void any`

---

## Type-feedback profiling

`IIRInstr` carries a `SlotState` that `vm-core` updates at runtime:

```
Uninitialized → Monomorphic(T) → Polymorphic{T,U,…} → Megamorphic
```

Caps at 4 distinct types.  On the 5th new type the slot goes `Megamorphic` and
stops recording.  `jit-core` reads the slot to decide what specialised code to
emit.

---

## Binary serialisation

```rust
use interpreter_ir::serialise::{serialise, deserialise};

let bytes = serialise(&module);
let module2 = deserialise(&bytes).unwrap();
```

Format: `b"IIR\0"` magic, `u16` version `1.0`, little-endian.  Profiling fields
are **not** serialised (they are runtime-only state).

---

## Quick start

```rust
use interpreter_ir::module::IIRModule;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};

let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
    "u8",
    vec![
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ],
);
let mut module = IIRModule::new("calc.src", "tetrad");
module.add_or_replace(fn_);
assert!(module.validate().is_empty());
```

---

## Crate layout

```
src/
├── lib.rs          — crate root, re-exports
├── module.rs       — IIRModule (container)
├── function.rs     — IIRFunction + FunctionTypeStatus
├── instr.rs        — IIRInstr + Operand
├── opcodes.rs      — opcode category predicates + type helpers
├── slot_state.rs   — SlotState (type-feedback)
└── serialise.rs    — binary encode / decode
```

---

## Tests

```
cargo test -p interpreter-ir
```

38 unit tests + 11 doctests, all green.
