# semantic-ir-to-c

Sixth backend for the narrow-waist [Semantic IR](../semantic-ir/).  Lowers a
`semantic_ir::Module` into **self-contained ISO C99 source code** — every
emitted `.c` file embeds the runtime it needs, so it builds with any C99
compiler and runs, with no dependency beyond the C standard library.

Implements [SIR24](../../../specs/SIR24-semantic-ir-to-c.md).

Because every SIR frontend lowers to the same waist, this one backend gives
**Ruby → C** (the driving goal) and Python / JavaScript / Twig → C for free.

```text
Ruby / Twig / … source
   │  <lang>-to-semantic-ir
   ▼
semantic_ir::Module ──► semantic-ir-to-c ──► self-contained prog.c ──► cc ──► ./prog
```

## Portability — MSVC, GCC, and Clang

The emitted C is **ISO C99 with no compiler-specific extensions** (no GNU
statement-expressions, nested functions, `typeof`, VLAs, or compound-literal
argument arrays).  It compiles on:

- **MSVC** `cl /std:c11`
- **GCC** (default C mode)
- **Clang** (default C mode)

The included `tests/compile_and_run.rs` compiles and runs every corpus program
through a real compiler (see below); the design itself is verified against all
three.

## Usage

```rust
use semantic_ir_to_c::{compile, CBackend};
use semantic_ir::Backend;

let artifact = compile(&sir_module)?;          // convenience
let artifact = CBackend::new().compile(&sir_module)?;  // via the trait
std::fs::write("prog.c", &artifact.source)?;
// $ cc prog.c -o prog && ./prog
```

Dump the C for a snippet during development:

```bash
cargo run -p semantic-ir-to-c --example dump_c -- ruby 'puts 2 + 3 * 4'
cargo run -p semantic-ir-to-c --example dump_c -- twig '(print (+ 2 3))'
```

## How it works

The emitter is **thin**; the semantics live in an inlined C runtime
(`runtime.rs`), the same self-contained model the Go and Rust backends use.

- **Value model** — a tagged union `SirValue` (the C analogue of Go's
  `interface{}` / Rust's `enum Value`): `nil`, `bool`, `int` (`int64_t`),
  `float`, interned `str`/`sym`, `pair`, `closure`.
- **Memory** — arena / leak-on-exit: every box is `malloc`'d and never freed.
  An emitted program is a batch program that runs and exits, so the OS reclaims
  everything; this removes use-after-free / double-free from the surface.
- **Block-as-expression** — portable C has no statement-expression, so the
  emitter is statement-oriented: a value is produced into a `return`
  (`emit_tail`) or an assignment (`emit_assign`); an `if` in tail position
  becomes a returning `if`/`else`; a call with a control-flow argument hoists
  its arguments into temporaries.
- **Variadic builtins** — `(+ a b c)` → `_sir_plus(3, a, b, c)` (real C
  variadic functions, not compound-literal arrays that older MSVC rejects).
- **Closures** — a `MakeClosure` becomes `_sir_make_closure(thunk, ncap, …)`;
  a per-function thunk adapts the body's fixed C signature to the uniform
  closure calling convention; an indirect call is `_sir_apply(...)`.
- **Display convention** — a single `__SIR_DISPLAY_RUBY__` placeholder is
  substituted with a boolean-selected literal (`1` = Ruby `true`/`false`,
  `0` = Lisp `#t`/`#f`) — never source-derived text.

## Capability declaration (v0)

**Accepts** `Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
`OptionalTypeAnnotations`, `MutualRecursion`, `Globals`; the SIR26 integer
conversions (`Conversions`, `SizedIntegers`, `Unsigned`, `WrappingArithmetic`);
SIR16 control flow and mutation (`Loops` — `While`, `ForRange`, `ForEach`; and
`MutableBindings`); SIR16 `Sequences` — a `SIR_SEQ` heap array with
`SeqLit`/`SeqIndex`/`SeqLen`/`SeqSet` and structural equality; and SIR16 `Maps`
— a `SIR_MAP` heap assoc-array with `MapLit`/`MapGet`/`MapSet`, structural
composite keys, positional structural equality, and `{k: v}` display (matching
the Go/Rust backends); and SIR16 `Floats` — a `SIR_FLOAT` `FloatLit` (`7.0`
stays a Float, not the Integer `7`; `Infinity`/`NaN` via `<math.h>`), with
native float arithmetic, the division frontier (Float promotes, two Integers
floor), and IEEE non-finite results.

**Rejects** (cleanly, with a source-positioned error): `TailCalls`,
`Intrinsics`, `NDArrays`, `ShortCircuit`, exceptions/OOP, and every
other not-yet-wired feature until its batch lands.  `Bignum` stays rejected
until a bignum runtime ships — a module needing arbitrary precision is refused,
never silently truncated.

## Roadmap to parity

This crate is the **v0 core**.  Later feature batches land incrementally,
mirroring the Go backend's landed order, each proven by the cross-backend
[`sir-conformance`](../sir-conformance/) harness:

1. v0 core (this release)
2. SIR16 — floats, short-circuit, mutable bindings, loops, sequences, maps
3. default & keyword parameters
4. the collection-method catalog (`String`/`Array`/`Hash`/…)
5. exceptions (`setjmp`/`longjmp`) + typed runtime errors
6. OOP — classes, modules (mixins / MRO)
7. optional — `Bignum`; SIR21 sized-integer native lowering (`int64_t` /
   `uint32_t` from the IR's `IntSpec`)

## Testing

```bash
cargo test -p semantic-ir-to-c
```

- `tests/emit.rs` — asserts the *text* of the emitted C (shape, determinism,
  identifier sanitisation, capability rejection).  Runs with no C compiler.
- `tests/compile_and_run.rs` — **compiles and runs** each corpus program and
  asserts stdout.  It finds a compiler from `SIR_CC` (an absolute path works),
  then `cc`/`clang`/`gcc` on `PATH`; if none is present it **skips** rather than
  failing.  Point it at a specific compiler with, e.g.:

  ```bash
  SIR_CC=clang cargo test -p semantic-ir-to-c
  ```
