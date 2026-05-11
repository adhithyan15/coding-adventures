# iir-builtin-lowering

A Rust crate that implements Phase 1 of the LANG31 builtin lowering pass: it
transforms `call_builtin` instructions for arithmetic and comparison operations
into the typed IIR opcodes that the `iir-to-*` backends can lower directly to
target bytecode (BEAM, WASM, JVM class file, CLR CIL).

## What is it?

The Twig language frontend (`twig-ir-compiler`) emits dynamically typed IR.
Every arithmetic or comparison operation becomes a generic `call_builtin`
instruction:

```
%r0 = call_builtin("+", %a, %b) : any
```

After `iir-type-checker` promotes the `"any"` type hints to concrete types,
this pass rewrites each recognised `call_builtin` into the typed IIR opcode:

```
%r0 = add(%a, %b) : i64
```

The backends (`iir-to-beam`, `iir-to-wasm`, `iir-to-jvm-class-file`,
`iir-to-cil-bytecode`) then handle the typed ops natively — e.g. `add` on
`i64` becomes `i64.add` in WASM or `gc_bif2 '+'/2` in BEAM.

## Pipeline position

```
twig-ir-compiler  →  iir-type-checker  →  iir-builtin-lowering  →  iir-to-<target>
```

**This pass MUST run after `iir-type-checker`.**  Running it before leads to
`UntypedBuiltin` errors, which is an intentional hard failure to surface
pipeline ordering bugs.

## Lowering table (Phase 1 — 18 numeric builtins)

| Builtin name | Arity | Replacement op |
|:-------------|:-----:|:---------------|
| `"+"`        | 2     | `"add"`        |
| `"-"`        | 2     | `"sub"`        |
| `"*"`        | 2     | `"mul"`        |
| `"/"`        | 2     | `"div"`        |
| `"%"`        | 2     | `"mod"`        |
| `"neg"`      | 1     | `"neg"`        |
| `"="`        | 2     | `"cmp_eq"`     |
| `"!="`       | 2     | `"cmp_ne"`     |
| `"<"`        | 2     | `"cmp_lt"`     |
| `"<="`       | 2     | `"cmp_le"`     |
| `">"`        | 2     | `"cmp_gt"`     |
| `">="`       | 2     | `"cmp_ge"`     |
| `"and"`      | 2     | `"and"`        |
| `"or"`       | 2     | `"or"`         |
| `"not"`      | 1     | `"not"`        |
| `"shl"`      | 2     | `"shl"`        |
| `"shr"`      | 2     | `"shr"`        |
| `"xor"`      | 2     | `"xor"`        |

Unknown builtins (`"cons"`, `"make_closure"`, `"global_set"`, etc.) are left
completely unchanged so later passes or backends can handle them.

## API

```rust
use iir_builtin_lowering::{lower_builtins, lower_builtins_cloned,
                            lower_builtins_checked, BuiltinLoweringError};

// Mutating form — most efficient.
let errors: Vec<BuiltinLoweringError> = lower_builtins(&mut module);

// Cloning form — original is preserved.
let (lowered, errors) = lower_builtins_cloned(&module);

// Checked form — returns Err if any errors occur.
lower_builtins_checked(&mut module)?;
```

### Error types

```rust
pub enum BuiltinLoweringError {
    /// Builtin called with wrong number of arguments.
    WrongArity { builtin_name, function_name, expected, found },
    /// Builtin still has type_hint = "any" — pipeline ordering bug.
    UntypedBuiltin { builtin_name, function_name },
}
```

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
use iir_builtin_lowering::lower_builtins;

// Assume `module` was produced by twig-ir-compiler and then iir-type-checker.
let errors = lower_builtins(&mut module);
assert!(errors.is_empty());
```

## Source layout

```
src/
  lib.rs      — public API
  numeric.rs  — 18-entry lowering table and per-instruction rewrite logic
  error.rs    — BuiltinLoweringError enum
  lower.rs    — legacy simple lowering pass (kept for historical reference)
tests/
  test_lowering.rs  — 50 integration tests
```

## Relationship to the LANG31 spec

This crate implements §1.1 of LANG31.  Phase 2 (`"cons"`, `"car"`, `"cdr"`,
`"null?"`, `"pair?"`) will be added in a future commit as `src/heap.rs`.

## Running tests

```bash
cargo test -p iir-builtin-lowering
```
