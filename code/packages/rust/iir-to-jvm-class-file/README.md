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
| `jmp_if_true`  | `iload cond; ifne <offset>` (i32) / `lload; lconst_0; lcmp; ifne` (i64) + backpatch |
| `jmp_if_false` | `iload cond; ifeq <offset>` (i32) / `lload; lconst_0; lcmp; ifeq` (i64) + backpatch |
| `ret`          | `iload/lload/fload/dload result; ireturn/lreturn/…`       |
| `ret_void`     | `return`                                                  |
| `call`         | `iload args…; invokestatic CP#; istore dest`              |
| `load_reg`       | `iload/lload/fload/dload src; istore/lstore/… dest`         |
| `store_reg`      | same as `load_reg`                                          |
| `type_assert`    | nop (erased at lowering time)                               |
| `alloc_closure`  | `newarray T_LONG; lastore (×N); astore dest` — LANG36       |
| `call_closure`   | `newarray T_LONG; lastore (×M); invokestatic __callClosure` — LANG36 |
| `alloc_bytes`    | no bytecode — the tape is the static `env/BFRuntime.__tape : [B` (LM-J) |
| `load_byte`      | `getstatic __tape; <idx>; baload; sipush 0xFF; iand` (+`l2i`/`i2l` for i64) — LM-J |
| `store_byte`     | `getstatic __tape; <idx>; <val>; bastore` (+`l2i` for i64) — LM-J |
| `call_builtin putchar`/`getchar` | `invokestatic env/BFRuntime.putchar(I)V` / `getchar()I` — Brainfuck `.`/`,` |
| `alloc_array`    | `<count>; [l2i]; newarray T_<elem>; astore dest` — native `int[]`/`long[]`/`double[]` (E5); a `str` element → `anewarray java/lang/String` (`String[]`, E4d-BA-arr) |
| `array_get`      | `aload handle; <idx>; [l2i]; <T>aload; store dest` — native bounds check (E5); a `str` element uses `aaload` (E4d-BA-arr) |
| `array_set`      | `aload handle; <idx>; [l2i]; <val>; <T>astore` — native bounds check (E5); a `str` element uses `aastore` (E4d-BA-arr) |
| `array_len`      | `aload handle; arraylength; [i2l]; store dest` — E5 |
| `str_const`      | `ldc CONSTANT_String; astore dest` — ASCII literal-output foothold (E4) |
| `str_concat`     | `aload a; aload b; invokevirtual java/lang/String.concat(String); astore dest` — literal append foothold (E4) |
| `str_len`        | `aload s; invokevirtual java/lang/String.length()I; [i2l]; store dest` — literal length foothold (E4) |
| `str_index`      | `aload s; <idx>; invokevirtual java/lang/String.charAt(I)C; [i2l]; store dest` — literal index foothold (E4) |
| `str_eq`         | `aload a; aload b; invokevirtual java/lang/String.equals(Object)Z; [i2l]; store dest` — literal equality foothold (E4) |
| `str_cmp`        | `aload a; aload b; invokevirtual java/lang/String.compareTo(String)I; invokestatic java/lang/Integer.signum(I)I; [i2l]; store dest` — literal ordering foothold (E4) |
| `print_str`      | `getstatic System.out; aload s; invokevirtual PrintStream.print(String)` — E4 |

The byte-tape ops (`alloc_bytes`/`load_byte`/`store_byte`) are how Brainfuck runs on
the JVM (LANG-MATRIX LM-J): the tape is a host-provided static `byte[]`, `baload`/`bastore`
index it (masking the sign-extended load back to an unsigned cell), and `.`/`,` call the
`env.BFRuntime` host class — the JVM sibling of the LLVM libc / wasm `env.putchar` I/O.

The E4 string rows are intentionally a narrow literal-output slice: Dartmouth BASIC
`PRINT "HELLO"` now runs on real `java`, and Twig `(string-length "HELLO")`
uses `String.length()`, `(string-ref "ABC" 1)` uses `String.charAt(I)`,
`(string=? "HELLO" "HELLO")` uses `String.equals(Object)`, and
`(string<? "ALPHA" "BETA")` uses `String.compareTo(String)`, while
`(string-length (string-append "AB" "CDE"))` uses `String.concat(String)` plus
`String.length()`. Non-literal string values remain rejected until the JVM
backend owns the shared UTF-8 byte semantics.

**Narrow-width register arithmetic wraps mod-2ⁿ** (LANG-FULL E2): narrow **unsigned**
integers (`u4`/`u8`/`u16`/`u32`) use the JVM **`int` model** — `int` locals, `I` descriptors,
the int opcodes (`iadd`/`iand`/…), and the result masked with `iconst/sipush/ldc <mask>;
iand` — so `200u8+100u8=44` and `~0u8=255`. JVM `int` ops already wrap mod-2³², so `u32`/`i32`
need no mask. A positive mask + `iand` is used (not `i2b`/`i2s`, which sign-extend) to keep
the unsigned widths unsigned.

A scalar program reaches this backend through `lang_aot::concretize_scalar_any_for_jvm`,
which narrows the module's `i64`→`i32` *before* lowering (the in-repo `jvm-simulator` is
32-bit and a scalar entry must `ireturn`). So a narrow op already meets `i32` operands — the
int op + int mask are operand-consistent. *(v0.13.0 briefly used a `long` register model,
like wasm; that was reverted in v0.13.1 because it conflicts with `concretize` — it left the
narrow op `long` while the consts/return were `int`, producing unverifiable bytecode. wasm
keeps genuine `i64` operands with no concretize-to-i32, so its i64 model stands; the JVM is
the odd one out because of the 32-bit-simulator concretization.)*

**Narrow mask on the `long` model** (LANG-FULL O2, v0.14.0): a *printing* program (Oct's
`out`, BASIC's `PRINT`) is **not** concretized — it keeps the `i64`/`long` model so its value
can reach `print_i64`. Oct's only integer type is `u8`, so a printing Oct program emits a
narrow-hinted op (`200u8 + 100u8`, `~0u8`) over **`long`** operands. There the int op + int
mask would be unverifiable, so `narrow_op_over_long` keeps the op on the long model
(`ladd`/`lxor`/…) and the mask becomes `i2l; land` (the masks are positive, so widening
zero-extends). It keys off the actual operand types, so concretized int-model programs are
untouched. This is what makes Oct `200u8+100u8=44` / `~0u8=255` run on real `java`.

## Closures (LANG36)

The JVM backend supports **first-class closures** via a `long[]`-based
dispatch-table approach.

### Closure representation

A JVM closure is a `long[]` array:

```
closure[0]  = function dispatch index (u32 as long)
closure[1]  = first captured value (as long)
closure[2]  = second captured value (as long)
…
```

Integer captures (`i32`/`u32`/`bool`) are sign-extended to `long` via `i2l`.
`i64`/`u64` captures are stored directly.  Float captures (`f32`, `f64`) are
deferred — they still produce a `ClosureOpcode` validation error.

### `__callClosure` dispatch method

When a module contains any `alloc_closure` instruction, the lowering pass
automatically generates a synthetic static method:

```
static long __callClosure(long[] closure, long[] args)
```

This method reads `closure[0]` and dispatches to the correct underlying static
method via a chain of `lcmp`/`ifeq` branches — one per closure-eligible function.
Dispatch indices are alphabetically assigned for deterministic output.

### Quick example

```rust
// IIR:
//   __adder(x: i64, cap: i64) -> i64: ret x + cap
//   main():
//     c = alloc_closure(Str("__adder"), Var("cap"))   ; closure over cap=3
//     r = call_closure(Var("c"), Var("arg"))           ; call with arg=4
//     ret r                                            ; → 7
```

## Unsupported (validation rejects)

`call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`, `store_mem`, `alloc`,
`box`, `unbox`, `field_load`, `field_store`, `is_null`, `safepoint`, and any
instruction with `type_hint` of `"any"`, `"polymorphic"`, `"str"`, or `"ref<…>"`.

`alloc_closure` with `f32`/`f64` captures — deferred to LANG38.

Note: float type hints (`f32`, `f64`) and float constant operands **are supported**
for non-closure paths (unlike the BEAM backend).

## Type mapping

| IIR type                              | JVM descriptor | Slot width |
|---------------------------------------|----------------|------------|
| `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `bool` | `I` (int)  | 1 |
| `i64`, `u64`                          | `J` (long)     | 2          |
| `f32`                                 | `F` (float)    | 1          |
| `f64`                                 | `D` (double)   | 2          |
| `void`                                | `V` (void)     | 0          |

A **comparison op** (`cmp_eq`/…/`cmp_ge`) is special-cased to an `int` dest slot
regardless of its `type_hint`: the hint is the *operand* width, but a comparison
always produces a 0/1 `int` (stored with `istore`). Without this, a comparison
over `i64` operands got a `Long` slot, so a later `jmp_if_false` read it with the
long guard (`lload; lconst_0; lcmp`) while it was `istore`d as int → the verifier
rejected an "uninitialized register pair" (LANG-FULL BA-JVM-1, the BASIC `IF`/
`FOR` programs over their i64 value model).

## Register allocation

Variables are allocated to JVM local variable slots via a deterministic
two-pass scan:

1. Function parameters → slots 0..N-1 (in order)
2. All `dest` names and `Var` src operands → next available slot,
   or the existing slot if the name was already seen.

The same variable name always maps to the same slot within one function.
This is a simple linear scan — no liveness analysis, no spilling.

A `mov` whose source and destination slots differ in width **bridges** them with
`i2l`/`l2i`: e.g. a bool/int comparison result moved into a `long` accumulator
(Oct's `&&`/`||` short-circuit over its i64 value model) widens with `i2l` before
`lstore`, so the long slot's second half is initialised — otherwise a later
`lload` of it fails JVM verification ("uninitialized register pair").

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
