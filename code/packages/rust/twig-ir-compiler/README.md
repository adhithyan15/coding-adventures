# twig-ir-compiler

**TW00** — Twig (Lisp-precursor) → InterpreterIR (IIR) compiler in Rust.

Third stage of the Rust [Twig](../../specs/TW00-twig-language.md) pipeline:

```
Twig source --> [twig-lexer] --> tokens --> [twig-parser::parse_to_ast] --> GrammarASTNode (CST) --> [twig-ir-compiler] --> IIRModule
```

## Input: the parse tree (LANG84 decision 6)

The compiler's only input is the generic `GrammarASTNode` that the
grammar-driven lexer and parser build from `code/grammars/twig/twig.tokens`
and `twig.grammar`. There is no typed AST on the IIR path: since v0.46 the
compiler no longer lowers `twig_parser::Program` (the typed AST that
`twig_parser::parse` lifts out of the CST, and which stays available to the
type checker, formatter and LSP crates).

- `src/cst.rs` — **borrowed views** over the CST. `ExprNode::kind` classifies
  one `expr` node (`IntLit`, `If`, `Apply`, …) and hands back its children as
  further unclassified handles. Nothing is copied and no tree is built.
  Sugar (`'x` = `(quote x)`; `(define (f x) …)` = `(define f (lambda (x) …))`)
  is expanded by the views on the fly.
- `src/check_cst.rs` — **`check_cst`**, the validation pass that rejects
  exactly what the old typed-AST extractor rejected (i64 overflow,
  `(typed bogus)`, a two-expression value `define`, nesting past
  `MAX_AST_DEPTH`, malformed hand-built nodes), with the same messages,
  positions and order.
- **The TW05-B gate** for `(typed lenient|strict)` modules hands the CST to
  `twig_type_checker::check_cst_with_globals` and reads back only position-keyed
  diagnostics; nothing the checker computes feeds IIR emission.

The emitted IIR is byte-identical to the v0.45 typed-AST pipeline.

| Entry point | Input |
|-------------|-------|
| `compile_source(src, name)` | source text (`parse_to_ast` + `compile_ast`) |
| `compile_ast(&cst, name)` | a `program` CST from `twig_parser::parse_to_ast` |
| `compile_ast_with_externs(&cst, name, externs)` | … plus cross-module function names (LANG56) |
| `compile_ast_with_externs_and_globals(&cst, name, externs, globals)` | … plus imported type-checker globals (LANG72) |
| `compile_typed_source(src, name)` | source text, with LANG50 type-hint propagation |

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
`(string<? "..." "...")`, `(string>? "..." "...")`, `(substring "..." i j)`,
and `(string-length (string-append "..." "..."))`
lower to typed E4 string
metadata ops: `str_const` for each direct literal, then `str_len`, `str_index`,
`str_eq`, `str_cmp`, `str_slice`, or `str_concat` for the result path. The same E4
lowering also handles immutable top-level string values, so `(define s "ABC")
(string-ref s 2)` and named-string `string-append`/`string=?` proofs avoid the
dynamic builtin path.
Lexical `let`/`let*` string literal bindings use the same typed registers, so
local strings can feed `string-ref`, `string-length`, `string=?`, `string<?`,
`string>?`, and
`string-append` without dynamic builtins; a `string-append` result can also feed
`string-ref` directly through `str_concat` -> `str_index`, and `string-length`
can compute a typed arithmetic index that feeds `str_index`. `substring` over a
known string and known index expressions lowers to `str_slice`, so its result
can feed `string-ref` through `str_index`. Direct top-level functions whose
final expression already infers a concrete type also preserve that return type
at later direct call sites, so `(define (strlen) (string-length "HELLO"))
(strlen)` returns through a typed `call [i64]`. Bare `str`/`string` parameter
annotations on top-level functions also seed concrete `str` IIR params, so
`(define (strlen (s : str)) (string-length s)) (strlen "HELLO")` lowers without
dynamic string builtins. Conservative `main`-level direct-call evidence can also
seed an otherwise-unannotated top-level string parameter, so `(define (strlen s)
(string-length s)) (strlen "HELLO")` follows the same typed path without
synthesizing refinement annotations. Reassignable strings, captured strings,
unobserved/conflicting or closure-derived string parameters, and broader dynamic
string values remain follow-up work.

Typing a parameter `str` promises that every value that reaches it is a
string, so the evidence pass walks the whole program. Only a direct call in
`main` can establish `str`. The parameter falls back to `any` if any of these
is true:

- A direct call anywhere else (another function body, a lambda, a `match`
  arm) passes a non-string.
- A call has the wrong number of arguments.
- The function is used as a value, as in `(define h strlen)`,
  `(let ((g strlen)) …)` or `(map strlen xs)`, so its callers cannot be known.
- The program is module code: it has a `(module …)` declaration, or it was
  compiled with externs by `twig-module-driver`. The driver makes every
  module's functions, private ones included, visible to every other module,
  so no single module sees all the call sites. Module code gets no
  call-site inference. Explicit `(x : str)` annotations still apply.

Twig remains dynamically typed, so functions keep `type_status = Untyped` and
dynamic paths still carry `type_hint = "any"`. The LANG-FULL fast paths above
stamp concrete hints only where source-local evidence makes the type
unambiguous. The vm-core profiler observes runtime types; the JIT specialises
later.

## Apply-site dispatch

The compiler decides at compile time. The rows are tried top to bottom, so
the first one that matches wins:

| Function position           | Emitted IIR                                 |
|-----------------------------|---------------------------------------------|
| A name bound in the current lexical scope (parameter, `let`/`let*` binding, lambda parameter, `match` binding) | `call_closure <local>, ...args`, even if a top-level function, extern or builtin has the same name |
| Top-level user fn or extern | `call <name>, ...args`, using the known return type when the function was already lowered in source order |
| Typed arithmetic (`+`,`-`,`*`,`/`) on `i64` args | a chain of typed `add`/`sub`/`mul`/`div` |
| Direct literal, immutable top-level, lexical-local, annotated top-level-parameter, or direct-call-inferred top-level-parameter string metadata (`string-length`, `string-ref`, `substring`, `string=?`, `string<?`, `string>?`, `string-append`) | `str_const` + `str_len`/`str_index`/`str_slice`/`str_eq`/`str_cmp`/`str_concat`, with typed index arithmetic for `string-ref` and `substring` bounds |
| Builtin (`cons`, `<`, …)    | `call_builtin <name>, ...args`              |
| Anything else (e.g. `((f 1) 2)`) | `call_closure h, ...args`              |

Top-level recursion stays on the fast `call` path; only locals holding closures pay the indirect cost.

### Lexical scope wins

Twig is lexically scoped, like Scheme: a use of a name refers to the
innermost binding of that name. This holds in call position as well as in
value position:

```scheme
(define (helper x) (* x 100))
(define (f helper) (helper 1))   ; this `helper` is f's parameter
(f (lambda (x) (+ x 1)))         ; => 2
```

Before issue #18435 was fixed, the compiler checked "is this a top-level
function?" before "is this a local?" in call position. The body of `f`
compiled to `call helper`, a direct call to the global function, and the
program returned 100. Every lookup now checks the lexical scope first:

- **Calls.** A shadowing local is called with `call_closure`. The global's
  direct `call`, its known return type and its known parameter types are not
  used, because the callee is a different function.
- **Lambda captures.** If a lambda uses a local that shares a global's name,
  the lambda captures the local. Free-variable analysis does not skip the
  name as a global.
- **E4 string fast paths.** If a local is named `string-append`, `substring`
  or `string-length`, calls to it are not lowered to `str_concat`, `str_slice`
  or `str_len`.
- **Call-site string inference in `main`.** A call to a shadowing `let`
  binding tells the compiler nothing about the parameters of the top-level
  function with the same name.
- **Type facts belong to their binding.** Each binding has its own
  register (below), and `var_types` is keyed by register, so an inner
  string `x` cannot make a later `(string-length x)` on an integer parameter
  lower to `str_len`, and an inner `s` bound to 5 does not read the outer
  string's alias register.

### Every binding owns its register

An IIR register has a name, and a Twig local *is* a register. If the
register were simply the source name, two bindings of the same name would
share it, and the inner one would overwrite the outer (#18439):

```scheme
(define (f x) (begin (let ((x 5)) x) x))
(f 1)   ; must be 1 — the parameter
```

```text
one register per NAME (old)      one register per BINDING (now)
mov x ← 5     ; parameter lost    mov __r1_x ← 5   ; parameter untouched
ret x         ; → 5               ret x            ; → 1
```

The wrong value was the mild symptom. The compiler stamps static types on
registers and lowers to typed ops (`str_len`, `add`, …) that the native
backends trust, so a shared register could hand a typed op a value of the
wrong kind: `(let* ((x "abc") (y x) (x 5)) (string-length y))` ran `str_len`
on 5.

So `FnCtx` keeps a **name → stack of registers** map (`scope`). Every
binder pushes and pops it: function and lambda parameters, closure captures,
`let`, `let*` and both kinds of `match` binding. Every reader goes through
`FnCtx::lookup` — value references, call-position checks, closure captures,
the E4 type queries.

| Binding                                 | Register                                  |
|-----------------------------------------|-------------------------------------------|
| first binding of `x` in the function    | `x` (unchanged IR for programs without repeated names) |
| any later binding of `x`, nested or sibling | `__r1_x`, `__r2_x`, … (per-function counter) |
| E4 string binding (`(let ((s (string-append a b))) …)`) | no register of its own: `s` reads the producer's register |

Because each binding register is written by exactly one instruction, its
recorded type stays true for its whole life — even after its scope closes,
when `(let ((x 1)) x)`'s result may still be waiting to be used. No type
has to be saved or restored at scope boundaries. A per-function `claimed`
set keeps all register names collision-free: a user local spelled
`__r1_x` or `_n1` simply pushes the generated or temporary name to the next
free one.

`and` and `or` are special forms, not bindings, so a local cannot shadow
them.

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

## Reserved names

Every name beginning with two underscores (ignoring ASCII case, in any
`/`-separated segment) is reserved for the implementation, as in C. The
known owners are `__lisp_`, `__dyn_`, `__lambda_`, `__twig_`, `__gc_`,
`__aot_` and `__clo_` (`RESERVED_PREFIXES`), and an error names the one a
refused name uses; any other `__` name is refused as `"__"`. They belong to
the functions later lowering passes add to the module, to the native
runtime's symbols, to the functions this compiler lifts each `(lambda …)`
into (`__lambda_0`, …), and to **registers**: a parameter or `let` name
becomes an IIR register verbatim, in the same function as the registers the
compiler and later passes invent (`__aot_str0_len`, `__clo_argnil1`,
`iir-to-llvm`'s `__tobool4` …). This compiler's own temporaries are
`__t{n}_{what}` and its labels `__l{n}_{what}`, so no program name can
reach them. (They were `_n1`, `_else1`, …, which a program could spell:
`(let ((_n2 40)) (+ _n2 2))` computed 4.) A program may not define, bind or
reference a reserved name:

```text
(define (__lisp_equal a b) 42)      → TwigCompileError at 1:10: the name "__lisp_equal" is reserved
(define (__twig_print_i64 n) 0)     → refused: native `print` would have called it
(define (__lambda_0 y) …)           → refused: a lifted lambda would have shared its name
(let ((__aot_str0_len 9)) …)        → refused: twig-aot's string length register
(let ((_n2 40)) (+ _n2 2))          → 42: one underscore is the program's
'__lisp_equal                       → fine: a quoted symbol is data, not a name
```

The generated lambda names stay `__lambda_N` rather than using a character
Twig cannot spell. A generated name reaches every backend's symbol table,
and no character is legal unquoted in all of them: the JVM forbids `.` in a
method name, and LLVM quotes anything outside `[-a-zA-Z$._0-9]`. Reserving
the prefix changes no symbol.

`RESERVED_PREFIXES`, `RESERVED_NAMESPACE` and `reserved_prefix` are exported
for tools that want the same rule.

## Usage

```rust
use twig_ir_compiler::compile_source;

let module = compile_source(
    "(define (square x) (* x x)) (square 7)",
    "demo",
).unwrap();

assert_eq!(module.entry_point.as_deref(), Some("main"));
// One IIRFunction for `square`, one for `main`.

// Or, from a parse tree you already hold:
let cst = twig_parser::parse_to_ast("(define (square x) (* x x)) (square 7)").unwrap();
let same = twig_ir_compiler::compile_ast(&cst, "demo").unwrap();
assert_eq!(format!("{same:?}"), format!("{module:?}"));
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
`tests/cst_walk.rs` pins the LANG84 contract: a source check that no compiler
file names the typed AST, the `compile_ast*` entry points, `check_cst` errors
matching the extractor's on hand-built malformed trees (including the depth
guard), and the sugar / type-gate behaviour.

## Where it fits in the stack

```
LANG01  interpreter-ir         ← IIRModule format
LANG02  vm-core                ← executes IIRModule
LANG03  jit-core               ← JIT (hot fn → native)
TW00    twig-lexer             ← tokens
TW00    twig-parser            ← GrammarASTNode CST (+ typed AST for tooling)
TW00    twig-ir-compiler       ← THIS CRATE
TW02    twig-jvm-compiler      ← Twig → JVM .class (separate path)
TW03    full Lisp surface + GC ← cross-backend roadmap
```
