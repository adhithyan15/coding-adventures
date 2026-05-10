# TW05 - self-hosted, statically typed Twig compiler

## Overview

TW04 added modules partly because a real Twig compiler is too large to live in
one file.  TW05 defines the next step: a Twig compiler written in Twig.

The self-hosted compiler is not just a rewrite of the dynamic compiler.  It is
the first large production user of typed Twig:

- every compiler module is statically typed;
- public compiler APIs use refinement types;
- proof obligations are discharged by the repo's `constraint-vm` stack;
- compiler bugs such as out-of-bounds token access, invalid spans, wrong arity,
  unresolved labels, and invalid register ids become compile-time errors where
  the solver can prove them.

The compiler's output is InterpreterIR.  Native, JVM, CLR, WASM, JIT, and AOT
remain shared downstream backends.

```text
typed Twig compiler source
        |
        v
bootstrap Twig compiler (Rust/Python host implementation)
        |
        v
self-hosted compiler as IIR/native executable
        |
        v
Twig source -> InterpreterIR -> LANG VM / JIT / AOT / JVM / CLR / WASM
```

## Goals

- Write the Twig lexer, parser, resolver, type checker, refinement checker
  adapter, IIR emitter, optimizer, sidecar emitter, and driver in Twig.
- Make typed Twig strong enough to express the compiler without falling back to
  untyped `any` at module boundaries.
- Use `lang-refined-types`, `lang-refinement-checker`, and `constraint-vm` for
  real proof obligations.
- Keep the compiler target-independent by emitting InterpreterIR.
- Reach a fixed point: the compiler compiled by the host compiler and the
  compiler compiled by itself produce equivalent output.

## Non-goals

- Rewriting `constraint-vm` in Twig as part of this milestone.
- Making all user Twig code statically typed.  Dynamic Twig remains supported.
- Replacing the native backend work from LANG25.
- Adding macros before the compiler is self-hosted.
- Full dependent types.  Refinements are constrained to decidable predicates the
  solver can handle, with opaque predicates treated as unknown.

## Relationship to existing specs

| Spec | Relationship |
|------|--------------|
| TW00 | Defines the dynamic Twig surface and IIR lowering. |
| TW04 | Adds modules, imports, exports, host package, and stdlib-in-Twig support. |
| LANG23 | Defines refinement types and the three checker outcomes. |
| LANG24 | Defines `constraint-vm`, the solver substrate used by refinement checking. |
| LANG25 | Defines the native AOT/JIT/debugger completion plan that typed Twig will reuse. |
| LANG26 | Defines the shared Rust stdlib, including source-span and diagnostic helpers the self-hosted compiler can reuse. |

## Language additions

TW05 introduces a typed Twig dialect.  The dialect is enabled per module:

```scheme
(module compiler/lexer
  (typed strict)
  (export lex)
  (import compiler/token compiler/span))
```

Modes:

- `(typed off)` - current dynamic Twig behavior.
- `(typed lenient)` - type and refinement annotations are checked; unknown
  refinement outcomes become runtime checks.
- `(typed strict)` - no public `any`, unknown refinements are compile errors,
  and every exported function has typed parameters and return type.

Compiler modules must use `(typed strict)` once bootstrapping reaches stage 3.

### Function annotations

```scheme
(define (ascii-info (i : (Int 0 128)) -> TokenInfo)
  ...)

(define (emit-load
          (reg : (RegId frame-size))
          (frame-size : Nat)
          (out : IirBuilder)
        -> IirBuilder)
  ...)
```

Parameter syntax is `(name : Type)`.  Return syntax is `-> Type` before the
body.  Unannotated parameters in strict mode are compile errors.

### Local annotations

```scheme
(define start : (Index source-len) pos)
(let (((next : (Index source-len)) (+ pos 1)))
  ...)
```

Local annotations are optional in lenient mode and required only when inference
cannot prove a useful type.  The compiler should prefer inference inside a
function and explicit annotations at module boundaries.

### Type aliases

```scheme
(type Nat       (Int 0 _))
(type Byte      (Int 0 256))
(type CharCode  (Int 0 1114112))
(type RegId     (fn (frame-size) (Int 0 frame-size)))
(type Index     (fn (len) (Int 0 len)))
```

Type aliases are compile-time only.  They expand before IIR emission.

### Records

The compiler needs structured data.  TW05 adds typed records:

```scheme
(record Span
  (source-id : SourceId)
  (start     : (Index source-len))
  (end       : (Index source-len)))

(record Token
  (kind   : TokenKind)
  (lexeme : String)
  (span   : Span))
```

Records lower to the existing heap/runtime representation.  Field access lowers
to typed property operations where available, or runtime calls in early stages.

### Tagged unions

The compiler needs AST and type nodes.  TW05 adds tagged unions:

```scheme
(union Expr
  (IntLit    (value : Int) (span : Span))
  (BoolLit   (value : Bool) (span : Span))
  (NameRef   (name : Symbol) (span : Span))
  (IfExpr    (cond : Expr) (then : Expr) (else : Expr) (span : Span))
  (CallExpr  (callee : Expr) (args : (List Expr)) (span : Span)))
```

Pattern matching can be minimal in v1:

```scheme
(match expr
  ((IntLit value span) ...)
  ((NameRef name span) ...)
  (_ ...))
```

The first self-hosted compiler may lower `match` to nested tag checks.

## Refinement syntax

Refinement types use the existing LANG23 vocabulary with Twig-friendly syntax.

### Ranges

```scheme
(Int 0 128)      ; 0 <= x < 128
(Int 1 _)        ; x >= 1
(Int _ 0)        ; x < 0
Nat              ; alias for (Int 0 _)
Byte             ; alias for (Int 0 256)
```

### Membership

```scheme
(Member TokenKind LParen RParen Identifier Integer EOF)
```

### Named predicates

```scheme
(where Int
  (and (<= 0 x)
       (< x source-len)))
```

`x` is the value being checked.  Other names must be in scope as immutable
parameters or locals with known refinements.

### Built-in compiler refinements

The compiler should ship aliases for common compiler invariants:

```scheme
(type SourceId      Nat)
(type TokenIndex    (fn (token-count) (Int 0 token-count)))
(type NonEmptyList  (fn (T) (List T where (not (null? x)))))
(type InstrIndex    (fn (instr-count) (Int 0 instr-count)))
(type LabelId       Nat)
(type FrameSlot     (fn (frame-size) (Int 0 frame-size)))
```

## Constraint solver integration

The typed Twig checker lowers annotations to:

- `lang-refined-types::RefinedType`
- `lang-refinement-checker` obligations
- `constraint-vm` programs

The checker uses the LANG23 outcome model:

| Outcome | Compiler action in lenient mode | Compiler action in strict mode |
|---------|---------------------------------|--------------------------------|
| ProvenSafe | Strip runtime check and narrow downstream type. | Same. |
| ProvenUnsafe | Compile error with counter-example. | Same. |
| Unknown | Emit runtime check and warning. | Compile error. |

Compiler modules use strict mode at public boundaries.  During early bootstrap,
internal modules may temporarily use lenient mode while the type checker grows.

## Flow-sensitive narrowing

The type checker must learn from guards:

```scheme
(define (byte? (n : Int) -> Bool)
  (and (<= 0 n) (< n 256)))

(define (write-byte-safe (n : Int) -> Int)
  (if (byte? n)
      (host/write-byte n)   ; n narrowed to Byte
      -1))
```

Required guard forms for v1:

- `<`, `<=`, `>`, `>=`, `=`
- `and`, `or`, `not`
- predicates that have a declared refinement effect, such as `byte?`
- union tag checks generated by `match`

## Compiler invariants to prove

The self-hosted compiler should deliberately use refinements where compiler bugs
usually hide.

### Lexer

Invariant examples:

- cursor position is always `(Index source-len)`;
- `peek` only reads when `pos < source-len`;
- produced token spans satisfy `0 <= start <= end <= source-len`;
- integer literal scanners consume at least one digit;
- string scanners either return a closed string token or a precise unterminated
  string diagnostic.

Example:

```scheme
(define (peek (src : Source) (pos : (Index (source-len src)))
        -> (Option Char))
  ...)
```

### Parser

Invariant examples:

- token cursor is always `(TokenIndex token-count)`;
- successful parse returns `next > start`;
- error spans are valid source spans;
- every AST node carries a valid span;
- parser recursion consumes input or explicitly reports an error.

Example:

```scheme
(record ParseOk
  (expr : Expr)
  (next : (where Int (and (< start x) (<= x token-count)))))
```

### Resolver

Invariant examples:

- every `NameRef` resolves to exactly one binding or produces a diagnostic;
- imported names must be exported by the imported module;
- no duplicate exported names;
- no dependency cycles unless a later spec permits them.

### Type checker

Invariant examples:

- every public function in strict mode has refined parameter types;
- every call arity matches the callee signature;
- every branch narrows path facts consistently;
- every `match` is exhaustive or has `_`;
- no unresolved type alias reaches IIR emission.

### IIR emitter

Invariant examples:

- every register read has a dominating write;
- every register id is `< frame-size`;
- every function call target exists;
- every builtin name is known to the selected `LangBinding`;
- every jump target is a valid instruction index;
- every generated sidecar source span is valid.

Example:

```scheme
(define (emit-jump
          (target : (InstrIndex instr-count))
          (builder : IirBuilder)
        -> IirBuilder)
  ...)
```

## Compiler module layout

The self-hosted compiler source should live as Twig modules:

```text
code/twig/compiler/
  compiler/
    token.tw
    span.tw
    diagnostic.tw
    lexer.tw
    ast.tw
    parser.tw
    module_resolver.tw
    types.tw
    type_parser.tw
    type_env.tw
    flow.tw
    refinement.tw
    constraint_lowering.tw
    iir.tw
    emit_iir.tw
    sidecar.tw
    optimizer.tw
    driver.tw
```

Bootstrap host crates should live in Rust until the fixed point is reached:

- `twig-type-checker`
- `twig-refinement-lowering`
- `twig-self-host-bootstrap`
- `twig-self-host-driver`

Once the Twig implementation is complete, the Rust driver remains as a thin
launcher: parse CLI flags, load files, call the compiled Twig compiler, write
artifacts.

## Bootstrap stages

### TW05-A - typed syntax accepted

- Extend Twig parser for `(typed ...)`, parameter annotations, return types,
  local annotations, aliases, records, unions, and match.
- Preserve annotations in the AST.
- Lower typed programs exactly like dynamic programs by erasing annotations.
- Add golden parser tests.

Acceptance:

- typed syntax round-trips through parse/format;
- annotation-free Twig still parses unchanged;
- annotated programs run after erasure.

### TW05-B - base static type checker

- Add `twig-type-checker`.
- Check base kinds: Int, Bool, Nil, Symbol, String, List, Record, Union,
  Function, Any.
- Check call arity, module imports/exports, record fields, union constructors,
  and match exhaustiveness.
- Produce typed AST.

Acceptance:

- type errors include source spans;
- compiler modules can type-check in lenient mode without refinements.

### TW05-C - refinement checker bridge

- Parse range, membership, and `where` refinements.
- Lower Twig refinements to `lang-refined-types`.
- Build proof obligations for annotations, calls, returns, guards, and indexes.
- Call `lang-refinement-checker`.
- Emit runtime checks for unknown outcomes in lenient mode.

Acceptance:

- `ascii-info` rejects unconstrained calls in strict mode;
- guard-based narrowing proves safe calls;
- an out-of-range literal produces a counter-example diagnostic.

### TW05-D - compiler data model in typed Twig

- Implement `Token`, `Span`, diagnostics, AST, type AST, and IIR builders in
  typed Twig.
- Prove basic invariants with refinements.

Acceptance:

- typed Twig modules define the compiler's data structures;
- generated IIR builder APIs reject invalid register and instruction ids.

### TW05-E - self-hosted lexer and parser

- Port lexer and parser to typed Twig.
- Keep the host compiler as oracle.
- Compare token streams and ASTs against the existing implementation.

Acceptance:

- self-hosted lexer/parser match existing fixtures;
- injected cursor/span bugs fail refinement checking.

### TW05-F - self-hosted IIR emitter

- Port resolver and IIR emitter to typed Twig.
- Emit debug sidecar data.
- Use the host compiler only to compile the compiler itself.

Acceptance:

- self-hosted compiler compiles sample Twig programs to IIR equivalent to the
  host compiler;
- sidecar line tables match source spans.

### TW05-G - fixed-point self compilation

Build stages:

```text
stage0: host Rust/Python compiler builds typed Twig compiler
stage1: stage0 compiler compiles typed Twig compiler
stage2: stage1 compiler compiles typed Twig compiler again
```

Acceptance:

- stage1 and stage2 IIR are byte-for-byte identical after deterministic
  serialization, or equivalent under an approved normalizer;
- stage1 and stage2 compile the fixture suite identically;
- no host compiler is used after stage0.

### TW05-H - strict refined compiler

- Turn compiler modules to `(typed strict)`.
- Enforce no public `any`.
- Treat unknown proof obligations as compile errors in compiler modules.
- Allow narrow opt-outs only with an explicit annotation:

```scheme
(unsafe-assume "parser stack invariant proven by enclosing loop" expr)
```

Every `unsafe-assume` must carry a string reason and must appear in an audit
report.

Acceptance:

- compiler source builds in strict mode;
- audit report is empty or every opt-out is tracked by issue id;
- mutation tests that break cursor bounds, arity, or register bounds fail at
  compile time.

## Artifact and CLI shape

The long-term CLI should be:

```bash
twigc --emit=iir src/main.tw -o main.iir
twigc --emit=aot src/main.tw -o main
twigc --emit=wasm src/main.tw -o main.wasm
twigc --check src/main.tw
twigc --check --strict-refinements src/main.tw
twigc --self-check code/twig/compiler/compiler/driver.tw
```

`twigc --self-check` runs stage0/stage1/stage2 and verifies the fixed point.

## Runtime checks

Runtime checks are allowed for user programs in lenient mode.  The compiler
source itself must eventually avoid them in public APIs.

Runtime check lowering:

- `refine_assert` IIR instruction if available; otherwise
- `call_builtin "refine_assert"` with predicate metadata; otherwise
- explicit guard and diagnostic path emitted by the compiler.

Runtime checks must include:

- source span;
- expected refined type;
- actual value where available;
- checker reason if the compile-time result was unknown.

## Diagnostics

Refinement diagnostics must be concrete:

```text
compiler/parser.tw:118:17
parse-list calls token-at with next = token-count

required:
  next : (Int 0 token-count)

counter-example:
  token-count = 12
  next = 12

reason:
  token-at requires an index strictly less than token-count.
```

Diagnostics should prefer source names over generated solver variable names.

## Test strategy

Required tests:

- parser fixtures for typed syntax;
- unit tests for type alias expansion;
- checker tests for base type failures;
- checker tests for refinement safe/unsafe/unknown outcomes;
- flow narrowing tests for `if`, `and`, `or`, `not`, and `match`;
- compiler invariant tests for spans, token indexes, register ids, jump targets,
  and arity;
- self-hosted lexer/parser parity tests;
- self-hosted IIR parity tests;
- stage1/stage2 fixed-point tests;
- mutation tests that intentionally break common compiler invariants.

## Definition of done

TW05 is complete when:

- the Twig compiler is written primarily in typed Twig modules;
- the compiler source builds in strict refinement mode;
- `twigc --self-check` reaches a stage1/stage2 fixed point;
- sample programs compiled by the self-hosted compiler run on the LANG VM;
- the same programs can use the existing downstream paths to AOT, JIT, JVM,
  CLR, and WASM;
- refinement tests catch at least:
  - out-of-bounds token access;
  - invalid source spans;
  - wrong call arity;
  - unresolved labels;
  - invalid register ids;
  - non-exhaustive union matching.

The result is a compiler whose implementation language, static type system, and
constraint-solver-backed refinement layer are all exercised by the compiler
itself.
