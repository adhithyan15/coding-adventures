# Changelog — twig-ir-compiler

## [0.43.0] — 2026-07-13 (LANG-FULL E6d-4 — quote literals as interned symbol consts)

A Twig quote literal (`'a` / `(quote a)`, an `Expr::SymLit`) now lowers to
`const Var(name) : symbol` — the **same interned-const form McCarthy Lisp's
`emit_symbol` emits** — instead of the runtime `make_symbol` string path (`const
Var(name) : any` + `call_builtin "make_symbol"`).

Why: `make_symbol` needs data-section string-literal emission that the code-gen
backends (native/LLVM/WASM/JVM/CLR) don't have, so quoted symbols never ran there.
The interned-const form rides the already-wired `intern_symbols` (native) /
`intern_symbols_structural` (managed) passes, which assign each distinct name one
module-wide id in a reserved high range — so `equal?` on symbols is bit-equality
(`(equal? 'a 'a)` #t, `(equal? 'a 'b)` #f) on **all five code-gen backends**, with
no new value type. On twig-vm the `const Var(name)` dispatch already interns the
text to a symbol, so the VM is unaffected (176 twig-vm tests pass). Runtime symbol
*creation* (`string->symbol` over a runtime string) keeps `make_symbol`.

The `quoted_symbol_emits_make_symbol` unit test is updated to
`quoted_symbol_emits_interned_const`.

## [0.37.0] — 2026-06-28 (LANG-FULL E4 — direct-call string parameter inference)

Top-level Twig functions can now infer `str` for otherwise-unannotated
parameters from conservative `main`-level direct-call evidence:

```scheme
(define (strlen s) (string-length s)) (strlen "HELLO")
```

The prepass only considers direct top-level calls from `main` expressions or
non-lambda define RHSs. A parameter becomes `str` only when observed direct-call
arguments are static E4 string expressions with no conflicting evidence. The
callee body then emits `str_len [i64]`, the caller materialises the string
argument through E4 string ops, and no refinement annotations are synthesized.
Unobserved, conflicting, closure-derived, captured, or reassigned parameter
paths remain dynamic.

## [0.36.0] — 2026-06-28 (LANG-FULL E4 — annotated string parameters)

Bare `str` / `string` parameter annotations on top-level Twig functions now seed
the compiler's static IIR type map. A function body can therefore lower
parameter-derived string operations through E4:

```scheme
(define (strlen (s : str)) (string-length s)) (strlen "HELLO")
```

The callee parameter is `str`, the body emits `str_len [i64]`, and direct callers
with known string arguments materialise those arguments through E4 string ops
instead of raw `const [str]`.

## [0.35.0] — 2026-06-28 (LANG-FULL E4 — top-level string function returns)

Direct calls to top-level Twig functions now inherit the function's statically
known return type when the function body already lowers to typed IIR. This lets a
function-wrapped E4 string operation run through the code-gen backends:

```scheme
(define (strlen) (string-length "HELLO")) (strlen)
```

The function body emits `str_len [i64]`, its `ret` carries `i64`, and the caller's
`call` result is typed `i64` rather than `any`.

## [0.34.0] — 2026-06-28 (LANG-FULL E4 — lexical string ordering predicates)

Literal and known local `string<?` / `string>?` expressions now lower through
the shared `str_cmp` op followed by typed integer comparison against zero.

## [0.33.0] — 2026-06-28 (LANG-FULL E4 — substring feeds string-ref proof)

Lexical string bindings can now feed `substring`, and the resulting string can
feed `string-ref` through shared E4 ops:

```scheme
(let ((s "ABCDE")) (string-ref (substring s 1 4) 1))
```

The compiler emits `str_slice` for `substring` when the source is a known E4
string expression and both bounds are known E4 index expressions. The indexed
byte is `67` (`C` in `BCD`), with no dynamic string builtin fallback.

## [0.32.0] — 2026-06-28 (LANG-FULL E4 — computed string index proof)

Lexical string bindings can now compute a `string-ref` index with typed integer
arithmetic over `string-length`:

```scheme
(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))
```

The E4 recognizer accepts this statically-typed index expression, so the
compiler emits `str_len`, typed `sub`, and `str_index` without falling back to
dynamic `string-length`, `-`, or `string-ref` builtins. The indexed byte is
`69` (`E` in `ABCDE`).

## [0.31.0] — 2026-06-28 (LANG-FULL E4 — local concat indexing proof)

Lexical string bindings now have an explicit compiler proof that the result of
`string-append` can feed `string-ref` through E4 without falling back to dynamic
string builtins:

```scheme
(let ((a "AB") (b "CDE") (i 3)) (string-ref (string-append a b) i))
```

The compiler lowers the local strings to typed `str_const` registers, emits
`str_concat` for the append result, then consumes that temporary with
`str_index`. The indexed byte is `68` (`D` in `ABCDE`).

## [0.30.0] — 2026-06-27 (LANG-FULL E4 — lexical string equality branch proof)

Lexical string bindings now have an explicit compiler proof for `string=?`
driving control flow:

```scheme
(let ((s "OK") (t "OK")) (if (string=? s t) 42 0))
```

The compiler lowers both locals to typed `str_const` registers, feeds them to
E4 `str_eq`, and branches on the resulting i64 boolean without using the
dynamic `call_builtin` path.

## [0.29.0] — 2026-06-27 (LANG-FULL E4 — lexical string concat proof)

Lexical string bindings now have an explicit compiler proof for non-literal E4
concat: `(let ((a "AB") (b "CDE")) (string-length (string-append a b)))`
lowers to direct `str_const` locals, `str_concat`, and `str_len`, with no
dynamic `call_builtin` path.

This does not widen the representation boundary: captured/reassigned strings
and broader dynamic string values remain follow-up E4 work.

## [0.28.0] — 2026-06-27 (LANG-FULL E4 — lexical string locals)

Lexical `let` and `let*` string literal bindings now materialize as typed
`str_const` registers instead of legacy dynamic `const Operand::Str` values when
they feed the E4 string-op path. The E4 recognizer now accepts known local
typed `str` registers and local integer index registers, so `(let ((s "ABC")
(i 2)) (string-ref s i))` lowers to shared `str_index` without `call_builtin`.

This proves local string slots for the current literal-value foothold while still
leaving captured/reassigned strings and broader dynamic string values to the
future byte-string representation work.

## [0.27.0] — 2026-06-27 (LANG-FULL E4 — named string values)

Immutable top-level string value defines now lower to `str_const` registers when
they are not captured by a lambda or forced through a forward reference. Reads of
those names stay in `main`, so `string-length`, `string-append`, `string=?`, and
`string-ref` can lower to the shared E4 `str_len`/`str_concat`/`str_eq`/
`str_index` ops over named string values instead of falling back to dynamic
globals or `call_builtin`.

This deliberately does not claim full string variable semantics: `let`/reassignable
string slots, captured strings, and dynamic string values still stay on their
existing paths until the broader E4 string representation lands.

Verified by compiler tests for named string length, concat length, equality in a
branch, and indexing, plus new `lang-aot` matrix rows across all seven backends.

## [0.26.0] — 2026-06-27 (LANG-FULL E4 — literal string index)

Literal `(string-ref "..." i)` now lowers to the shared E4 `str_index` path:
`str_const` for the direct literal, an integer `const` for the index, then
`str_index` for the typed byte result. This adds the direct ASCII indexing
foothold to the existing literal string metadata paths while preserving the
dynamic `call_builtin` path for non-literal string values.

Verified by a compiler test asserting the exact `str_const` + `const` +
`str_index` + `ret` shape and by the cross-backend `lang-aot` matrix row.

## [0.25.0] — 2026-06-27 (LANG-FULL E4 — literal string metadata)

Literal `(string-length "...")`, `(string=? "..." "...")`, and
`(string-length (string-append "..." "..."))` now lower to the shared E4 VM
string ops: `str_const` for each direct literal, then `str_len`, `str_eq`, or
`str_concat` for the typed result path. This gives the code-gen backends typed,
language-neutral string-length, string-equality, and literal-append-length proofs
while preserving the existing dynamic `call_builtin` paths for non-literal
string values.

Verified by compiler tests asserting the exact `str_const` +
`str_len`/`str_eq`/`str_concat` + `ret` shapes and by the cross-backend
`lang-aot` matrix rows.

## [0.24.0] — 2026-06-14 (Path A increment 4 — top-level value `define`, LANG-FULL TW2)

### Added — a `main`-only value `define` lowers to a typed local

A top-level value `define` previously lowered to `call_builtin "global_set" name
value`, and every read to `call_builtin "global_get" name` — both carrying
`type_hint = "any"`, which every IIR-to-{llvm,wasm,jvm,clr} backend validator
rejects.  So a program as simple as `(define x 40) (define y 2) (+ x y)` ran only
on the VM.

This increment adds a small **escape analysis** (`free_vars::lambda_captured_globals`):
a value `define` that is **not captured by any lambda** is read only from `main`,
so the compiler keeps its statically-typed (`i64` / `bool`) value in a `main`
register and resolves reads to that register directly.  No `global_set` /
`global_get` is emitted, so `main` stays fully typed and clears every backend
validator.  Verified by RUNNING: `lang-aot`'s `lang_matrix.rs` executes
`(define x 40) (define y 2) (+ x y)` ⇒ exit 42 across native / LLVM / WASM / JVM
/ CLR / VM / JIT.

A value captured by a closure (read inside a lambda body) still compiles to a
separate function with no access to `main`'s registers, so it stays on the host
global table (`global_set` / `global_get`) exactly as before.  A top-level
forward reference (a read before the matching `define`) likewise stays on the
global table, keeping behaviour byte-identical to the pre-TW2 dynamic path.

## [0.23.0] — 2026-06-14 (Path A increment 3 — typed variadic arithmetic, LANG-FULL TW1)

### Added — n-ary `(+ a b c …)` folds to a typed binary chain

Scheme arithmetic is variadic, but only the binary form `(+ a b)` lowered to a
typed CIR `add`; a three-or-more-argument call (`(+ 10 20 12)`) fell back to the
legacy `call_builtin "+"` path with `type_hint = "any"`, which every
IIR-to-{llvm,wasm,jvm,clr} backend validator rejects — so variadic Twig
arithmetic ran only on the dynamic VM path.

This increment folds an all-`i64` arithmetic call (`+`, `-`, `*`, `/`) into a
**left-associated chain of typed binary mnemonics**:

```text
(+ 10 20 12)   →   r1 = add 10, 20  [i64]
                   r2 = add r1, 12  [i64]      ⇒ result r2
```

so the call now clears every backend validator. Verified by RUNNING:
`lang-aot`'s `lang_matrix.rs` executes `(+ 10 20 12)` ⇒ exit 42 across
native / LLVM / WASM / JVM / CLR / VM / JIT.

Comparisons are deliberately excluded: variadic `(< a b c)` is a chained
predicate (`a<b ∧ b<c`), not a fold, so it stays on the dynamic path. Unary /
nullary forms (`(+)`, `(- a)`) also stay on the fallback; this increment targets
the `n ≥ 2` arithmetic fold (the `n == 2` case was already typed). A call with
any dynamically-typed argument continues to use `call_builtin`.

## [0.22.0] — 2026-05-26 (Path A increment 6c — typed `car` / `cdr`)

### Added — `car` / `cdr` emit `field_load [ref<any>]`

Increment 6c — the closing piece of Twig's list-handling vocabulary.
Replaces every `call_builtin "car" pair [any]` / `call_builtin "cdr" pair [any]`
with the typed Phase 2 form `field_load dest, pair, idx [ref<any>]`
(idx 0 = car, idx 1 = cdr).

After this PR, **zero `call_builtin "car"` / `"cdr"` emission sites**
remain in twig-ir-compiler.  Combined with 6a (typed `make_nil`) and
6b (typed `cons`), the entire cons-cell vocabulary is now typed and
Twig record / union constructors + accessors flow through every
backend.

#### Sites converted (8 total)

| Site | Function context |
|------|------------------|
| `(car matched)` for variant tag extraction | `compile_match` |
| `(cdr cur_cdr)` step in variant field binding | `compile_match` |
| `(car cur_cdr)` to extract field after cdr chain | `compile_match` |
| `(cdr cur)` step in record accessor body | `compile_record_constructor` (record accessor) |
| `(car cur)` to extract field in record accessor | `compile_record_constructor` (record accessor) |
| `(car v)` for union variant tag extraction | `compile_record_constructor` (variant predicate) |
| `(cdr cur)` step in union variant accessor | `compile_record_constructor` (variant accessor) |
| `(car cur)` to extract field in variant accessor | `compile_record_constructor` (variant accessor) |

#### Companion changes

- **twig-vm 0.22.0**: new `exec_field_load` dispatch arm.  Reads
  `car` (idx 0) or `cdr` (idx 1) from a cons cell via
  `lispy_runtime::heap::car` / `cdr`.  Surfaces non-cons input as
  `RuntimeError::TypeError`.
- **iir-to-wasm 0.6.0**: validator accepts `ref<any>` in addition to
  `ref<LispyPair>` for `field_load` results.  WasmGC lowering already
  uses `anyref` for cons-cell fields, so this matches the actual code
  shape.
- **iir-to-jvm-class-file 0.6.0**: validator accepts `ref<any>` for
  `field_load` results (lowers to `Object`).
- **iir-to-cil-bytecode 0.6.0**: validator accepts `ref<any>` (lowers
  to `System.Object`) and adds `mov` to the supported-ops list for
  reference types.

#### What this unlocks

- Record programs like `(record Point (x : int) (y : int))` now flow
  end-to-end through wasm/jvm/clr/beam (constructor + accessors).
- Union variant programs (constructor + accessor + variant predicate
  body) similarly accept-after-6c (the `pair?` predicate still uses
  `call_builtin "pair?"`, which is out of scope for Path A).

#### Tests

- 1 new backend acceptance test
  (`twig_full_record_program_accepted_by_every_backend`) — asserts
  the constructor + both accessors validate cleanly on every backend.
- All 73 lib + 14 backend e2e + 179 twig-vm + 88 iir-to-wasm +
  86 iir-to-jvm + 83 iir-to-cil + 65 iir-to-beam tests pass.

## [0.21.0] — 2026-05-24 (Path A increment 6b — typed `cons`)

### Added — `cons` cells emit `alloc` + `field_store` triples

Increment 6b of the Twig → IIR-to-* end-to-end story.  Replaces every
`call_builtin "cons" head tail [any]` emission site with the typed
three-instruction triple:

```
alloc cell [ref<LispyPair>]
field_store cell, 0, head [void]   -- car
field_store cell, 1, tail [void]   -- cdr
```

matching the Phase 2 heap-lowering convention used by every IIR-to-*
backend and the iir-builtin-lowering pass.  Record and union
constructors — which build cons chains internally — now emit IR that
every backend accepts.

#### Sites converted (3 total)

| Site                                | Function context             |
|-------------------------------------|------------------------------|
| Record constructor cons-chain       | `compile_record_constructor` (record) |
| Union variant cons-chain            | `compile_record_constructor` (union variant) |
| Union variant tag prepend           | `compile_record_constructor` (variant tag head) |

#### Companion changes

- **twig-vm 0.21.0**: new `exec_alloc` and `exec_field_store` dispatch
  arms that allocate a fresh `(NIL,NIL)` cons cell and mutate fields
  in place via `lispy_runtime::heap::set_field_unchecked`.
- **lispy-runtime 0.5.0**: new `unsafe fn set_field_unchecked(pair,
  index, value)` that mutates `car` or `cdr` of a live ConsCell.  The
  function re-validates the class id internally so misuse on
  non-cons heap values surfaces as Err rather than memory corruption.
- **iir-to-wasm validator**: now accepts `field_store [void]` (the
  canonical Phase 2 form) in addition to `field_store [ref<*>]`.  CLR,
  JVM, BEAM already accepted the void form.

#### What this unlocks

- Twig record constructors (e.g. `(record Point (x : int) (y : int))`)
  now emit backend-valid IR (new test
  `twig_record_constructor_emits_typed_alloc_and_field_store`).
- Full-module backend acceptance for record / union programs is still
  bottlenecked on `car`/`cdr` accessors — that's increment 6c.

#### What's not in this PR

- `car` / `cdr` → typed `field_load [ref<any>]` (increment 6c, 8 sites)
- `pair?` predicate (later)

After 6c, no list-handling op in twig-ir-compiler emits an untyped
`call_builtin`, and Twig programs that build / traverse lists flow
end-to-end through every backend.

#### Tests

- 1 new backend acceptance test
  (`twig_record_constructor_emits_typed_alloc_and_field_store`).
- All 73 lib + 13 backend e2e + 179 twig-vm tests pass.

## [0.20.0] — 2026-05-23 (Path A increment 6a — typed `make_nil`)

### Added — `nil` literal emits `const 0 [ref<LispyPair>]`

Increment 6a of the Twig → IIR-to-* end-to-end story.  Replaces
every `call_builtin "make_nil" [any]` emission site with a typed
`const 0 [ref<LispyPair>]`, matching the Phase 2 heap-lowering
convention used by the IIR-to-{wasm,jvm,clr,beam} backends.

Nil is encoded as the null `LispyPair` reference — represented as
`0` — which is exactly how `iir-builtin-lowering/src/heap.rs:288`
produces it from the legacy `call_builtin "make_nil"` form.  Every
backend already accepts the typed const for `ref<*>` types.

#### Sites converted (5 total)

| Site                                | Function                       | Where                                |
|-------------------------------------|--------------------------------|--------------------------------------|
| Empty program return-nil            | `compile_program`              | Implicit `nil` when no final expr    |
| `nil` literal                       | `compile_expr_inner`           | `Expr::NilLit` match arm             |
| `match` fallthrough init            | `compile_match`                | Initial `result` register value      |
| Record constructor cons-chain tail  | `compile_record_constructor`   | Fold-right base for record fields    |
| Union variant cons-chain tail       | `compile_record_constructor`   | Fold-right base for variant fields   |

#### What this unlocks

- `nil` literal flows through every backend (new test
  `twig_nil_literal_accepted_by_every_backend`).
- Empty Twig program flows through every backend (new test
  `twig_empty_program_accepted_by_every_backend`).
- `match` programs whose arms all have known types now have a
  typed fallthrough register, removing the last untyped `mov`
  source in `compile_match`.

#### twig-vm dispatch wrapper (companion change)

twig-vm's `exec_const` now special-cases `const … [ref<LispyPair>]`
with `Operand::Int(0)` to produce `LispyValue::NIL` (instead of the
plain `LispyValue::int(0)` it would otherwise emit).  Without this,
HOF builtins like `map` / `filter` / `fold-*` would see `Int(0)`
where they expect the NIL sentinel and bail out with
`"list tail 0 is not a cons cell"`.

This is the symmetric companion to the increment 2 dispatch wrapper
that synthesised `call_builtin "+"` from typed `add`.

#### What's not in this PR

Still pending for increment 6:

- `cons` → typed `alloc` + `field_store` (increment 6b, 3 sites)
- `car` / `cdr` → typed `field_load` (increment 6c, 8 sites)

After 6b and 6c, no list-handling op in twig-ir-compiler emits an
untyped `call_builtin`, and Twig programs that build / traverse
lists flow end-to-end through every backend.

#### Tests

- 2 new backend acceptance tests (`nil`, empty program).
- Lib test `nil_literal_emits_make_nil_builtin` renamed to
  `nil_literal_emits_typed_const_ref_lispy_pair` and updated.
- Lib test `empty_program_returns_nil` renamed to
  `empty_program_returns_typed_nil_const` and updated.
- All 73 lib + 12 backend e2e + 179 twig-vm tests pass.

## [0.19.0] — 2026-05-22 (Path A — typed `match` arm merges)

### Added — Typed `mov` for the 7 remaining `compile_match` sites

Increment 5 of the Twig → IIR-to-* end-to-end story.  Converts all
remaining `call_builtin "_move"` emission sites in `compile_match`
to typed `mov` via `FnCtx::emit_move`.

After this PR, **zero `call_builtin "_move"` emission sites** remain
in twig-ir-compiler.  Every value-copy goes through the typed `mov`
opcode.

#### Sites converted

| Site                           | Description                                      |
|--------------------------------|--------------------------------------------------|
| Scrutinee → matched (initial)  | Bind scrutinee to a stable register              |
| nil_init → result              | Default fallthrough value                         |
| arm_result → result (unknown variant) | Bare-binding arm result merge               |
| field_reg → binding            | Field extraction in variant arms                  |
| body_v → result (variant arm)  | Variant-arm body result merge                     |
| arm_result → result (binding arm) | Binding-arm result merge                       |
| body_v → result (wildcard arm) | Wildcard-arm body result merge                    |
| matched_reg → name (helper)    | Binding-arm helper's name binding                 |

#### What's not in this PR

Match arms produce mixed types in general (variant constructors vs.
raw values), so the match expression's result type stays `"any"` —
no consensus pass across arms.  Adding that is a separate increment
(would need a Hindley–Milner-style unifier over the arm types).

#### What this unlocks

The IR is now structurally valid for the backends: no
`call_builtin "_move"` survives in compile_match's output.
Backend acceptance of full match programs is bottlenecked on
`make_nil`, `car`, `cdr`, `=` over reference types — which all
still emit `call_builtin "<op>"` with `type_hint "any"`.  Those are
the next increments.

#### Tests

- New e2e test `twig_typed_match_wildcard_accepted_by_every_backend`
  asserts the IR no longer contains `call_builtin "_move"` and
  contains at least one typed `mov`.  Full backend acceptance for
  `(match 1 (_ 42))` is deferred — the `make_nil` initialiser still
  emits an untyped `call_builtin`.
- 73 lib + 10 backend e2e tests pass (was 73 + 9).
- 179 twig-vm tests pass.

## [0.18.0] — 2026-05-22 (Path A — typed `let` / `let*` / `and` / `or`)

### Added — Typed `mov` at the remaining branch-merge sites

Increment 4 of the Twig → IIR-to-* end-to-end story.  Extends the
`FnCtx::emit_move` helper from increment 3 to the four other
control-flow sites that previously emitted `call_builtin "_move"`:
`compile_let`, `compile_let_star`, `compile_and`, `compile_or`.

#### What changed

- `compile_let` — each binding's RHS-to-name copy uses `emit_move`.
  Type propagates from RHS to binding.
- `compile_let_star` — same.  Each subsequent RHS sees the previous
  binding's type, so chains like `(let* ((a 1) (b (+ a 1))) b)` end
  up fully typed.
- `compile_and` — both the then-merge and the constant-`#f` else-merge
  use `emit_move`.  The `#f` literal is emitted with `type_hint "bool"`
  (matching `Expr::BoolLit`'s increment-1 behaviour).
- `compile_or` — both the truthy-merge and the falsy-merge use
  `emit_move`.

`compile_match` (variant arms, binding arms, wildcard arms) still
uses `call_builtin "_move"` at 7 sites.  Match programs are deferred
to increment 5+ — the lowering is more complex (variant tag checks,
field extraction, fallthrough) and needs separate review.

#### What this unlocks

| Program                              | wasm | jvm | clr | beam |
|--------------------------------------|------|-----|-----|------|
| `(let ((x 5)) x)`                    | ✅ (was ❌) | ✅ | ✅ | ✅ |
| `(let* ((a 1) (b (+ a 1))) b)`       | ✅ (was ❌) | ✅ | ✅ | ✅ |
| `(let ((x 5) (y 10)) (+ x y))`       | ✅ (was ❌) | ✅ | ✅ | ✅ |
| `(and #t #t)`                        | ✅ (was ❌) | ✅ | ✅ | ✅ |
| `(or #f 42)`                         | ✅ (was ❌) | ✅ | ✅ | ✅ |
| `(match x ((Some v) v))`             | ❌ still rejected — match arm `_move`s | ❌ | ❌ | ❌ |

#### Tests

- Existing `let_binds_via_move` test renamed → `let_binds_via_typed_mov`
  and updated to assert `mov x [i64]`, not `call_builtin "_move"`.
- 2 new e2e tests in `tests/backend_compat.rs`:
  - `twig_typed_let_accepted_by_every_backend` — `(let ((x 5)) x)`
  - `twig_typed_let_star_with_arithmetic` — `(let* ((a 1) (b (+ a 1))) b)`
- 73 lib + 9 backend e2e tests pass (was 73 + 7).

## [0.17.0] — 2026-05-22 (Path A — typed `if` + typed `mov`)

### Added — Typed `mov` for branch-merge sites

Increment 3 of the Twig → IIR-to-* end-to-end story.  Replaces the
legacy `call_builtin "_move"` emission pattern with the **typed `mov`
IR opcode** at branch-merge sites in `compile_if`.  When both arms
of an `(if cond then else)` expression produce a value of the same
statically-known type, the if's result type is recorded so
downstream `ret` instructions propagate it cleanly.

#### What changed

- New `FnCtx::emit_move(dst, src, loc)` helper that emits a typed
  `mov dst = src [type]` instruction.  Type is sourced from
  `var_types[src]` (or `"any"` when unknown).  When the type is
  concrete, the destination is also recorded.
- `compile_if` now uses `emit_move` for both then-branch and
  else-branch merge sites.  After both arms are emitted, the
  compiler computes a *consensus type*: if both arms produce the
  same concrete type, that becomes the if's result type; otherwise
  the result stays `"any"` (matching the existing dynamic semantics).
- `compile_if` no longer emits any `call_builtin "_move"` form.

#### What this unlocks

| Program                          | wasm | jvm | clr | beam |
|----------------------------------|------|-----|-----|------|
| `(if #t 1 2)`                    | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `(if (< 1 2) (+ 10 20) (- 10 20))` | ✅ (combined typed cmp+arith+if) | ✅ | ✅ | ✅ |
| `(if cond 1 "hello")`            | ❌ still rejected — disagreeing arm types (any) | ❌ | ❌ | ❌ |

Programs where both arms produce the same concrete type now flow
through every backend.  Programs whose if branches return different
types still hit the dynamic fallback.

#### twig-vm regression fix

Path-A increments 2 (PR #3949) and this PR break twig-vm runtime
execution by emitting typed CIR mnemonics that twig-vm's dispatch
table doesn't recognise.  PR #3949 includes the corresponding
twig-vm patch (`twig-vm` 0.19.0) — typed CIR mnemonics now
synthesise the equivalent `call_builtin "<runtime_name>"` form and
delegate to the existing builtin dispatch.

#### Tests

- Existing `if_emits_jmp_if_false_and_two_labels` test renamed
  and updated to assert two typed `mov` instructions (zero legacy
  `_move`).
- 2 new e2e tests in `tests/backend_compat.rs`:
  - `twig_typed_if_accepted_by_every_backend` — `(if #t 1 2)`
  - `twig_typed_arithmetic_in_if_accepted_by_every_backend` —
    `(if (< 1 2) (+ 10 20) (- 10 20))`
- 73 lib + 7 backend e2e tests pass.

#### Compatibility

- Non-Twig callers unaffected.
- twig-vm 0.19.0 (this stack, base #3949) handles the typed `mov`
  natively — no Twig program changes runtime behaviour.

## [0.16.0] — 2026-05-22 (Path A — typed binary arithmetic + comparison)

### Added — Typed CIR mnemonics for binary arithmetic / comparison

Increment 2 of the Twig → IIR-to-* end-to-end story.  Builds on
0.15.0's typed-literals work to lower **binary arithmetic** (`+ - * /`)
and **comparisons** (`= < > <= >=`) on i64 arguments to typed CIR
mnemonics (`add`, `sub`, `mul`, `div`, `cmp_eq`, `cmp_lt`, `cmp_gt`,
`cmp_le`, `cmp_ge`) instead of the legacy `call_builtin "<op>"`
dispatch.

Mirrors the same pattern PR #3903 used for Nib
(`compile_binary_chain` → typed CIR mnemonics).

#### What changed

- New `typed_arith_op_for(name) -> Option<&'static str>` table maps
  Twig builtin names to the typed-CIR mnemonic.  9 entries:
  `+ - * /` → `add sub mul div`; `= < > <= >=` → `cmp_eq cmp_lt
  cmp_gt cmp_le cmp_ge`.
- `compile_apply`'s `is_builtin` branch now:
  1. Resolves all argument expressions first (existing behaviour).
  2. For binary forms (n=2) where both args have statically-known
     `i64` type, emits the typed mnemonic with `type_hint = "i64"`
     (arithmetic) or `"bool"` (comparison), records the dest's type,
     and short-circuits the legacy `call_builtin` path.
  3. Otherwise falls back to the existing `call_builtin "<op>"`
     dispatch.
- Result types:
  - `add` / `sub` / `mul` / `div` over `i64` → `i64`.  Recorded so a
    chained expression like `(+ (* 2 3) 4)` flows through the typed
    path for the outer `+` too (the `*` dest is `i64`).
  - `cmp_*` → `bool`.

#### What this unlocks

| Program             | wasm | jvm | clr | beam |
|---------------------|------|-----|-----|------|
| `(+ 1 2)`           | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `(< 1 2)`           | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `(+ (* 2 3) 4)`     | ✅ (typed chain) | ✅ | ✅ | ✅ |
| `(+ (car (cons 1 2)) 3)` | ❌ still rejected (left arg is `any`) | ❌ | ❌ | ❌ |

Variadic forms (`(+ a b c)`, n>2) and arithmetic over dynamically-typed
sources (results of `car` / `length` / user-defined functions) still
flow through `call_builtin`.  Subsequent increments will lower
variadic folds and inject runtime type guards.

#### Tests

- Existing `builtin_call_uses_call_builtin_directly` test renamed
  → `builtin_call_uses_typed_add_for_i64_args` and updated to assert
  the typed path.
- New `builtin_call_falls_back_to_call_builtin_for_dynamic_args`
  asserts the fallback path still fires when an arg is `any`.
- `builtins_recognised` narrowed to non-typed builtins (cons / car /
  cdr / predicates / print) — typed arithmetic moved to its own
  dedicated test.
- 2 new e2e tests in `tests/backend_compat.rs`:
  - `twig_typed_arithmetic_accepted_by_every_backend` — `(+ 1 2)`
  - `twig_typed_comparison_accepted_by_every_backend` — `(< 1 2)`
- The "still rejected" boundary marker from increment 1 has flipped
  to `twig_arithmetic_over_dynamic_args_still_rejected`, pinning the
  current boundary one step further along.
- 73 lib + 5 backend e2e tests pass.

## [0.15.0] — 2026-05-22 (Path A — typed literals + typed return)

### Added — Local type inference for integer / boolean literals

Increment 1 of the Twig → IIR-to-* end-to-end story (the LANG VM
"any frontend, any backend" promise).  A probe against
`iir-to-{wasm,jvm,clr,beam}` validators on the simplest possible Twig
program (`42`) showed every backend rejected it — every instruction
carried `type_hint = "any"`, which the validators all reject with
`UntypedInstruction`.

This release narrows the gap by stamping concrete `type_hint`s on
integer / boolean literals and propagating those types through `ret`
emission sites.

#### What changed

- New `var_types: HashMap<String, String>` on `FnCtx`, populated only
  at sites where the type is statically obvious — literal-defining
  expressions (`IntLit`, `BoolLit`).  Dynamic / `call_builtin`
  destinations are intentionally not recorded; absence means
  "genuinely `any`".
- `Expr::IntLit` now emits `const Int(n)` with `type_hint = "i64"`
  (was `"any"`) and records `var_types[dest] = "i64"`.
- `Expr::BoolLit` now emits `const Bool(b)` with `type_hint = "bool"`.
- `ret` emission sites propagate the source var's inferred type via
  `FnCtx::type_of`.  Dynamic returns still emit `"any"` correctly.
- `main`'s `return_type` is now derived from the last `ret`
  instruction's `type_hint` rather than hard-coded to `"any"`.

#### What this unlocks

The simplest Twig programs flow through every IIR-to-* backend:

| Program   | wasm | jvm | clr | beam |
|-----------|------|-----|-----|------|
| `42`      | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `#t`      | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `(+ 1 2)` | ❌ still rejected — `call_builtin "any"` | ❌ | ❌ | ❌ |

Arithmetic / list / closure programs still emit `call_builtin` with
`type_hint = "any"` and stay rejected.  Subsequent path-A increments
will lower `(+ 1 2)` to typed `add_i64`, then `cmp_*`, then non-trivial
control flow.

#### Tests

- 3 new e2e tests in `tests/backend_compat.rs`:
  - `twig_int_literal_accepted_by_every_backend` — `42` validates on
    all four backends; `main.return_type == "i64"`.
  - `twig_bool_literal_accepted_by_every_backend` — same for `#t`.
  - `twig_arithmetic_still_rejected_in_increment_1` — pins down the
    current boundary; a future increment that types arithmetic must
    explicitly update this test.
- The existing `every_instruction_has_any_or_void_type_hint` test
  renamed to `every_instruction_has_known_type_hint` and updated to
  accept `"i64"` / `"bool"` / `"str"` in addition to `"any"` / `"void"`.
- 72 unit tests pass (was 71).

#### Compatibility

- Non-Twig callers unaffected.
- Downstream Twig tooling (twig-aot, twig-vm) all continue to pass —
  the new type hints are *stricter* than the old `"any"`, never
  broader.
- Pre-existing twig-module-driver `tw05*` self-compile tests fail on
  this branch *and* on `main` (a Windows file-path issue in the test
  fixture, unrelated to this PR).

## [0.14.0] — 2026-05-17

### Added (LANG72 — TW05-Q cross-module strict type checking)

- **`compile_program_with_externs_and_globals`** — new public function that
  accepts a `&HashMap<String, TwigKind>` of cross-module globals in addition
  to the existing extern-fn list.  When a `(typed strict)` module is compiled
  this function forwards the globals map to `check_program_with_globals` so
  that imported names from peer modules are visible during the type-check pass.

  This fixes a pre-existing regression where `compile_program_with_externs`
  called `check_program(program, None)` internally, which caused strict modules
  that imported names from other modules to always fail type checking with
  "unresolved variable" errors — even when the imports were correct.

  Called by `twig-module-driver` Phase 4 instead of `compile_program_with_externs`;
  the driver now passes each module's accumulated export globals from Phase 3.5.

## [0.13.0] — 2026-05-15

### Added (LANG58 — TW05-E string/char builtins)

- **13 string and character builtins added to `BUILTINS`** — these operations
  have been in `lispy-runtime` since LANG47 but were missing from the
  compiler's `BUILTINS` constant, so calls from Twig source were treated as
  user-function calls and failed with "unbound name" at runtime:
  - `string-length`, `string-ref`, `substring`, `string-append`
  - `string->number`, `string=?`, `string<?`, `string>?`
  - `char->integer`, `integer->char`
  - `char-alphabetic?`, `char-numeric?`, `char-whitespace?`

  These are required by `compiler/lexer.tw` (TW05-E) for scanning source text
  character-by-character using ASCII integer comparison.

## [0.12.0] — 2026-05-15

### Added (LANG57 — TW05-D string/symbol conversion builtins)

- **`"number->string"`, `"string->symbol"`, `"symbol->string"` added to
  `BUILTINS`** — these three conversions have been in `lispy-runtime` since
  LANG47 but were accidentally omitted from the compiler's `BUILTINS`
  constant.  Without this entry the compiler treated calls like
  `(number->string 42)` as user-function calls, which then failed with
  "unbound name" when no top-level define existed.  Adding them makes
  string↔number↔symbol conversions usable from any Twig source file,
  including the new `code/packages/twig/compiler/` data model modules.

- **`extern_fns` in `compile_module_tree` now covers record/union generated
  names** — the `twig-module-driver` Phase 3 pre-pass was extended to
  collect constructor, predicate, and accessor names from `Form::RecordDef`
  and `Form::UnionDef`.  This fixes "unbound name" errors when one Twig
  module calls record/union functions defined in another module.

---

## [0.11.0] — 2026-05-15

### Fixed (LANG57 — TW05-D prerequisite)

- **`compile_match`: `jmpif` → `jmp_if_false`** — The variant-arm lowering in
  `compile_match` previously emitted a non-standard three-operand opcode `"jmpif"`
  (`jmpif cond arm_label skip_label`).  This opcode was never registered in the VM
  dispatch table, causing every `(match …)` expression to fail at runtime with
  `UnsupportedOpcode("jmpif")`.
  
  Fixed by replacing it with the standard two-operand `jmp_if_false` pattern
  (identical to `compile_if`): when the tag comparison is false, jump to
  `skip_label`; when true, fall through to the arm body.  The now-redundant
  `label arm_label` instruction is also removed.

---

## [0.10.0] — 2026-05-14

### Added (LANG56 — Multi-File Module Driver)

- **`Compiler::with_extern_fns(&[&str]) -> Self`** — builder method that pre-registers
  extern function names in `fn_globals` before the compiler's own pre-pass runs.  Allows
  cross-module calls (`(double 21)` calling `double` from another `.tw` file) to compile
  to `call` instructions rather than failing with "unbound name".  The linker resolves the
  actual call targets.

- **`compile_program_with_externs(program, module_name, extern_fns)`** — public entry point
  for the module driver.  Equivalent to `compile_program` but pre-populates `fn_globals`
  with `extern_fns` before compiling.  Applies the same LANG49 type-check pre-pass as
  `compile_program`.

- **IIRExport population from `module_info`** — when a program carries a
  `(module name (export f1 f2 ...))` clause, the compiler now populates `IIRModule.exports`
  from `info.exports`, filtered to names that were actually compiled as top-level functions.
  Previously `exports` was always `vec![]`.

---

## [0.9.0] — 2026-05-14

### Added (LANG55 — Higher-Order List Operations)

- **BUILTINS expansion** — `"map"`, `"filter"`, `"fold-left"`, `"fold-right"` added
  to the `BUILTINS` constant.  The compiler now emits `call_builtin "map" fn_reg list_reg`
  (etc.) for these names rather than treating them as user-defined functions.
  The actual execution is handled by the new special-cased `exec_hof_*` handlers in
  `twig-vm` which can recurse into `dispatch` to call the supplied closure.

---

## [0.8.0] — 2026-05-14

### Added (LANG52 — stdlib completeness + LANG51 string literals)

#### LANG51: string literal lowering

- **`Expr::StrLit` → `const(Operand::Str(value)) : "str"`** — string literals compile to a
  `const` instruction with `Operand::Str` payload and `type_hint = "str"`.  The VM's
  `exec_const` handler (introduced in LANG47) materialises this as a `LangString` heap object.
- `Expr::StrLit` added to the leaf-atom arm of `free_vars.rs` (never a free variable).

#### LANG52: `let*` sequential bindings

- **`Expr::LetStar` → `compile_let_star`** — sequential bindings: each RHS is compiled in a
  scope extended by all prior names.  Each binding gets a fresh register allocated via
  `_move`; the body is compiled after all bindings are live.
- **`free_vars.rs` Expr::LetStar walk** — incremental bound-set extension mirrors the
  compiler's sequential scoping exactly (each name bound before the next RHS).

#### LANG52: `and` / `or` special forms (short-circuit)

- **`(and e₁ e₂ …)`** — intercepted in `compile_apply` before the builtin-resolution path.
  Lowered to: evaluate `e₁`, branch on `jmp_if_false`, evaluate tail with
  recursive `compile_and`, merge into shared result register via `_move`.
  `(and)` → `#t`; `(and e)` → `e`.
- **`(or e₁ e₂ …)`** — similar pattern.  `(or)` → `#f`; `(or e)` → `e`.
- Neither `and` nor `or` is in the `BUILTINS` constant — they never reach the
  `resolve_builtin` path.

#### LANG52: expanded BUILTINS constant

Added to the `BUILTINS: &[&str]` array (used for higher-order closure wrapping):
`<=`, `>=`, `modulo`, `remainder`, `quotient`, `not`, `boolean?`, `equal?`,
`list`, `list?`, `length`, `append`, `reverse`, `list-ref`, `assoc`,
`symbol-append`, `host/write_string`, `host/read_line`, `host/read_file`.

## [0.7.0] — 2026-05-14 (LANG51 — string literal lowering, included here)

*Note: 0.7.0 was the planned standalone LANG51 release; changes are rolled into 0.8.0
above since LANG52 depends on LANG51 and both land together.*

## [0.6.0] — 2026-05-14

### Added (LANG50 — Annotation-aware IIR emission)

- `compile_typed_source(source, module_name) -> Result<IIRModule, TwigCompileError>`
  — new compilation entry point that runs the LANG50 grammar-type-checker pass
  first and post-processes the resulting IIR to propagate concrete `type_hint`
  values (`"i64"`, `"bool"`, `"str"`, `"closure"`) on instructions whose source
  positions map to concretely-typed `AnnotatedNode`s.
- `build_hint_map` — traverses the `AnnotatedNode` tree to build a
  `HashMap<(line, col), &'static str>` of concrete hints.
- `apply_hints` — post-processes an `IIRFunction`'s instructions using the
  hint map and the function's `source_map` for position correlation.
- `set_function_type_status` — sets `IIRFunction::type_status` to
  `FullyTyped` / `PartiallyTyped` / `Untyped` based on the fraction of
  non-void instructions carrying a concrete type hint.
- 7 new unit tests in `tests` module:
  `typed_source_int_literal_hint`, `typed_source_bool_literal_hint`,
  `typed_source_nil_literal_hint`, `typed_source_untyped_fallback`,
  `typed_source_function_status_fully_typed`,
  `typed_source_strict_mode_type_error_returns_err`,
  `typed_source_off_mode_no_errors`.

### Dependencies added

- `type-declarations = { path = "../type-declarations" }`

### Backward compatibility

- `compile_source` and `compile_program` are **unchanged**.
- `FunctionTypeStatus` set by `set_function_type_status` only affects
  functions compiled via `compile_typed_source`; the existing path still
  emits `Untyped` everywhere.

---

## [0.5.0] — 2026-05-14

### Added (LANG49 — TW05-B type-check pre-pass)

Wires the new `twig-type-checker` crate as an optional pre-pass in
`compile_program`.

#### Behaviour

- `TypedMode::Strict`: if `check_program` returns `ok: false`, the first
  `TypeErrorDiagnostic` is wrapped in a `TwigCompileError` and returned
  as `Err` before any IIR is emitted.
- `TypedMode::Lenient`: type errors are printed as warnings to `stderr`
  (prefix `twig type warning (line:col): …`), then compilation proceeds.
- `TypedMode::Off` / no `module_info`: pre-pass skipped entirely —
  zero performance overhead for dynamic Twig programs.

#### Dependency added

- `twig-type-checker = { path = "../twig-type-checker" }` — the new
  TW05-B base type checker crate.

---

## [0.4.0] — 2026-05-14

### Added (LANG48 — TW05-A annotation erasure)

Implements the TW05-A bootstrap stage: typed Twig source compiles to
dynamic IIR by erasing all type annotations.  No type checker yet (that's
TW05-B/C); the compiler accepts typed programs and lowers them faithfully.

#### New `Compiler` field

- `variant_tags: HashMap<String, usize>` — populated during the pre-pass
  from every `Form::UnionDef`; consulted when lowering `Expr::Match` arms
  to determine variant integer tags for dispatch.

#### New form lowering

- `Form::TypeAlias` — erased (no-op, type aliases are compile-time only).
- `Form::RecordDef` — lowered via `emit_record_def`:
  - Constructor function `Name(f0, f1, …)` using a right-fold `cons` chain.
  - Positional accessor `name-field-i(r)` using `car` of `cdr^i`.
  - Type predicate `name?(v)` using `pair?`.
- `Form::UnionDef` — lowered via `emit_union_def`:
  - Per-variant constructor `Variant(f0, …)` — prepends the zero-based
    integer tag via `cons`.
  - Per-variant predicate `Variant?(v)` — checks `(= (car v) tag)`.
  - Per-variant field accessor `variant-field-k(v)` using `car` of
    `cdr^(k+1)` (skip the tag slot).

#### New expression lowering

- `Expr::Match` — lowered via `compile_match` to a `jmpif`/`label`/`jmp`
  chain:
  - Scrutinee evaluated once into a fresh register.
  - `Variant` arm: test `(= (car scrutinee) tag)`, bind fields via
    `car`/`cdr` chains, evaluate body.
  - `Binding` arm: bind scrutinee to name, evaluate body.
  - `Wildcard` arm: evaluate body directly.
  - After all arms: fall through to `nil`.

#### Annotation erasure extension

- `TypeAnnotation::Opaque(_)` → `TypeAnnotation::Any` in the annotation
  map.  Any type expression that isn't a LANG23 shape is silently erased
  to the `Any` (untyped) refinement, preserving backward compat.

### Tests

- Regression tests confirm `alloc_closure` / `call_closure` emission is
  unchanged by LANG48 changes.
- New compiler tests for record def erasure (constructor + accessor +
  predicate IIR shapes), union def erasure (tagged variants), and match
  expression lowering (variant/binding/wildcard dispatch chains).

---

## [0.3.0] — 2026-05-12

### Changed (LANG34 — Emit alloc_closure / call_closure)

Three emission sites updated to use the LANG34 first-class closure opcodes:

#### Lambda allocation (`compile_anonymous_lambda`)

```
BEFORE:
  %s0 = const("__lambda_N")          ← string_arg indirection
  %c0 = call_builtin("make_closure", %s0, caps...) : "any"

AFTER:
  %c0 = alloc_closure(Str("__lambda_N"), caps...) : "closure"
```

No preceding `const` instruction is emitted; `fn_name` is now an inline
`Operand::Str` in `srcs[0]`.

#### Top-level function as value (`compile_var_ref` / fn_globals)

```
BEFORE:
  %s0 = const("fn_name")
  %fnref = call_builtin("make_closure", %s0) : "any"

AFTER:
  %fnref = alloc_closure(Str("fn_name")) : "closure"
```

#### Indirect call (`compile_apply`, indirect path)

```
BEFORE:
  %r = call_builtin("apply_closure", %handle, args...) : "any"

AFTER:
  %r = call_closure(%handle, args...) : "any"
```

The `string_arg` helper is retained for `global_set`/`global_get`/`make_symbol`
which still use the const-via-Var register convention.

#### Tests updated

Three tests renamed/updated to assert the new opcode forms:
- `anonymous_lambda_emits_make_closure` → `anonymous_lambda_emits_alloc_closure`
- `closure_call_uses_apply_closure` → `closure_call_uses_call_closure`
- `fn_globals_can_be_passed_as_values` (assertion updated)

---

## [0.2.1] — 2026-05-11

### Fixed (LANG33 — Module System)

- Added `exports: Vec::new(), imports: Vec::new()` to the `IIRModule { ... }`
  struct literal in `compiler.rs` (`compile_module`).  Required by the new
  LANG33 fields on `IIRModule`; the workspace `cargo build` enforces this.

---

## [0.2.0] — 2026-05-04

### Added (LANG23 PR 23-E — emit RefinedType annotations into IIR)

- `type_annotation_to_refined_type(ann: &TypeAnnotation) -> RefinedType`:
  conversion function that bridges the parser's `TypeAnnotation` enum to
  `lang-refined-types::RefinedType`.  Matches all five `TypeAnnotation` variants:
  - `UnrefinedInt` → `RefinedType::unrefined(Kind::Int)`
  - `UnrefinedBool` → `RefinedType::unrefined(Kind::Bool)`
  - `Any` → `RefinedType::unrefined(Kind::Any)`
  - `RangeInt { lo, hi }` → `RefinedType::refined(Kind::Int, Predicate::Range { lo, hi, inclusive_hi: false })`
  - `MembershipInt { values }` → `RefinedType::refined(Kind::Int, Predicate::Membership { values })`
- `compile_top_level_lambda` now populates `IIRFunction::param_refinements` and
  `IIRFunction::return_refinement` from the `Lambda` node's annotation fields.
- `lang-refined-types` added as a dependency.
- Round-trip tests in `lib.rs` (PR 23-E section, 7 new tests):
  - `ranged_int_param_annotation_round_trips_to_iir`
  - `unrefined_int_param_annotation_round_trips`
  - `return_annotation_round_trips_to_iir`
  - `multiple_annotated_params_lockstep`
  - `unannotated_function_has_no_refinement_fields`
  - `annotation_does_not_change_existing_type_hints`
  - `source_map_lockstep_holds_for_annotated_functions`

## [0.1.0] — 2026-04-29

### Added

- Initial Rust implementation of the Twig → InterpreterIR compiler
  (TW00).  Mirrors the Python reference at
  `code/packages/python/twig/src/twig/compiler.py`.
- `compile_source(source, module_name)` — lex + parse + compile in one
  call.
- `compile_program(program, module_name)` — compile a parsed
  `twig_parser::Program` into an `IIRModule`.
- `Compiler` struct — one-program, mutable lowering driver.
- Pre-pass classification of top-level defines into
  `fn_globals` (lambda RHS) and `value_globals` (non-lambda RHS) so
  the main pass can resolve names before walking any bodies.
- Per-function compilation context (`FnCtx`) tracking accumulated
  instructions, in-scope locals, and fresh-name counters for
  registers and labels.
- Free-variable analysis (`free_vars` module) — Scheme-`let`-aware
  walk that returns captures in stable insertion order.
- Apply-site dispatch decided at compile time:
  - top-level user function → `call <name>, ...args`
  - builtin → `call_builtin <name>, ...args`
  - everything else → `call_builtin "apply_closure", h, ...args`
- Lambda handling: each anonymous lambda becomes a synthesised
  top-level `IIRFunction` named `__lambda_N`; captured variables
  appear as the *leading* parameters in the order produced by
  `free_vars`; the call site emits `call_builtin "make_closure"
  <fn_name> <captures...>`.
- `if` lowering to `jmp_if_false` + two-branch `_move`s + final
  `label`s — preserves value type across branches (booleans are not
  coerced to integers).
- `let` lowering with mutually-independent bindings, copied into
  named registers via `_move`.
- `begin` returns the value of the last expression.
- Top-level value defines lower to `call_builtin "global_set" name
  value`; references to value globals lower to
  `call_builtin "global_get" name`.
- Top-level function names in non-call position wrap in a 0-capture
  `make_closure`; builtin names wrap in `make_builtin_closure`.
- Synthesised `main` function holds top-level value defines and bare
  expressions in source order.  Programs with no bare expression
  return `nil` via `call_builtin "make_nil"`.
- Every emitted instruction carries `type_hint = "any"` (or
  `"void"` for control-flow ops); functions are tagged
  `FunctionTypeStatus::Untyped`.
- `TwigCompileError { message, line, column }` with
  `From<TwigParseError>` so callers handle a single error type at
  the public entry point.
- `MAX_COMPILE_DEPTH = 256` cap in `compile_expr` — defence-in-depth
  against stack overflow on hand-built ASTs (the parser already
  caps source-paren-depth at 64 before reaching the compiler).
- 45 unit tests verifying instruction shape, dispatch decisions,
  closure layout, recursion, and error paths.
