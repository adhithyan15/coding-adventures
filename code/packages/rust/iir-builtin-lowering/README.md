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

### Heap builtins: structural vs. runtime-call lowering (LANG77)

Cons cells (`cons`/`car`/`cdr`) have two **target-aware** lowerings:

```rust
use iir_builtin_lowering::{lower_heap_builtins, lower_heap_builtins_runtime};

// Managed backends (wasm/jvm/clr/beam): expand to host GC objects —
// `cons` → `alloc` + 2×`field_store`, `car`/`cdr` → `field_load`.
lower_heap_builtins(&mut module);

// Native backends (twig-aot → aarch64/x86_64): route to the linked C lisp
// runtime — `cons`/`car`/`cdr` → `call_builtin "lispy_cons"/"lispy_car"/
// "lispy_cdr"` → `__twig_lispy_*`, keeping the value NaN-box tagged.
lower_heap_builtins_runtime(&mut module);
```

Both are driven by the *same* frontend IIR, so every lisp-family frontend
reaches both worlds with no language-specific code.

For the native runtime path, follow the rename with **`lower_lisp_repr`** —
a type-directed pass that gives lisp values their NaN-box tag: it boxes the
integer atoms that flow into `lispy_*` calls (`n << 3`), tags the nil
sentinel, and unboxes the program result at the exit boundary. It keys on
use-sites, not the language, so a non-lisp arithmetic program is untouched:

```rust
use iir_builtin_lowering::{lower_heap_builtins_runtime, intern_symbols, lower_lisp_repr};
lower_heap_builtins_runtime(&mut module); // cons/car/cdr + pair?/not/equal? → lispy_*
intern_symbols(&mut module);              // const Var(name):symbol → (id<<32)|TAG_SYMBOL
lower_lisp_repr(&mut module);             // box atoms, tag nil, COND truthiness, unbox result
```

`intern_symbols` assigns each distinct symbol name a module-wide id and emits
the finished tagged immediate, so native `EQ`/`equal?` on symbols is word
equality with no runtime interning or string constants.

`lower_lisp_repr` also normalises `COND`: a `jmp_if_false` whose condition is
a tagged boolean (from `ATOM`/`EQ`) is rewritten to test `lispy_truthy(cond)`
(raw `0`/`1`), and a clause literal funnelled by `mov` alongside the tagged
nil fallthrough is boxed too (a bidirectional `mov` fixpoint) so the funnel
register is uniformly tagged.

For the **managed** backends (wasm/jvm/clr/beam) the twin is
**`lower_lisp_repr_structural`**, which runs after the *structural*
`lower_heap_builtins` instead. Same idea, different value model: a lisp integer
is boxed as a WasmGC `i31ref` (`box`, narrowing the atom to `i32`) rather than
NaN-box-tagged, and the entry function's reference result is unboxed
(`unbox` → `i32`) at the return boundary. It partitions the module with
`concretize_scalar_any_for_wasm` (heap functions vs pure-scalar) so every value
is concretely typed — letting `(CAR (CONS 7 9))` compile to a runnable WasmGC
module (LANG77 / McCarthy L3b-3a-3c).

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
