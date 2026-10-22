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
| `IIRModule` | Top-level container: functions + entry point + language tag + the source release it was compiled against (`release`, LANG84; `None` for languages without a release inventory) |
| `IIRFunction` | Named, parameterised sequence of `IIRInstr` |
| `IIRInstr` | One instruction: `op`, `dest`, `srcs`, `type_hint`, profiling fields |
| `Operand` | Instruction operand: `Var(name)`, `Int`, `Float`, `Bool`, or `Str` |
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
| Strings | `str_const str_len str_index str_concat str_slice str_eq str_cmp print_str` |
| Exceptions (AOT00 T2, inert — no backend lowers these yet) | `throw catch landingpad` |
| Float math (ALGOL AL8) | `f64_sqrt f64_sin f64_cos f64_ln f64_exp f64_atan f64_tan f64_pow` |
| Conversions (LANG-FULL E8) | `int_to_real real_to_int_trunc real_to_int_floor` |
| LANG83 PR1b (lowered by every backend) | `i64_div_floor i64_mod_floor trap` |
| LANG83 PR4 overflow-checked arithmetic (lowered by every backend; trap `Trap.Overflow`) | `i64_add_ovf i64_sub_ovf i64_mul_ovf i64_neg_ovf` |

Type strings: `u8 u16 u32 u64 i8 i16 i32 i64 bool f32 f64 str void any`

Exception kind names (`exception_kind` module) are dotted paths, e.g. `"Trap.Bounds"`;
`catch "Trap"` catches any `Trap.*`, and `"*"` catches anything. The closed table a
LANG83 `trap` may name is the six built-in leaves (`Trap.Bounds`, `Trap.Null`,
`Trap.DivZero`, `Trap.ConvRange`, `Trap.Overflow`, `Trap.User`) plus
`LANGUAGE_TRAP_KINDS`; `trap_kind_id` gives each a stable, append-only integer id.

## The standard operation registry (LANG83)

`op_registry::STANDARD_OPS` names every operation a frontend may emit and every
backend must lower or refuse — output builtins (`print_i64`, `putchar`,
`write_i64`, and since PR6a `write_f64_py` / `write_f64_js`, which print an `f64` as
the shortest round-trip decimal in Python's or JavaScript's layout), `print_str` and the string ops, arithmetic and comparison, `f64` math,
conversions and `trap`. Each backend crate publishes an `OP_SUPPORT` table of
`OpSupport::{Lowered, Refused(reason)}` and checks it with
`op_registry::check_op_support_table`, so adding a registry op fails CI in every
backend until each one decides. `lang_aot::check_module_for` uses those tables to
refuse an unsupported op at compile time instead of miscompiling it.

```rust
use interpreter_ir::op_registry::{lookup, name_rule_violation, OpKind};

assert_eq!(lookup("write_i64").unwrap().kind, OpKind::Builtin);
// New names are type-first: a `div_` prefix would be captured by the native
// backends' prefix dispatch and silently compiled as truncating division.
assert!(name_rule_violation("div_floor").is_some());
```

`opcodes::may_trap`, `opcodes::trap_kinds` and `opcodes::is_terminator` answer the
analysis questions the new ops raise; `trap`, `i64_div_floor`, `i64_mod_floor` and
the four `i64_*_ovf` ops count as side-effecting so dead-code elimination keeps
them. See
`code/specs/LANG83-iir-standard-operation-registry.md`.

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
├── op_registry.rs  — LANG83 standard operation registry + OpSupport
├── exception_kind.rs — exception kind names, trap-kind ids + kind_matches() (AOT00 T2, LANG83)
├── slot_state.rs   — SlotState (type-feedback)
└── serialise.rs    — binary encode / decode
```

---

## Tests

```
cargo test -p interpreter-ir
```

119 unit tests + 52 doctests, all green.
