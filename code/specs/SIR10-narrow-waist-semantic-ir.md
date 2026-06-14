# SIR10 — Narrow-Waist Semantic IR (Rust)

## Status

This spec is a **re-design** of the Semantic IR concept introduced in
[`SIR00-semantic-ir.md`](SIR00-semantic-ir.md).  SIR00 grew an
incompatible set of design choices over time (per-language extension
bags, an `INFERRED` emitter mode that performs loose→strict type
inference, a large per-language node taxonomy).  This spec replaces
those choices with the **narrow-waist** discipline described below;
the two designs are deliberately not compatible.  The existing Python
`semantic-ir` package implementing SIR00 remains in place pending a
decision to deprecate.

The Rust implementation of this spec lives in three crates:

- [`semantic-ir`](../packages/rust/semantic-ir/) — the IR itself,
  validator, walker, text format, backend interface.
- [`twig-to-semantic-ir`](../packages/rust/twig-to-semantic-ir/) — the
  first frontend, consuming `twig-parser::Program` and producing a
  `semantic_ir::Module`.
- [`semantic-ir-to-typescript`](../packages/rust/semantic-ir-to-typescript/) —
  the first backend, consuming `semantic_ir::Module` and emitting
  self-contained TypeScript source code with an inlined runtime.

## Motivation

The Semantic IR (SIR) is a neutral intermediate representation that
sits between language frontends and code-emitting backends.  Without
it, every pair of N source languages and M target languages requires
its own translator — N × M.  With it, every frontend lowers once into
SIR and every backend consumes SIR — N + M total implementations.

This is the **hourglass** or **narrow-waist** architecture used by
LLVM, GCC, MLIR, Pandoc, and the protobuf wire format.  It works for
the same reason in all of them: when the waist has *no opinions*
about either end, it stays small, stable, and easy to reason over.

> **The single most important design rule:** disambiguation is the
> frontend's job.  Every semantic concept in the SIR is a distinct,
> named node with typed operands.  There is never a case where a
> backend has to ask "what did the programmer mean here?"

## Direction: strict → loose only

The SIR is designed exclusively for translating from a source
language with *more* semantic information to a target language with
*less*.  The reverse direction (loose → strict) requires inventing
information that was never there — type inference, ownership
inference, "what did the programmer mean by `x.append(y)`" — and is
explicitly out of scope.

In practice: when SIR carries a piece of semantic information (a
type, an ownership marker, an effect tag), backends that don't need
it may discard it; backends that *contradict* it must reject.
Frontends that can't supply it leave the slot absent and backends
decide what to do with the absence.

## Three roles

### Frontend

A frontend understands one source language.  Its job is to lower
source code into SIR, committing to one semantic interpretation per
construct.  In particular:

- The frontend disambiguates every operation that the source language
  leaves contextual.  `let y = x;` in Rust becomes a `Move` node; in
  Python it would become a `ShareRef` node; in Lua a `Copy` node —
  the frontend knows the language semantics and picks.
- The frontend produces a **module-level feature manifest** listing
  which SIR features the module's body uses.  This is a fast-reject
  mechanism for backends.
- The frontend may attach metadata to nodes (`source_language`,
  `source_version`, `origin_symbol`).  Metadata is **advisory** —
  the IR's correctness must not depend on it.  Strip all metadata
  and the program must still mean exactly the same thing.

### The SIR itself

The SIR is opinionated about its *own* semantics.  It defines exactly
what `MakeClosure`, `DirectCall`, `IndirectCall`, `BuiltinCall`,
`Intrinsic`, `LetBinding`, `LetStarBinding`, etc. mean.  It is **not**
opinionated about how those concepts appear in any source language
or how they're lowered in any target language.

The SIR has no concept of "Python's `list`" or "Rust's `Vec<T>`".  It
has primitive concepts (in v0: `Pair`, `Symbol`, `Str`, `Int`, `Bool`,
`Nil`, `Closure`).  A frontend lowering Python's `list` would commit
to a sequence concept (added in a later SIR version); a backend
emitting Python would render that concept as `list`.  The IR is
unaware of both decisions.

### Backend

A backend understands one target language.  Its job is to consume SIR
modules and either:

- Emit valid source code (or bytecode, or AST, or whatever the
  target format is) that preserves the SIR's semantics; **or**
- **Reject** the module cleanly with a source-positioned error
  pointing at the feature or node it cannot support.

Each backend publishes a **capability declaration** — the explicit
list of SIR features and intrinsic names it accepts.  Backends never
silently emit wrong code.  When a backend can express a feature only
via a polyfill or library import, it accepts that node but emits the
appropriate import alongside it.

## Module structure

```text
Module {
    name:      String,             // module identifier
    manifest:  FeatureManifest,    // declared features (see below)
    imports:   Vec<Import>,        // other SIR modules referenced
    exports:   Vec<ExportName>,    // names visible to other modules
    functions: Vec<Function>,      // including synthesised _init / main
    globals:   Vec<Global>,        // top-level value bindings
    metadata:  Metadata,           // source language, version, sir version
    span:      Span,               // source position of the module form
}
```

### Imports and exports

```text
Import {
    module_path: String,             // e.g. "compiler/lexer"
    names:       Vec<ImportName>,   // explicit names (no wildcards)
    span:        Span,
}

ImportName {
    source_name: String,             // name in the exporting module
    local_name:  String,             // name in this module
}

ExportName {
    name: String,                    // function or global name
    span: Span,
}
```

### Globals

```text
Global {
    name:          String,
    sir_type:      Option<SirType>,
    init_function: String,           // synthesised _init function name
    span:          Span,
}
```

Top-level value bindings (`(define x expr)` in Twig) become globals.
The actual initialization expression lives in a synthesised `_init`
function that runs before `main`.  This keeps function bodies pure
expressions and isolates initialization ordering.

## Functions

```text
Function {
    name:        String,
    params:      Vec<Param>,
    return_type: Option<SirType>,    // None = unspecified / dynamic
    captures:    Vec<Capture>,       // empty for top-level functions
    body:        Block,
    effects:     EffectSet,
    metadata:    Metadata,
    span:        Span,
}

Param   { name: String, sir_type: Option<SirType>, span: Span }
Capture { name: String, sir_type: Option<SirType> }
```

A function with non-empty `captures` is a closure body.  Backends
that do not support closures (the `Closures` feature absent from
their capability list) reject any module whose manifest declares
`Closures`.

## Blocks and statements

A `Block` is the body shape used by every compound form:

```text
Block {
    stmts: Vec<Stmt>,
    value: Expr,                     // a Block always produces a value
    span:  Span,
}

Stmt =
    | LetBinding     { name, sir_type?, value, span }
    | LetStarBinding { name, sir_type?, value, span }
    | ExprStmt       { expr, span }
```

`LetBinding` semantics: the RHS is evaluated in the scope **outside**
the let group.  Multiple `LetBinding`s in a row share that property —
they may be evaluated in parallel.

`LetStarBinding` semantics: the RHS is evaluated in the scope
including all **prior** `LetStarBinding`s in the same group.  Order
is load-bearing.

A frontend that wants Scheme `let` emits a run of `LetBinding`s; one
that wants `let*` emits `LetStarBinding`s.  The IR commits to one or
the other at lowering time — never both ambiguously.

## Expressions

Every distinct semantic operation is a distinct node kind.

### Atomic literals

```text
IntLit  { value: i64,    span }
BoolLit { value: bool,   span }
NilLit  {                span }
SymLit  { name: String,  span }
StrLit  { value: String, span }
```

### Variable reference

```text
VarRef {
    name:  String,
    scope: Scope,                    // committed at lowering time
    span:  Span,
}

Scope =
    | Local      // bound by let / let*
    | Param      // function parameter
    | Capture    // captured from enclosing scope (closure)
    | Global     // top-level define
    | Builtin    // language built-in
```

The frontend commits to a scope tag at lowering time.  Backends do
not re-resolve names.

### Conditional

```text
If {
    cond:        Box<Expr>,
    then_branch: Box<Block>,
    else_branch: Box<Block>,
    span:        Span,
}
```

Always ternary.  Both branches are blocks (every branch must produce
a value).  Truthiness rules are the frontend's responsibility — the
`cond` expression must evaluate to a boolean SIR value, and the
frontend inserts any coercions.

### Calls (three distinct kinds)

```text
DirectCall {
    fn_name: String,                 // known top-level function name
    args:    Vec<Expr>,
    effects: EffectSet,
    span:    Span,
}

IndirectCall {
    target:  Box<Expr>,              // value position (closure handle)
    args:    Vec<Expr>,
    effects: EffectSet,
    span:    Span,
}

BuiltinCall {
    name:    String,                 // builtin name (e.g. "+", "cons")
    args:    Vec<Expr>,
    effects: EffectSet,
    span:    Span,
}
```

The three call shapes correspond to: a static call to a known
top-level function (the fast path), a dynamic dispatch through a
value that holds a closure handle, and a call to a language builtin
(which backends typically lower to primitive ops or runtime helpers).

A frontend that doesn't know whether a call site is direct or
indirect **must** commit to `IndirectCall`.  The IR has no "maybe
direct" node.

### Closure construction

```text
MakeClosure {
    fn_name:  String,                // top-level Function this closure runs
    captures: Vec<CaptureValue>,
    span:     Span,
}

CaptureValue {
    name:  String,                   // capture name in the called Function
    value: Expr,                     // value provided at construction site
}
```

A `MakeClosure` produces a closure handle.  The handle is opaque to
the IR — its representation is the backend's choice (JS function
object, Python lambda, etc.).  An `IndirectCall` whose target
evaluates to a closure handle is the natural use site.

### Intrinsic — the escape hatch

```text
Intrinsic {
    targets:     Vec<String>,        // non-empty target tag set
    name:        String,             // intrinsic identifier
    args:        Vec<Expr>,
    return_type: SirType,
    effects:     EffectSet,
    span:        Span,
}
```

The intrinsic is an admission that the operation has no portable SIR
representation — e.g. inline assembly, a DOM API call, FFI to a
specific runtime.  Constraints (see "Escape hatch discipline" below)
are non-negotiable.

## Types — a carrier, not a verifier

SIR carries type information but does not infer or verify it.  A
frontend either supplies a `SirType` or supplies `None`; SIR
roundtrips that decision faithfully.

```text
SirType =
    | Any                            // top type
    | Int                            // 64-bit signed integer
    | Bool
    | Nil
    | Symbol
    | Str
    | Pair                           // cons cell
    | Closure                        // any closure handle
    | Fn { params: Vec<SirType>,
           ret:    Box<SirType> }    // function type
```

`SirType` is intentionally small.  Twig is dynamically typed so the
common case is `Any` or `None` everywhere.  Future frontends (typed
TW05, Rust, Haskell) will extend this enum — that's a versioned
change to the SIR.

## Effects

Every call node carries an `EffectSet`.  Frontends annotate; backends
may use or ignore.  v0 effect tags:

```text
EffectSet (bitset) =
    | Pure          // empty set: no observable effects
    | MayThrow
    | MayPrint
    | MayAllocate
    | MayBlock      // I/O wait
    | Divergent     // may not terminate
```

Effects compose naturally — a `BuiltinCall("print", ...)` carries
`MayPrint | MayAllocate`; a `BuiltinCall("+", ...)` carries `Pure`.

## Feature manifest

Every module's manifest lists which features its body uses.  Backends
check the manifest in O(1) before traversing the body.  v0 features:

```text
Feature =
    | Closures
    | Pairs
    | Symbols
    | Strings
    | DynamicTyping
    | OptionalTypeAnnotations
    | MutualRecursion
    | TailCalls
    | Globals
    | Intrinsics
```

A frontend that omits a feature its body uses produces a validator
error.  Backends fast-reject when the manifest declares a feature
they do not list in their capability declaration.

## Escape hatch discipline

The intrinsic escape hatch exists because some operations cannot be
expressed in portable SIR semantics (inline assembly is the
canonical example).  Without strict constraints the escape hatch
becomes load-bearing and the IR becomes a thin wrapper around
target-specific blobs.

**Hard rules — violations are validator errors:**

1. **Whitelist-only acceptance.**  Backends declare exactly which
   intrinsic names they accept.  Default is reject.  An intrinsic
   whose name is not in the list fails compilation with a clear
   error pointing at the source location.

2. **Non-empty target tag set.**  `Intrinsic.targets` must contain
   at least one target identifier.  A bare intrinsic with no tag is
   a validator error.

3. **Transparent type signature.**  The intrinsic carries a typed
   `return_type` and typed `args`.  The body is opaque; the boundary
   is not.

4. **No control flow across the boundary.**  An intrinsic is a leaf:
   takes typed inputs, produces a typed output, returns to its
   caller.  No goto, no break out of enclosing loop, no longjmp.

5. **Manifest declaration mandatory.**  Any module using an
   intrinsic must declare `Feature::Intrinsics` in its manifest.

6. **No intrinsic composition pretending to be normal IR.**  Each
   intrinsic stands alone; its result is a regular typed value.

**Rule of thumb:** "An intrinsic is an admission that a feature is
fundamentally undefinable in the IR's semantic model, not a shortcut
for something I didn't get around to modeling yet."  If reaching for
an intrinsic, first ask whether the feature should be added to the
IR's core instead.

## Source positions

Every node carries a `Span`:

```text
Span {
    file:       String,            // logical filename ("<inline>" if in-memory)
    start_line: usize,             // 1-indexed
    start_col:  usize,             // 1-indexed
    end_line:   usize,             // 1-indexed
    end_col:    usize,             // 1-indexed
}
```

## Textual surface syntax

The SIR has a human-readable S-expression textual form, analogous to
LLVM's `.ll` and WebAssembly's `.wat`.  Round-tripping (text → SIR →
text) is required to produce byte-identical output up to whitespace
normalization.  This is essential for debugging and golden tests.

A small example:

```text
(sir-module compiler/example v0
  (manifest closures pairs symbols dynamic-typing globals)
  (metadata (source-language twig) (source-version 0.7))

  (global counter)

  (function _init () (effects pure) (block
    (stmt (expr (builtin-call global_set (effects pure)
                  (sym counter) (int 0))))
    (nil)))

  (function add ((n any) (m any)) any (effects pure) (block
    (builtin-call + (effects pure)
      (var-ref n param) (var-ref m param))))

  (function main () any (effects may-print) (block
    (direct-call add (effects pure) (int 1) (int 2)))))
```

The grammar is intentionally close to s-expression Lisp; every node
kind has a head keyword that names it.

## Validation

The `semantic-ir` crate exposes:

```rust
pub fn validate(module: &Module) -> Result<(), Vec<ValidatorError>>;
```

The validator checks:

- Manifest covers every feature actually used in the body
- No `VarRef` references an undefined name in its scope
- Every `Block` ends with a value expression
- Every `Intrinsic.targets` is non-empty
- Function names are unique within a module
- Effects propagation is internally consistent (best-effort)
- Manifest features declared-but-not-used produce **warnings**
  (frontends may over-declare conservatively)

## Backend interface

Backends implement the `Backend` trait:

```rust
pub trait Backend {
    /// Target identifier (e.g. "typescript", "python", "rust").
    fn target_tag(&self) -> &'static str;

    /// Features this backend accepts.  Modules declaring any feature
    /// outside this set are rejected.
    fn accepts_features(&self) -> &'static [Feature];

    /// Intrinsics this backend accepts by name.  Whitelist.
    fn accepts_intrinsics(&self) -> &'static [&'static str];

    /// Compile a SIR module to a target artifact.
    fn compile(&self, module: &Module) -> Result<Artifact, BackendError>;
}

pub struct Artifact {
    pub filename: String,
    pub source:   String,
    pub metadata: ArtifactMetadata,
}
```

A `BackendRegistry` allows runtime discovery of registered backends.

## Versioning

The SIR is versioned at the spec level.  v0 (this spec) is the
initial cut.  Adding a new node kind, feature, or `SirType` variant
is a v.bump.  Renames and removals are prohibited within a major
version.

Modules carry a `sir_version: String` field in their metadata.
Backends and validators check it on entry; a v0 backend refuses a
v1 module.

## What this spec is not

- **Not an optimizing IR.**  SIR carries semantic info but does not
  prescribe an evaluation strategy.  Optimization passes (if any
  ever exist) consume SIR and emit SIR in separate crates.
- **Not an executable representation.**  SIR has no register file,
  heap, or program counter.  Backends emit executable code.
- **Not a type checker.**  SIR carries optional types.  Type
  checking is the frontend's job, or a separate verifier crate.
- **Not Twig-shaped.**  Twig is the first frontend; the design is
  driven by narrow-waist principles, not Twig surface semantics.

## v0 scope summary

What v0 covers (in the Rust implementation):

- Modules with manifest, imports, exports, metadata
- Functions with params, return types, captures, effects
- Atomic literals (int, bool, nil, symbol, string)
- VarRef with explicit scope tags
- If, Block, LetBinding, LetStarBinding, ExprStmt
- DirectCall, IndirectCall, BuiltinCall
- MakeClosure
- Intrinsic with escape-hatch discipline
- SirType (Any, Int, Bool, Nil, Symbol, Str, Pair, Closure, Fn)
- EffectSet bitset
- FeatureManifest
- Textual form (parser + printer)
- Validator
- Backend trait + registry

What v0 defers to later versions:

- Ownership / borrow markers (Move / Copy / Borrow)
- Async / await / coroutines
- Exception handling (Raise / Try / Catch)
- Pattern matching (Match)
- Records / unions / type aliases
- Effect inference (manual annotation only)
- The stdlib primitive set beyond Twig needs (`Seq`, `Map`, `Set`,
  `Option`, `Result`)
- Refinement types
