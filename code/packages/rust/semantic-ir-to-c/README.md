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
the Go/Rust backends); SIR16 `Floats` — a `SIR_FLOAT` `FloatLit` (`7.0`
stays a Float, not the Integer `7`; `Infinity`/`NaN` via `<math.h>`), with
native float arithmetic, the division frontier (Float promotes, two Integers
floor), and IEEE non-finite results; SIR16 `ShortCircuit` — `LogicalAnd`
(`&&`) and `LogicalOr` (`||`) lowered to an `if (_sir_truthy(...))` overwrite
that short-circuits the dead operand and yields the deciding operand (not a
bool); SIR19 `DefaultParams` — a positional default via a `_sir_missing`
sentinel: a `DirectCall` pads omitted trailing arguments and each function opens
with an `if (_sir_is_missing(p)) { p = <default>; }` prologue (a later default
may reference an earlier parameter); and SIR19 `KeywordParams` — a keyword
argument resolved to its callee's parameter slot **by name** at emit time (KW6),
producing a plain positional C call (omitted optional keywords filled with
`_sir_missing()` and substituted by the same default prologue); and SIR17
`Exceptions` — `begin/rescue/ensure` + `raise` lowered to a `setjmp`/`longjmp`
handler stack (a `SIR_ERROR` value, a baked-in exception-class ancestry table
for `rescue`-by-class matching, and a two-handler structure so `ensure` runs
even when a rescue body raises). Rescue-type names are emitted as quoted string
literals (no injection); `retry` is deferred, rejected cleanly. And the OOP
mirror **slice 1** — `Classes` + `Constants`: an empty class
(`class Foo; end` → a comment), construction (`Foo.new` → `_sir_new_instance`, a
new `SIR_INSTANCE` box stored inline in the union that prints `#<Foo>`), and
constants (`PI = 3` / `PI` → a runtime `_sir_const_set` / `_sir_const_get`
table).  Class/constant names are quoted C string literals (no injection).  And
**slice 2** — instance methods: `__def_method__` registers a `(class, method) →
closure` into an explicit table (`_sir_def_method`), and `__method__` dispatches
via `_sir_call_method` (resolve `(recv's class, method)`, apply the closure; miss
→ `NoMethodError`).  Dispatch is an explicit data lookup — **never reflection** on
a source string — so it is anti-RCE by construction; a dispatch to a built-in
method the module never defined routes to the Collections runtime (below) or, if
not lowered yet, is rejected cleanly.  And
**slice 3** — `InstanceVars`: `@v = x` / `@v` (`Scope::Instance`) →
`_sir_ivar_set` / `_sir_ivar_get` on the receiver's lazily-allocated `@name →
value` map (an unset `@v` reads nil), and a bare `self` → `_sir_self()`.  The
receiver is carried across the hoisted method body in `_sir_current_self` (saved
and restored by `_sir_call_method`; an enclosing `begin`/`rescue` restores it on
the unwind path).  The `@`-name is a quoted C string literal (no injection).  And
**slice 4** — inheritance + `super`: `class Dog < Animal` emits
`_sir_register_super("Dog", "Animal")` into a mutable user-ancestry table that
`_sir_class_super` consults **before** the baked-in exception hierarchy (so ONE
`super_of` drives both `rescue`-matching and method resolution); `_sir_call_method`
resolves a method up the ancestry (`_sir_resolve_method`), so a subclass inherits
its parent's methods; and `super` → `_sir_call_super`, resolving the method from
the superclass of the defining class and applying it to the current `self`.  Every
ancestry walk is bounded (`SIR_ANCESTRY_MAX`), so a cyclic hand-built hierarchy
cannot hang.  Class / method / super-class names are all quoted C string literals
(no injection).  And **slice 5** — class methods: `def self.m` →
`_sir_def_class_method` into a SEPARATE class-method (singleton) table (so a class
method and an instance method of the same name never collide), and `Class.m(args)`
→ `_sir_call_class_method`, an ancestry-walking table lookup (class methods
inherit) that binds `self` to nil (no instance receiver).  And **slice 6** —
class variables: `@@x` → `_sir_cvar_get`/`_sir_cvar_set` on a `(class, @@name)`
table shared down the hierarchy (owner resolved via the ancestry), the owning
class taken from `_sir_current_class` (bound by dispatch); a class-body `@@x = 0`
initializer seeds it with the class named explicitly (`_sir_cvar_set_in`).  And
**slice 7** (the final OOP slice) — modules / mixins: a `module` is a name whose
methods register like a class's, `include M` (`_sir_register_include`) folds M's
methods into a class's instance-method resolution and `extend M`
(`_sir_register_extend`) into its class-method resolution — completing the C
OOP surface (**6-backend OOP parity**).

And the **Collections** batch has begun (**slice 1** — String methods): a
`__method__` dispatch to a built-in name the module never defined routes to the
runtime dispatcher `_sir_builtin_method`, which type-checks the receiver and
applies the implementation.  Covered so far: `length`/`size`, `upcase`,
`downcase`, `reverse`, `empty?`, `to_s` (`length`/`size`/`empty?` polymorphic
over String/Array/Hash).  A wrong-type receiver raises `NoMethodError`; a
built-in method not lowered yet is still rejected cleanly.

**Rejects** (cleanly, with a source-positioned error): `TailCalls`,
`Intrinsics`, a `class << self` singleton, and
every other not-yet-wired feature until its batch lands.  `Bignum` stays rejected
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
