# `iir-type-checker`

Optional-typing spectrum checker and inference engine for
[InterpreterIR](../interpreter-ir) modules.

## Overview

The LANG pipeline's `type_hint` field on each `IIRInstr` is *optional*: it
can be `"any"` (unknown, to be profiled) or a concrete type like `"u8"`,
`"i64"`, `"bool"`, `"f64"`, or `"ref<T>"`.

This crate answers two questions:

1. **How typed is this module?** — classifies a module as `Untyped`,
   `Partial(f%)`, or `FullyTyped`.
2. **What types can we infer?** — fills in `"any"` hints for instructions
   whose type is determinable from constants, comparisons, or SSA-propagated
   arithmetic.

## The typing spectrum

```
Untyped ─────────────────────────────────── FullyTyped
(all "any")   Partial(0.3)  Partial(0.9)   (all concrete)

Typical:       TypeScript,   Hack/PHP,      Tetrad, Algol,
MRI Ruby,      Sorbet-Ruby   Dart, Swift    Oct, C
Twig, Lisp
```

| Tier | AOT strategy | JIT strategy |
|------|-------------|--------------|
| `Untyped` | Link full `vm-runtime`; speculate from profile | Wait for profiling |
| `Partial` | Compile typed functions; interpret untyped | JIT hot paths |
| `FullyTyped` | Compile everything natively; skip `vm-runtime` | Specialise immediately |

## Usage

### Check an already-annotated module

```rust
use interpreter_ir::module::IIRModule;
use iir_type_checker::check_module;

let report = check_module(&module);
println!("{}", report.summary()); // e.g. "tier=FullyTyped typed=100% errors=0 warnings=0 inferred=0"
```

### Infer types then check

```rust
use iir_type_checker::infer_and_check;

let report = infer_and_check(&mut module);
// module.functions[0].instructions[n].type_hint is now populated
assert!(report.ok());
```

### Integration with JIT/AOT

```rust
// Before calling jit-core or aot-core, enrich the module:
let report = iir_type_checker::infer_and_check(&mut module);

// jit-core and aot-core read type_hint via IIRInstr::is_typed() /
// effective_type() — they automatically benefit from the inferred hints.
let bytes = aot_core.compile(&module)?;
```

## Inference rules

| Rule | Trigger | Result |
|------|---------|--------|
| R1 const-int | `const dest, Int(n)` with `type_hint="any"` | `"i64"` |
| R2 const-float | `const dest, Float(f)` | `"f64"` |
| R3 const-bool | `const dest, Bool(b)` | `"bool"` |
| R4 cmp-bool | `cmp_* dest, [a, b]` | `"bool"` (always) |
| R5 arith-same | `add/sub/…` where both srcs have the same concrete type | same type |
| R6 bitwise-same | `and/or/xor/shl/shr` with same-typed srcs | same type |
| R7 unary-passthrough | `neg/not dest, [a]` where `a` is typed | same type as `a` |
| R8 ssa-copy | `load_reg dest, [a]` where `a` is typed | same type as `a` |

Inference is multi-pass (fixed-point) so SSA chains of arbitrary depth
resolve correctly.

## Validation errors

| Error | Meaning |
|-------|---------|
| `InvalidType` | `type_hint` is not `"any"`, `"void"`, or a concrete type |
| `TypeMismatch` | Binary op srcs have different concrete types |
| `ConditionNotBool` | `jmp_if_true`/`jmp_if_false` condition is not `"bool"` |

## Stack position

```
iir-type-checker
    └── interpreter-ir    (IIRModule, IIRInstr, opcodes)

Consumers:
    jit-core              (reads type_hint via is_typed() / effective_type())
    aot-core              (same)
```
