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
| LANG27 | Defines the Rust IIR-to-host-VM path for JVM, CLR, BEAM, WASM, and future VM targets. |

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

---

## Appendix A — Native GC and the self-hosted compiler

The self-hosted compiler is a Twig program.  Every AST node, every token list,
every IIR builder accumulator is a heap-allocated object managed by whichever
backend the compiler runs on:

- When bootstrapped through the **Rust interpreter** (`twig-vm`), cons cells and
  records are managed by `lispy-runtime`'s heap.
- When compiled to **JVM**, CLR, BEAM, or WASM via LANG31, the compiler's own
  data structures (token lists, AST trees, IIR instruction lists) become native
  JVM/CLR/BEAM/WASM GC-managed objects.

This means the self-hosted compiler is the most demanding test of the LANG31
heap-op lowering pipeline: a compiler that compiles itself exercises every
allocation site the language surface can produce.

### A.1  How AST nodes are allocated

A `(record Token kind lexeme span)` in typed Twig lowers to a `ref<Token>` IIR
allocation.  The IIR emitter produces:

```
alloc %t  type_hint="ref<Token>"  may_alloc=true
field_store %t 0 %kind
field_store %t 1 %lexeme
field_store %t 2 %span
```

On the JVM backend this becomes `anewarray 3` (a 3-element `Object[]`); on BEAM
it becomes three nested `put_list` calls.  The GC trace starts here: every call
to `lex()` allocates a token; every call to `parse()` allocates AST nodes; the
whole pipeline is allocation-heavy, which makes it a realistic GC stress test.

### A.2  Nil termination and list traversal

Token lists and instruction lists in the compiler are Lisp lists (`cons`/`nil`).
The `length` function used in parser bookkeeping:

```scheme
(define (length (xs : (List Token)) -> Nat)
  (if (null? xs)
      0
      (+ 1 (length (cdr xs)))))
```

The refinement `Nat` on the return type means the solver must prove
`length ≥ 0` — trivially true, but it forces the solver to track the base case
and confirms the function cannot return a negative value.

The token cursor invariant:

```scheme
; Token cursor stays strictly within the token list.
(define (advance
          (pos   : (TokenIndex token-count))
          (toks  : (List Token where (= (list-length toks) token-count)))
        -> (TokenIndex token-count))
  (if (< (+ pos 1) token-count)
      (+ pos 1)
      pos))    ; clamp at end — parser must check (not advance past)
```

The solver proves `(+ pos 1)` is within `(Int 0 token-count)` when
`pos < token-count - 1`, and the clamp branch is proven safe by `pos < token-count`.

---

## Appendix B — Worked examples: refinement types catching real compiler bugs

Each example shows a real class of compiler bug, the Twig code where the bug
would hide, the refinement annotation that exposes it, and the solver output.

### B.1  Off-by-one in the lexer cursor

**Bug class:** The lexer advances `pos` past the end of the source string and
reads a character at `source[source-len]`, causing an out-of-bounds access.

**Where it hides:** Any `(string-ref src pos)` call in `scan-token`.

**Twig annotation:**

```scheme
(define (scan-token
          (src : String)
          (pos : (Index (string-length src)))
        -> Token)
  (let ((ch (string-ref src pos)))  ; ← solver checks pos : (Index src-len)
    ...))
```

`string-ref` is declared as:

```scheme
(define (string-ref (s : String) (i : (Index (string-length s))) -> Char) ...)
```

**Solver interaction:**

If the caller passes `pos = string-length src` (one past the end), the solver
produces:

```
compiler/lexer.tw:44:5
scan-token receives pos out of bounds

required:
  pos : (Index (string-length src))    ; i.e. (Int 0 (string-length src))

counter-example:
  string-length src = 42
  pos = 42

reason:
  pos must be strictly less than (string-length src); 42 is not less than 42.
```

**Why this matters:** Without the annotation, this is a silent buffer read
returning garbage or a runtime panic.  With the refinement, it is a compile-time
error at the call site.

---

### B.2  Parser producing a wider token range than the input

**Bug class:** The parser slices a token range `[start, end)` where `end > token-count`,
producing a range that would cause downstream index errors.

**Where it hides:** `parse-list` builds child nodes and returns a `ParseOk` that
includes the new cursor position.

**Twig annotation:**

```scheme
(record ParseOk
  (expr : Expr)
  (next : (where Int (and (<= start x) (<= x token-count)))))
```

The `next` field is refined: it must be at least `start` (parser made progress)
and at most `token-count` (parser did not go past the end).

**Bug introduced deliberately:**

```scheme
; Buggy: when tokens run out we return token-count + 1 as a sentinel.
(define (parse-list ...)
  (if (exhausted? toks pos)
      (ParseOk (ErrorNode "unterminated list") (+ token-count 1))  ; BUG
      ...))
```

**Solver output:**

```
compiler/parser.tw:88:16
ParseOk.next exceeds token-count

required:
  next : (where Int (<= x token-count))

counter-example:
  token-count = 10
  next = 11   (= token-count + 1)

reason:
  (+ 10 1) = 11, which violates (<= x 10).
```

The fix is to return `token-count` (the legal sentinel for exhaustion):

```scheme
(ParseOk (ErrorNode "unterminated list") token-count)
```

---

### B.3  IIR emitter allocating a register index that overflows BEAM

**Bug class:** The BEAM target allows only x-registers 0..254 (255 values,
since some opcodes encode register counts as a byte).  A program with ≥ 255
live variables would silently truncate the register index or corrupt the
instruction stream.

**Where it hides:** The IIR emitter's `alloc-reg` function.

**Twig annotation:**

```scheme
(type BeamReg (Int 0 255))   ; BEAM x-registers: x0 .. x254

(define (alloc-reg
          (frame : IirBuilder)
          (hint  : String)
        -> (Result BeamReg BuildError))
  (let ((idx (builder-next-reg frame)))
    (if (< idx 255)
        (ok idx)
        (err (BuildError::RegOverflow idx)))))
```

The return type `(Result BeamReg BuildError)` forces all callers to handle the
overflow case.  Any call site that ignores the `err` branch produces a type error
in strict mode.

**Downstream refinement propagation:**

```scheme
(define (emit-load
          (reg   : BeamReg)         ; narrowed: (Int 0 255)
          (src   : Operand)
          (out   : IirBuilder)
        -> IirBuilder)
  ...)
```

`emit-load` requires a `BeamReg`, not a raw `Int`.  Every call that passes an
unverified register index is a type error.  Callers must either prove the index
is in range or handle the error through `alloc-reg`'s `Result`.

---

### B.4  Jump target referring to a label that was never emitted

**Bug class:** The IIR emitter generates a `jmp target` where `target` is a
label id that has no corresponding `label target` instruction in the function.
This produces corrupted bytecode that either crashes or silently mis-executes.

**Where it hides:** `emit-if` emits a forward jump before the else branch is
generated.  If the else branch generation is accidentally skipped (e.g. in
an early-return path), the target label is never emitted.

**Twig type for IIR builders:**

```scheme
(record IirBuilder
  (instrs       : (List IirInstr))
  (label-count  : Nat)
  (used-labels  : (Set LabelId))          ; labels referenced in jumps
  (def-labels   : (Set LabelId)))         ; labels defined via "label" instr

; Invariant enforced at finalise():
(define (finalise (b : IirBuilder) -> (Result IirFunction BuildError))
  (if (subset? (used-labels b) (def-labels b))
      (ok (to-function b))
      (err (BuildError::UndefinedLabels
              (set-diff (used-labels b) (def-labels b))))))
```

The `finalise` function proves (at runtime) that no jump targets are dangling.
With refinements, the solver can go further: if `emit-if` always emits both
branches before `finalise`, the checker can prove the sets are always equal and
eliminate the runtime check entirely.

**Solver output for a code path that skips the else branch:**

```
compiler/emit_iir.tw:213:5
finalise: used-labels ⊄ def-labels

required:
  (subset? used-labels def-labels) = true

counter-example:
  used-labels  = {label_7}
  def-labels   = {}

reason:
  label_7 was added to used-labels by emit-jmp at line 207
  but no emit-label call with label_7 was observed on any path to finalise.
```

---

### B.5  Call arity mismatch at a known call site

**Bug class:** A function defined as `(define (f x y) ...)` is called as
`(f 1)`, passing too few arguments.  This produces incorrect register
allocation and silent wrong output at runtime.

**Where it hides:** The IIR emitter's `emit-call` function, which trusts that
the argument list has the right length without checking.

**Twig type:**

```scheme
(record FunctionSig
  (name   : Symbol)
  (arity  : Nat))

(define (emit-call
          (sig  : FunctionSig)
          (args : (List Operand where (= (list-length args) (arity sig))))
          (dest : FrameSlot)
          (out  : IirBuilder)
        -> IirBuilder)
  ...)
```

The refinement `(= (list-length args) (arity sig))` is a dependent predicate
linking the length of the argument list to the known arity of the function.

**Solver output for a call with wrong arity:**

```
compiler/emit_iir.tw:301:7
emit-call: argument list length does not match signature arity

required:
  (= (list-length args) (arity sig))

counter-example:
  (arity sig) = 2    ; f takes 2 parameters
  (list-length args) = 1    ; caller passes 1

reason:
  The list has length 1 but the signature requires length 2.
```

---

## Appendix C — IIR builder API in typed Twig

This appendix gives the full typed Twig API for the IIR builder, showing how
every method uses refinements to prevent invalid instruction generation.

```scheme
(module compiler/iir-builder
  (typed strict)
  (export IirBuilder new-builder emit-const emit-add emit-sub emit-mul
          emit-cmp-eq emit-cmp-lt emit-jmp emit-jmp-if-true emit-label
          emit-call emit-ret finalise)
  (import compiler/iir-types compiler/span))

;;; ─── Types ───────────────────────────────────────────────────────────────

(type LabelId    Nat)
(type FrameSlot  (fn (frame-size) (Int 0 frame-size)))

(union TypeHint
  (TInt)  (TBool)  (TAny)  (TRef (name : Symbol)))

(record IirInstr
  (op        : Symbol)
  (dest      : (Option Symbol))
  (srcs      : (List Symbol))
  (type-hint : TypeHint)
  (span      : Span))

(record IirBuilder
  (name          : Symbol)
  (instrs        : (List IirInstr))
  (reg-count     : Nat)
  (label-count   : LabelId)
  (used-labels   : (List LabelId))
  (def-labels    : (List LabelId)))

;;; ─── Construction ────────────────────────────────────────────────────────

(define (new-builder (name : Symbol) -> IirBuilder)
  (IirBuilder name nil 0 0 nil nil))

;;; ─── Slot allocation ──────────────────────────────────────────────────────

(define (alloc-slot
          (b    : IirBuilder)
        -> (values IirBuilder Symbol))
  (let* ((idx  (reg-count b))
         (name (symbol-append "r" (number->string idx)))
         (b2   (set-reg-count b (+ idx 1))))
    (values b2 name)))

;;; ─── Label allocation ─────────────────────────────────────────────────────

(define (alloc-label
          (b : IirBuilder)
        -> (values IirBuilder LabelId))
  (let* ((id (label-count b))
         (b2 (set-label-count b (+ id 1))))
    (values b2 id)))

;;; ─── Instruction emitters ─────────────────────────────────────────────────

(define (emit-const
          (b     : IirBuilder)
          (dest  : Symbol)
          (val   : Int)
          (span  : Span)
        -> IirBuilder)
  (append-instr b (IirInstr 'const (some dest) (list (number->string val))
                             TInt span)))

(define (emit-add
          (b     : IirBuilder)
          (dest  : Symbol)
          (a b-op : Symbol)
          (span  : Span)
        -> IirBuilder)
  (append-instr b (IirInstr 'add (some dest) (list a b-op) TInt span)))

;; emit-sub, emit-mul, emit-div identical pattern (omitted for brevity)

(define (emit-cmp-eq
          (b     : IirBuilder)
          (dest  : Symbol)
          (a b-op : Symbol)
          (span  : Span)
        -> IirBuilder)
  (append-instr b (IirInstr 'cmp_eq (some dest) (list a b-op) TBool span)))

(define (emit-label
          (b    : IirBuilder)
          (lbl  : LabelId)
          (span : Span)
        -> IirBuilder)
  (let ((b2 (set-def-labels b (cons lbl (def-labels b)))))
    (append-instr b2 (IirInstr 'label none (list (number->string lbl))
                               TAny span))))

(define (emit-jmp
          (b    : IirBuilder)
          (lbl  : LabelId)
          (span : Span)
        -> IirBuilder)
  (let ((b2 (set-used-labels b (cons lbl (used-labels b)))))
    (append-instr b2 (IirInstr 'jmp none (list (number->string lbl))
                               TAny span))))

(define (emit-jmp-if-true
          (b     : IirBuilder)
          (cond  : Symbol)
          (lbl   : LabelId)
          (span  : Span)
        -> IirBuilder)
  (let ((b2 (set-used-labels b (cons lbl (used-labels b)))))
    (append-instr b2 (IirInstr 'jmp_if_true none (list cond (number->string lbl))
                               TAny span))))

(define (emit-call
          (b      : IirBuilder)
          (dest   : Symbol)
          (target : Symbol)
          (args   : (List Symbol))
          (span   : Span)
        -> IirBuilder)
  (append-instr b (IirInstr 'call (some dest)
                             (cons target args)
                             TAny span)))

(define (emit-ret
          (b    : IirBuilder)
          (val  : Symbol)
          (span : Span)
        -> IirBuilder)
  (append-instr b (IirInstr 'ret none (list val) TAny span)))

;;; ─── Finalisation ─────────────────────────────────────────────────────────

;;; A builder is valid when every used label has been defined.
(define (well-formed? (b : IirBuilder) -> Bool)
  (subset? (used-labels b) (def-labels b)))

(define (finalise
          (b      : IirBuilder)
          (params : (List (Pair Symbol TypeHint)))
          (ret    : TypeHint)
        -> (Result IirFunction BuildError))
  (if (well-formed? b)
      (ok (IirFunction
            (name b)
            params
            ret
            (reverse (instrs b))
            (reg-count b)))
      (err (BuildError::UndefinedLabels
              (set-diff (used-labels b) (def-labels b))))))
```

The key refinement contract: `emit-label` adds to `def-labels`;
`emit-jmp`/`emit-jmp-if-true` add to `used-labels`; `finalise` checks
`used ⊆ defined`.  In strict mode with the solver, the checker can often prove
`well-formed?` statically when the control flow graph has no unreachable emit
paths.

---

## Appendix D — Bootstrapping the compiler on each backend

Once the self-hosted compiler source (Twig modules in `code/twig/compiler/`) is
complete, the pipeline for a full bootstrap is:

```
Stage 0:  Rust twig-ir-compiler compiles compiler.tw → IIR
          IIR runs on twig-vm (interpreted) → self-hosted binary B0

Stage 1:  B0 compiles compiler.tw → IIR
          IIR is lowered by iir-to-jvm (or beam, wasm, cil) → bytecode B1

Stage 2:  bytecode B1 (running on real JVM/BEAM/WASM/CLR) compiles compiler.tw
          → IIR, then → bytecode B2

Fixed-point check: B1 and B2 must produce identical IIR (or identical bytecode
after deterministic serialisation).
```

This is the same three-stage structure as Rust's `x.py`, GHC's boot compiler,
and Zig's bootstrap.  The novelty here is that stage 1 and stage 2 can run on
*different* backends (e.g., stage 1 on JVM, stage 2 on BEAM) and still produce
equivalent IIR — since IIR is the canonical intermediate form that is
target-independent by definition.

The `twigc --self-check` command automates stages 0-2 and reports:

```
Stage 0 ... ok (12.4 s)
Stage 1 (JVM) ... ok (0.9 s)
Stage 2 (JVM) ... ok (0.8 s)
Fixed-point check ... PASS (IIR hashes match: a3f7c9d2)

Cross-backend check:
  Stage 2 (BEAM) ... ok
  Stage 2 (WASM) ... ok
  Stage 2 (CLR)  ... ok
  All IIR hashes match.
```

The cross-backend check is the strongest proof of correctness: the same source
compiled by binaries running on four different VMs produces the same intermediate
code.
