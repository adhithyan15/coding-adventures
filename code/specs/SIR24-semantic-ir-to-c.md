# SIR24 — Semantic IR → C (Rust backend)

## Status

Sixth backend for the narrow-waist Semantic IR
([SIR10](SIR10-narrow-waist-semantic-ir.md)).  Joins
[SIR12](SIR12-semantic-ir-to-typescript.md) (TypeScript),
[SIR13](SIR13-semantic-ir-to-rust.md) (Rust),
[SIR14](SIR14-semantic-ir-to-python.md) (Python),
[SIR15](SIR15-semantic-ir-to-go.md) (Go), and
[SIR18](SIR18-semantic-ir-to-javascript.md) (JavaScript).  Implemented
as the Rust crate `semantic-ir-to-c`.

The crate consumes a [`semantic_ir::Module`] and emits a **self-contained
C source file** — an inlined runtime plus the user program, no external
libraries beyond the C standard library.  The output compiles with
`cc <file>.c -o <file>` (any C99 compiler: `gcc`, `clang`, `cc`) and runs.

This spec covers the **v0 core**.  Later feature batches (SIR16 floats /
loops / sequences / maps, default & keyword params, the collection-method
catalog, exceptions, and OOP) land incrementally through the same cascade
the Go backend followed, each recorded in the crate CHANGELOG and proven by
the cross-backend conformance harness ([`sir-conformance`](../packages/rust/sir-conformance/)).
The [§Roadmap](#roadmap-to-parity) section fixes the intended order.

## Why C is the same shape as Go and Rust

C is a **compiled, statically-typed** target with no runtime module system,
exactly like Go and Rust.  So this backend follows the Go/Rust model — emit a
**self-contained artifact with the runtime inlined** — rather than the
Python/TypeScript `sir-runtime-*` import model.  The emitter stays thin (a
`match` on each IR node kind, appending C text); the semantic weight lives in
the inlined C runtime, which ports the Go/Rust `runtime.rs` surface.

Three things C does *not* give for free that Go and Rust did, and how v0
handles each:

| Host gift Go/Rust had | C equivalent (v0) |
|---|---|
| A dynamic value type (`interface{}` / `enum Value`) | a hand-rolled **tagged union** `SirValue` (see [§Value model](#value-model)) |
| Garbage collection / `Rc` | **arena allocation, leak-on-exit** — `malloc` and never free (see [§Memory](#memory-model)) |
| Stack-unwinding exceptions (`panic`/`catch_unwind`) | a **`setjmp`/`longjmp`** handler stack (see [§Exceptions](#exception-model), a later batch) |

## Public API

```rust
use semantic_ir::{Backend, Module, Artifact, BackendError};

pub struct CBackend;

impl CBackend {
    pub fn new() -> Self;
}

impl Backend for CBackend {
    fn target_tag(&self) -> &'static str { "c" }
    fn accepts_features(&self) -> &'static [Feature] { /* ACCEPTED_FEATURES */ }
    fn accepts_intrinsics(&self) -> &'static [&'static str] { &[] }  // v0
    fn compile(&self, module: &Module) -> Result<Artifact, BackendError>;
}

pub fn compile(module: &Module) -> Result<Artifact, BackendError>;
```

`compile` runs, in order: (1) `semantic_ir::validate`, (2) the trait-default
`check_module` (every manifest feature must be accepted; every `Intrinsic`
must be whitelisted and target-tagged), (3) any backend-specific structural
gate (the Go `check_exception_soundness` analogue, added with the OOP batch),
then (4) `emit::emit_module`.

## Capability declaration

**v0 accepts** the SIR-v0 feature set:
`Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
`OptionalTypeAnnotations`, `MutualRecursion`, `Globals`.

**Landed since v0** (`ACCEPTED_FEATURES` grows one version-bumped batch at a
time — see the roadmap): the SIR26 integer conversions (`Conversions`,
`SizedIntegers`, `Unsigned`, `WrappingArithmetic`); and the SIR16 batches
`Loops` + `MutableBindings` (0.3.0–0.5.0), `Sequences` (0.6.0), `Maps`
(0.7.0 — the `SIR_MAP` assoc-array with `MapLit`/`MapGet`/`MapSet`), `Floats`
(0.8.0 — `FloatLit` on the v0 `SIR_FLOAT` tag; emitter-only, the runtime
already carried the float path), `ShortCircuit` (0.9.0 — `LogicalAnd`/
`LogicalOr` lowered to a truthiness-branch overwrite, reusing the eager
`and`/`or` builtin lowering; yields the deciding operand), `DefaultParams`
(SIR19, 0.10.0 — the `SIR_MISSING` sentinel now exists in the runtime; a
`DirectCall` pads omitted trailing defaults with `_sir_missing()` and each
function opens with an `if (_sir_is_missing(p)) { p = <default>; }` prologue),
and `KeywordParams` (SIR19, 0.11.0 — KW6: a `KeywordArg` is resolved to its
callee's parameter slot BY NAME at emit time, using the thread-local signature
map's parameter names, producing a plain positional call).

**Still rejects** (clean, source-positioned `UnsupportedFeature`):
`TailCalls` (C does not guarantee TCO), `Intrinsics` (empty whitelist), and
every not-yet-landed feature (`NDArrays`,
`Exceptions`, `Classes`, … — see the roadmap).  `Bignum` stays
rejected until a bignum runtime ships, so a module that *needs* arbitrary
precision is refused rather than silently truncated.

Accepting a feature and emitting it must stay in lockstep: every accepted
feature has a real, non-panicking emit path; every not-yet-implemented node
kind is unreachable because its feature is unaccepted.

## Value model

C has no dynamic type, so the runtime defines one — a tagged union, the C
analogue of Go's `interface{}` and Rust's `enum Value`:

```c
typedef enum {
    SIR_NIL, SIR_BOOL, SIR_INT, SIR_FLOAT, SIR_STR, SIR_SYM,
    SIR_PAIR, SIR_CLOSURE, SIR_SEQ, SIR_MAP, SIR_INSTANCE,
    SIR_ERROR, SIR_MISSING
} SirTag;

typedef struct SirValue {
    SirTag tag;
    union {
        bool         b;      // SIR_BOOL
        int64_t      i;      // SIR_INT   (v0: fixed 64-bit; see §Integers)
        double       f;      // SIR_FLOAT (Floats batch)
        const char  *s;      // SIR_STR / SIR_SYM (interned)
        SirPair     *pair;   // SIR_PAIR
        SirClosure  *clo;    // SIR_CLOSURE
        SirSeq      *seq;    // SIR_SEQ   (Sequences batch)
        SirMap      *map;    // SIR_MAP   (Maps batch)
        SirInstance *inst;   // SIR_INSTANCE (OOP batch)
        SirError    *err;    // SIR_ERROR (Exceptions batch)
    } as;
} SirValue;
```

- `SirValue` is passed **by value** (16 bytes); reference kinds hold a heap
  pointer, so copying a `SirValue` is a shallow copy — the reference
  semantics Python lists / JS arrays / Ruby objects require.
- Symbols are **interned** (`_sir_intern`): a global table maps a name to one
  canonical `const char *`, so `eq?` on symbols is pointer comparison.
- `SirValue` constructors: `_sir_int(int64_t)`, `_sir_bool(bool)`,
  `_sir_nil()`, `_sir_str(const char*)`, `_sir_sym(const char*)`,
  `_sir_cons(SirValue, SirValue)`, `_sir_make_closure(fn, captures…)`.

The v0 backend, like Go, is **fully dynamic** — every value is a `SirValue`.
Consuming `SirType`/`IntSpec` to emit *native* `int64_t`/`uint32_t` locals
(SIR21, where C benefits most) is a later, additive specialisation and does
not change v0 semantics.

## Memory model

**v0: arena / leak-on-exit.**  Every heap box (`SirPair`, `SirClosure`,
interned string, later `SirSeq`/`SirMap`/`SirInstance`) is `malloc`'d and
**never freed**.  This is correct for the workload: an emitted program is a
batch program — it runs `main`, prints, and exits, at which point the OS
reclaims everything.  It is also the simplest possible model and removes the
single largest source of C bugs (use-after-free, double-free) from the v0
surface.

Reference counting or a conservative (Boehm-style) collector is a **deferred,
optional** concern — relevant only if emitted C is ever meant to run as a
long-lived process, which is out of scope here.  The choice is invisible to
program semantics, so it can change later without touching the emitter.

## Integers

**v0: fixed 64-bit** (`int64_t`), matching the Go (`int64`) and Rust (`i64`)
backends.  Those backends already diverge from Ruby's arbitrary precision for
magnitudes past 2⁶³ (the tracked `10¹² * 10¹²` "bignum frontier" — only the
Python backend, riding native bignums, is exact today).  The C backend adopts
the same floor so it reaches parity with Go/Rust rather than blocking on a
net-new bignum library.

Arbitrary precision is a **later batch**: it accepts `Feature::Bignum`, adds a
minimal bignum to the runtime (or links one), and closes the frontier for C —
tracked alongside the same work for Go/Rust.  Until then `Bignum` is rejected,
never truncated silently.

## Portability contract

The emitted C is **ISO C99** and uses **no compiler-specific extensions** — no
GNU statement-expressions `({…})`, no nested functions, no `typeof`, no
zero-length or variable-length arrays, no compound-literal argument arrays.  It
must compile cleanly and warning-lean on **MSVC (`cl`), GCC, and Clang**, which
the test harness verifies by compiling every emitted program with all three
(see [§Tests](#tests)).  MSVC is invoked in C mode with `/std:c11` (its C99/C11
support); GCC/Clang use their default C mode.  Only ISO facilities are used:
`<stdint.h>` (`int64_t`) and the ordinary `<stdio.h>`/`<stdlib.h>`/`<string.h>`/
`<stdarg.h>` surface.  C99 *mixed declarations* are used (declare-at-use) — a
standard ISO feature all three compilers accept — but the Go/Rust IIFE trick
has no portable-C equivalent, which drives the block lowering below.

## Block-as-expression (portable, no extensions)

A SIR `Block` produces a value (its trailing `value` after its `stmts`), and
`If` is an expression.  Since portable C has no statement-expression, the
emitter is **statement-oriented**: a value is always produced *into a
destination* — either a `return` or an assignment to a named `SirValue`.  Two
mutually-recursive routines do this, and neither needs a temporary or a helper
for the common shapes:

- **`emit_tail(e)`** — emit `e` in return position.
  - simple `e` → `return <expr>;`
  - `If` → a **returning if/else**: `if (_sir_truthy(cond)) { <then-stmts>;
    emit_tail(then-value) } else { <else-stmts>; emit_tail(else-value) }`
  - `Block` → `{ <stmts>; emit_tail(value) }`
- **`emit_assign(dst, e)`** — emit statements that leave `e`'s value in the
  already-declared lvalue `dst`.
  - simple `e` → `dst = <expr>;`
  - `If` → assigning if/else (each branch ends `emit_assign(dst, branch-value)`)
  - `Block` → `{ <stmts>; emit_assign(dst, value) }`
  - a **call with a compound argument** → open a block, `SirValue _aN =` each
    argument in left-to-right order (`emit_assign` into a fresh temp for a
    compound arg, a direct initialiser for a simple one), then
    `dst = fn(_a0, _a1, …);`.  When *every* argument is simple the call is
    emitted inline with no temporaries.

Temporaries are declared at first use (C99 mixed declarations); block-scoped
lets live in a `{ … }` so sibling blocks never collide.  A **trivial** block
(empty `stmts`) emits its `value` inline.  Fresh temp/loop identifiers come
from a per-module counter reset at `emit_module` start (byte-stable output for
the determinism test).

To keep call arguments free of embedded control flow, all variadic builtins are
ordinary **C variadic functions** (`_sir_plus(int n, …)` over `stdarg.h`),
never compound-literal arrays (`(SirValue[]){…}` — unsupported by older MSVC).
So `(+ a b c)` emits `_sir_plus(3, a, b, c)`.

## Per-node lowering rules (v0)

| SIR node | Emitted C |
|---|---|
| `IntLit { value }` | `_sir_int(<value>)` |
| `BoolLit { value }` | `_sir_bool(<true\|false>)` |
| `NilLit` | `_sir_nil()` |
| `SymLit { name }` | `_sir_sym("<name>")` (interned at first use) |
| `StrLit { value }` | `_sir_str("<escaped>")` |
| `VarRef { Local \| Param \| Capture }` | `<sanitised name>` |
| `VarRef { Global }` | `_sir_global_get("<name>")` |
| `VarRef { Builtin }` | `_sir_builtin_closure("<name>")` |
| `If` (expression) | `_sir_if_<N>()` helper, or ternary over `_sir_truthy` when both branches are trivial |
| `Block` (with stmts) | `_sir_block_<N>(<captured locals>)` helper |
| `LetBinding` / `LetStarBinding` | `SirValue <name> = <value>;` |
| `ExprStmt` | `(void)<expr>;` |
| `DirectCall { fn_name, args }` | `<sanitised fn>(<args>)` |
| `IndirectCall { target, args }` | `_sir_apply(<target>, <argc>, <args…>)` |
| `BuiltinCall { name, args }` | `_sir_<helper>(<args>)` (fixed-arity core builtins) |
| `MakeClosure { fn_name, captures }` | `_sir_make_closure(<fn>, <n>, <cap-values…>)` |

Core v0 builtins (each a runtime helper): `+ - * / = < >`, `cons`, `car`,
`cdr`, `null?`, `pair?`, `number?`, `symbol?`, `print`, `global_get`,
`global_set`.  Arithmetic/comparison are **polymorphic** like Ruby's operators
(string/array `+`/`*` overloads and the integer-floor vs float-true division
split arrive with the Floats/collection batches; v0 covers the numeric and
string-concat cases the corpus exercises).

## Module layout of the emitted `.c`

```c
/* <banner: generated by semantic-ir-to-c vX from module <name>> */
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>   /* Exceptions batch */

/* ── inlined SIR runtime ─────────────────────────────── */
/* SirValue, arena alloc, _sir_intern, _sir_truthy, _sir_plus…, _sir_format… */
/* (RUNTIME const; __SIR_DISPLAY_RUBY__ substituted to true/false) */

/* ── globals ─────────────────────────────────────────── */
/* forward decls, then user functions (mutual recursion) */

/* ── user program ────────────────────────────────────── */
static SirValue user_main(void) { … }   /* SIR `main`, renamed */

int main(void) { _sir_runtime_init(); user_main(); return 0; }
```

SIR's `main` is renamed `user_main` so it never collides with C's process
entry `main`.  All user functions get forward declarations before any
definition, so mutual recursion needs no ordering.

## Display convention

The runtime carries the [display-convention](sir-display-convention.md)
switch: `_sir_format` renders a boolean as Ruby `true`/`false` when the
module's `source_language` is Ruby, else the Lisp `#t`/`#f`.  As in Go/Rust,
the emitter substitutes the single placeholder `__SIR_DISPLAY_RUBY__` in the
`RUNTIME` string with the literal `true` or `false`, **selected by a boolean**
— never text derived from any source-controlled field (the standing
anti-injection discipline; the substituted value can never be attacker text).

## Identifier sanitisation

C identifiers match `[A-Za-z_][A-Za-z0-9_]*`.  `sanitize_ident` maps SIR names
into that set (non-matching characters escaped to `_uXXXX_`), and any name
colliding with a C keyword (`is_c_keyword`: `int`, `for`, `return`, `struct`,
…) or a runtime symbol (the `_sir_`/`user_main` namespace) gets a disambiguating
suffix.  String and comment escaping (`quote_c_string`, `sanitize_comment`)
strips embedded terminators, mirroring the SIR12/SIR13 defence, so no source
text can break out of a literal or a comment.

## Exception model

*(Exceptions batch — recorded here so the value model reserves `SIR_ERROR`.)*

C has no stack unwinding, so `raise` / `TryCatch` lower to a **`setjmp` /
`longjmp` handler stack** — the C analogue of Go `panic`/`recover` and Rust
`catch_unwind`:

- a thread-local stack of `jmp_buf`; `TryCatch` pushes one and `setjmp`s.
- `raise` sets a current-error struct `{ const char *sir_class; SirValue msg; }`
  and `longjmp`s to the top handler.
- the handler runs `_sir_rescue_matches(err, class_names[])` over a **baked-in
  ancestry table** (ported from [`sir-runtime-exceptions`](sir-exception-hierarchy.md))
  to pick a matching `rescue` clause or re-`longjmp` to the next handler.
- an `ensure` block runs on both the normal and unwinding paths.
- typed runtime errors ([sir-typed-runtime-errors](sir-typed-runtime-errors.md))
  — `/0`→`ZeroDivisionError`, `.fetch` OOB→`IndexError`, hash `.fetch`
  miss→`KeyError`, unknown method→`NoMethodError` — raise through this path;
  `arr[i]`/`hash[k]` index reads return `nil` (do not raise).

## Security invariants

1. `__SIR_DISPLAY_RUBY__` is replaced by a boolean-selected literal, never by
   source-derived text — no injection into the emitted C.
2. Method dispatch (collection catalog + OOP, later batches) is always an
   **explicit name-switch**, never a function-pointer table keyed on a
   source-derived string — the repo's standing anti-RCE discipline (see
   [sir-collection-methods](sir-collection-methods.md)).  An unknown method
   name is a controlled `NoMethodError`, never a jump to attacker-chosen code.
3. All emitted string/symbol/comment text passes through the escapers above.

## Tests

`cargo test -p semantic-ir-to-c` covers per-node lowering, identifier
sanitisation, deterministic (byte-stable) output, and end-to-end *emit* from
Ruby and Twig source (asserting the emitted C text).  `tests/compile_and_run_*.rs`
compile the emitted C with a discovered `cc`/`clang` and assert stdout — each
**skips gracefully when no C compiler is present** (the `iir-to-llvm`
clang-probe pattern), so Windows CI and toolchain-less sandboxes degrade
rather than fail.  The cross-backend proof is [`sir-conformance`](../packages/rust/sir-conformance/):
a `Target::C` arm compiles-and-runs the emitted C and asserts byte-identical
stdout versus the reference oracle for every corpus program the backend
accepts.

## Roadmap to parity

Order mirrors the Go backend's landed cascade; each item is one version-bumped
PR that grows `ACCEPTED_FEATURES`, the runtime, and the conformance corpus in
lockstep:

1. **v0 core** (this spec) — the SIR-v0 feature set, dynamic `SirValue`, arena
   memory, core builtins, self-contained emission.
2. **SIR16** — `Floats`, `ShortCircuit`, `MutableBindings`, `Loops`,
   `Sequences`, `Maps`.
3. **Params** — `DefaultParams`, `KeywordParams` (the `_sir_missing` sentinel
   + call-time prologue).
4. **Collections** — the `__method__` dispatch catalog
   (String / Array / Hash / Numeric / Symbol / Object, block and non-block).
5. **Exceptions** — `setjmp`/`longjmp` + typed runtime errors.
6. **OOP** — `Classes` / `Constants` / `InstanceVars` / `ClassVars`, then
   `Modules` (mixins / MRO).
7. **Optional / later** — `Bignum`; SIR21 sized-integer native lowering
   (`int64_t`/`uint32_t` from `IntSpec`); `Range` / regex / backtick shims.

## Out of scope (v0)

- Native (unboxed) integers / SIR21 sized-integer lowering — v0 boxes every
  value, like Go.
- Arbitrary-precision integers (`Bignum`).
- Any feature past the SIR-v0 set (deferred to its roadmap batch, rejected
  cleanly until then).
- Freeing memory (arena / leak-on-exit by design).
- Source maps; raw-C intrinsic injection.
