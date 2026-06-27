# twig-ir-compiler

**TW00** — Twig (Lisp-precursor) → InterpreterIR (IIR) compiler in Rust.

Third stage of the Rust [Twig](../../specs/TW00-twig-language.md) pipeline:

```
Twig source --> [twig-lexer] --> tokens --> [twig-parser] --> AST --> [twig-ir-compiler] --> IIRModule
```

## What it produces

For each Twig source program, this crate emits an [`IIRModule`](../interpreter-ir) containing:

| Function shape                  | Origin                                |
|---------------------------------|---------------------------------------|
| One per `(define (name args) body+)` | top-level user functions          |
| One per `(lambda ...)`         | gensym'd `__lambda_0`, `__lambda_1`, … |
| `main` (always present)         | top-level value defines + bare exprs  |

Anonymous lambdas have their captured free variables prepended to the parameter list (in stable insertion order); the `make_closure` call site passes them in matching order. Bare top-level expressions accumulate into `main`, with the *last* expression's value becoming `main`'s return.

Top-level value defines lower one of two ways (TW2/E4). A value define that is **not captured by any lambda** is read only from `main`, so its statically-typed (`i64`/`bool` and immutable top-level `str`) value is kept in a `main` register and reads return it directly — fully typed, accepted by every code-gen backend. A value define **captured by a closure** (read inside a lambda body, which compiles to a separate function) stays on the host global table via `call_builtin "global_set" name value` / `global_get`, as does a top-level forward reference.

Literal `(string-length "...")`, `(string-ref "..." i)`, `(string=? "..." "...")`,
and `(string-length (string-append "..." "..."))` lower to typed E4 string
metadata ops: `str_const` for each direct literal, then `str_len`, `str_index`,
`str_eq`, or `str_concat` for the result path. The same E4 lowering also handles
immutable top-level string values, so `(define s "ABC") (string-ref s 2)` and
named-string `string-append`/`string=?` proofs avoid the dynamic builtin path.
Reassignable strings, captured strings, and string slots introduced by `let`
remain follow-up work.

All emitted instructions carry `type_hint = "any"` because Twig is dynamically typed. Functions therefore have `type_status = Untyped`. The vm-core profiler observes runtime types; the JIT specialises later.

## Apply-site dispatch

The compiler decides at compile time:

| Function position           | Emitted IIR                                 |
|-----------------------------|---------------------------------------------|
| Top-level user fn           | `call <name>, ...args`                      |
| Typed arithmetic (`+`,`-`,`*`,`/`) on `i64` args | a chain of typed `add`/`sub`/`mul`/`div` |
| Direct literal or immutable top-level string metadata (`string-length`, `string-ref`, `string=?`, `string-append`) | `str_const` + `str_len`/`str_index`/`str_eq`/`str_concat` |
| Builtin (`cons`, `<`, …)    | `call_builtin <name>, ...args`              |
| Anything else (locals etc.) | `call_builtin "apply_closure", h, ...args`  |

Top-level recursion stays on the fast `call` path; only locals holding closures pay the indirect cost.

### Typed arithmetic fold

Scheme arithmetic is variadic. When every argument is statically `i64`, an
arithmetic call folds to a **left-associated chain of typed binary CIR ops** —
`(+ 10 20 12)` becomes `r1 = add 10, 20; r2 = add r1, 12` — which the
IIR-to-{llvm,wasm,jvm,clr} backends accept directly. A call with any
dynamically-typed argument, or a chained comparison like `(< a b c)` (a
predicate, not a fold), stays on the dynamic `call_builtin` path.

## Builtins

Recognised by name at apply sites and at `VarRef` resolution:

```
+  -  *  /  =  <  >
cons  car  cdr
null?  pair?  number?  symbol?
print
```

Builtin references in non-call positions wrap into a `make_builtin_closure` so they can be passed as values to higher-order functions.

## Usage

```rust
use twig_ir_compiler::compile_source;

let module = compile_source(
    "(define (square x) (* x x)) (square 7)",
    "demo",
).unwrap();

assert_eq!(module.entry_point.as_deref(), Some("main"));
// One IIRFunction for `square`, one for `main`.
```

## Encoding string operands

`interpreter_ir::Operand` has `Var`, `Int`, `Float`, `Bool` — no dedicated `String` variant. Where the IR semantically needs a string literal (e.g. the function name passed to `make_closure`), we materialise it via a `const` instruction whose source operand is `Operand::Var(literal_text)`. The `vm-core` `const` handler stores the literal verbatim. See the module-level comment in `src/compiler.rs` for the full rationale.

## LANG23 PR 23-E: Refinement type annotation emission

When a Twig function carries LANG23 annotation syntax, the compiler lowers it
into the corresponding `IIRFunction` fields:

```scheme
; Both params and return type annotated:
(define (clamp-byte (x : int) -> (Int 0 256)) x)
```

produces an `IIRFunction` where:
- `params` = `[("x", "any")]` (unchanged — Twig is still dynamically typed)
- `param_refinements` = `[Some(RefinedType::unrefined(Kind::Int))]`
- `return_refinement` = `Some(RefinedType::refined(Kind::Int, Range(0, 256)))`

The refinement checker (`lang-refinement-checker`) reads these fields to
discharge proof obligations.  Callers that don't use LANG23 syntax see zero
change — annotation fields default to empty/`None`.

## Tests

```bash
cargo test -p twig-ir-compiler
```

56+ unit tests covering every literal, every form, apply-site dispatch, free-variable
analysis, top-level recursion, mutual recursion, error paths, and 7 LANG23 round-trip
tests verifying that annotations survive the parse → compile → IIR pipeline.

## Where it fits in the stack

```
LANG01  interpreter-ir         ← IIRModule format
LANG02  vm-core                ← executes IIRModule
LANG03  jit-core               ← JIT (hot fn → native)
TW00    twig-lexer             ← tokens
TW00    twig-parser            ← typed AST
TW00    twig-ir-compiler       ← THIS CRATE
TW02    twig-jvm-compiler      ← Twig → JVM .class (separate path)
TW03    full Lisp surface + GC ← cross-backend roadmap
```
