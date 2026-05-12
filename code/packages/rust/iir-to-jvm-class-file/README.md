# iir-to-jvm-class-file

**IIR → JVM class file backend** — lowers an `IIRModule` (from the
`interpreter-ir` crate) directly to a `JvmClassFile` (from the `jvm-class-file`
crate) **without going through the deprecated `compiler-ir` layer**.

## Overview

```
IIRModule                              ← interpreter-ir crate
   │
   ▼  validate_for_jvm()
   │  Returns Vec<String> of errors; empty = safe to lower.
   │
   ▼  lower_iir_to_jvm()
JvmClassFile                           ← jvm-class-file crate
```

The `compiler-ir` crate represents a flat, single-function machine IR.
`interpreter-ir` (IIR) is richer: it carries named variables, static type hints,
multiple functions with parameters, labels, and comparison operators.  This crate
bridges that richer world to the JVM class file format without any loss of
information through a deprecated intermediate.

## Why IIR → JVM directly?

The JVM is a **stack machine** with a rich type system.  IIR's named variables
and explicit type hints map naturally to JVM local variable slots and typed
load/store opcodes:

| IIR concept           | JVM concept                                    |
|-----------------------|------------------------------------------------|
| Named variable        | Local variable slot (allocated by a two-pass scan) |
| `const` (Int)         | `iconst_N` / `bipush` / `sipush` / `ldc`      |
| `const` (Float/Double)| `fconst` / `dconst` via constant pool         |
| `add` (i32)           | `iadd`                                         |
| `add` (i64)           | `ladd`                                         |
| `add` (f32)           | `fadd`                                         |
| `add` (f64)           | `dadd`                                         |
| `cmp_eq` (int)        | `if_icmpne` synthesis + `iconst_1` / `iconst_0` |
| `label`               | Backpatch target in bytecode stream            |
| `jmp`                 | `goto` + backpatch                             |
| `jmp_if_true`         | `iload cond; ifne` + backpatch                 |
| `jmp_if_false`        | `iload cond; ifeq` + backpatch                 |
| `ret` (i32)           | `iload result; ireturn`                        |
| `ret_void`            | `return`                                       |
| `call fn`             | `iload args…; invokestatic; istore dest`       |

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
use iir_to_jvm_class_file::{validate_for_jvm, lower_iir_to_jvm, IIRJvmConfig};

// Build a module with one function: add(a: i32, b: i32) -> i32
let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
    "i32",
    vec![
        IIRInstr::new("add", Some("result".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None,
            vec![Operand::Var("result".into())], "i32"),
    ],
);
let module = IIRModule {
    name: "mymod".into(),
    functions: vec![fn_],
    entry_point: Some("add".into()),
    language: "tetrad".into(),
};

// Step 1 — validate
let errors = validate_for_jvm(&module);
assert!(errors.is_empty(), "validation errors: {:?}", errors);

// Step 2 — lower
let config = IIRJvmConfig::new("MyClass");
let class_file = lower_iir_to_jvm(&module, &config).unwrap();
assert_eq!(class_file.methods.len(), 1);
assert_eq!(class_file.this_class_name, "MyClass");
```

## Supported opcodes

| IIR op         | JVM emission (i32 example)                                |
|----------------|-----------------------------------------------------------|
| `const` (Int)  | `iconst_N` / `bipush v` / `sipush v` / `ldc` + cp entry  |
| `const` (Bool) | `iconst_0` or `iconst_1`                                  |
| `const` (Float)| `fconst_0/1/2` or `ldc` via constant pool                 |
| `add`          | `iadd` / `ladd` / `fadd` / `dadd`                         |
| `sub`          | `isub` / `lsub` / `fsub` / `dsub`                         |
| `mul`          | `imul` / `lmul` / `fmul` / `dmul`                         |
| `div`          | `idiv` / `ldiv` / `fdiv` / `ddiv`                         |
| `mod`          | `irem` / `lrem`                                           |
| `neg`          | `ineg` / `lneg` / `fneg` / `dneg`                         |
| `and`          | `iand` / `land`                                           |
| `or`           | `ior` / `lor`                                             |
| `xor`          | `ixor` / `lxor`                                           |
| `not`          | `iconst_1; ixor` (XOR with 1 to flip LSB for booleans)    |
| `shl`          | `ishl` / `lshl`                                           |
| `shr`          | `ishr` / `lshr`                                           |
| `cmp_eq`       | `if_icmpne +7; iconst_1; goto +4; iconst_0`               |
| `cmp_ne`       | `if_icmpeq +7; iconst_1; goto +4; iconst_0`               |
| `cmp_lt`       | `if_icmpge +7; iconst_1; goto +4; iconst_0`               |
| `cmp_le`       | `if_icmpgt +7; iconst_1; goto +4; iconst_0`               |
| `cmp_gt`       | `if_icmple +7; iconst_1; goto +4; iconst_0`               |
| `cmp_ge`       | `if_icmplt +7; iconst_1; goto +4; iconst_0`               |
| `label`        | Backpatch target — no bytes emitted                       |
| `jmp`          | `goto <offset>` + backpatch fixup                         |
| `jmp_if_true`  | `iload cond; ifne <offset>` + backpatch                   |
| `jmp_if_false` | `iload cond; ifeq <offset>` + backpatch                   |
| `ret`          | `iload/lload/fload/dload result; ireturn/lreturn/…`       |
| `ret_void`     | `return`                                                  |
| `call`         | `iload args…; invokestatic CP#; istore dest`              |
| `load_reg`     | `iload/lload/fload/dload src; istore/lstore/… dest`       |
| `store_reg`    | same as `load_reg`                                        |
| `type_assert`  | nop (erased at lowering time)                             |

## Unsupported (validation rejects)

`call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`, `store_mem`, `alloc`,
`box`, `unbox`, `field_load`, `field_store`, `is_null`, `safepoint`, and any
instruction with `type_hint` of `"any"`, `"polymorphic"`, `"str"`, or `"ref<…>"`.

> **LANG35 note**: `alloc_closure` and `call_closure` (LANG34/LANG35 first-class
> closure opcodes) are BEAM-only and return a `ClosureOpcode` validation error
> rather than the generic `UntypedInstruction` message.

Note: float type hints (`f32`, `f64`) and float constant operands **are supported**
(unlike the BEAM backend).

## Type mapping

| IIR type                              | JVM descriptor | Slot width |
|---------------------------------------|----------------|------------|
| `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `bool` | `I` (int)  | 1 |
| `i64`, `u64`                          | `J` (long)     | 2          |
| `f32`                                 | `F` (float)    | 1          |
| `f64`                                 | `D` (double)   | 2          |
| `void`                                | `V` (void)     | 0          |

## Register allocation

Variables are allocated to JVM local variable slots via a deterministic
two-pass scan:

1. Function parameters → slots 0..N-1 (in order)
2. All `dest` names and `Var` src operands → next available slot,
   or the existing slot if the name was already seen.

The same variable name always maps to the same slot within one function.
This is a simple linear scan — no liveness analysis, no spilling.

## Label/jump backpatching

The lowering pass emits `goto` and `if*` instructions with a two-byte placeholder
offset and records a `Fixup { opcode_pos, target_label }`.  After the entire
function's bytecode is emitted, a second pass resolves all fixups:

```
offset = (target_pc as i32 - opcode_pos as i32) as i16
```

The JVM specifies that branch offsets are signed 16-bit values measured from the
start of the branch instruction's opcode byte — exactly what this formula produces.

## Crate layout

| Module    | Responsibility                                                  |
|-----------|-----------------------------------------------------------------|
| `validate`| Pre-flight checks on `IIRModule`                                |
| `lower`   | Two-pass IIR → JVM bytecode lowering; error types; config       |
| `codegen` | `IIRJvmCodeGenerator` — thin adapter (`name` / `validate` / `generate`) |
| `lib`     | Re-exports; crate entry point                                   |
