# TW00 — Twig: a Lisp precursor on the LANG VM

## Overview

Twig is a tiny purely-functional, S-expression language that runs on
`vm-core`.  It is explicitly designed as a **precursor to a full Lisp
implementation** — the surface and semantics are a strict subset that
will grow over subsequent specs.

The two motivating goals:

1. **Add a host-side heap and GC primitives** to the LANG VM
   infrastructure.  Brainfuck's tape is a flat byte array; Twig
   introduces *cons cells* — variable-sized heap objects with parent
   references.  The GC layer that emerges here will eventually serve
   every other LANG-pipeline language that needs heap allocation.
2. **Drive a multi-target compiler experience.**  Twig will eventually
   compile to BEAM (Erlang's VM) in addition to running through
   `vm-core`.  Designing the language with that future in mind keeps
   the semantics aligned (immutability, atoms-as-symbols, lists-as-cons,
   tail-call recursion).

## Roadmap

| Spec | Scope                                                      |
|------|------------------------------------------------------------|
| TW00 | This spec — language v1, runs through `vm-core`.           |
|      | **Includes full closures** via host-side closure objects.  |
| TW01 | `gc-core` package, native IIR heap ops, mark-sweep GC,     |
|      | `letrec`, cycle handling.                                  |
| TW02 | Twig → BEAM bytecode emission (direct, no Erlang source).  |
|      | Mirrors how `wasm-backend` produces WASM bytes natively.   |

This spec is **TW00 only**.  Everything below is the v1 surface.

## v1 surface

```
program     ::= form*
form        ::= define | expr
define      ::= ( "define" name expr )
              | ( "define" ( name name* ) expr+ )       ; sugar for define-lambda

expr        ::= literal | name | quote | if | let | begin | lambda | apply

literal     ::= integer | "#t" | "#f" | "nil"
quote       ::= "'" name | ( "quote" name )
if          ::= ( "if" expr expr expr )
let         ::= ( "let" ( ( name expr )* ) expr+ )
begin       ::= ( "begin" expr+ )
lambda      ::= ( "lambda" ( name* ) expr+ )            ; v1: top-level only
apply       ::= ( expr expr* )

builtin     ::= "+" | "-" | "*" | "/" | "=" | "<" | ">"
              | "cons" | "car" | "cdr"
              | "null?" | "pair?" | "number?" | "symbol?"
              | "print"
```

### First-class closures — **included in v1**

Lambdas can appear anywhere and capture lexical variables.  The classic
adder example just works:

```scheme
(define (adder n) (lambda (x) (+ x n)))
(define add5 (adder 5))
(print (add5 3))   ; 8
```

How: each `lambda` becomes a fresh top-level `IIRFunction` (the compiler
gensyms a unique name).  At the lambda's source location the compiler
performs a free-variable analysis and emits

```
call_builtin "make_closure" "lambda_<gensym>" capt1 capt2 ...
```

returning a closure *handle* (an integer pointing into the host
`Heap`'s closure table).  Applying a value that's a closure handle —
i.e. when the call's function position is a local variable, not a
top-level name — emits

```
call_builtin "apply_closure" handle arg1 arg2 ...
```

The host's `apply_closure` looks up `(fn_name, captured)` from the
heap and re-enters `vm.execute(module, fn=fn_name, args=captured + user_args)`.

This requires **zero changes to vm-core or IIR** — closures sit
entirely on the `call_builtin` seam.  The trade-off is that every
closure invocation pays a Python callback plus a `vm.execute()`
re-entry; for an educational interpreter that is fine.  Promoting
`apply_closure` to a native `call_indirect` IIR op is a future
optimization, not a v1 concern.

### Apply-site dispatch (compile-time decision)

The compiler picks the call shape at compile time, not runtime:

| Function position           | Emitted IIR                               |
|-----------------------------|-------------------------------------------|
| Top-level name              | `call <name>, ...args`  (direct)          |
| Local variable / parameter  | `call_builtin "apply_closure", v, ...args`|
| Builtin name (`+`, `cons`)  | `call_builtin "<name>", ...args`          |

This means top-level recursion stays on the fast path; only locals
holding closures pay the indirect cost.

### Recursion

Top-level `define` produces a callable name in the global function table.
Recursion works the natural way:

```scheme
(define (fact n)
  (if (= n 0) 1 (* n (fact (- n 1)))))

(print (fact 5))   ; 120
```

Mutual recursion works too (functions can reference each other by name).

### `letrec`?

Not in v1.  `letrec` introduces *local* recursive bindings and would
require closure objects with self-reference — a TW01 / TW03 concern.

### Macros?

Out of scope.  No quasiquote, no `defmacro`, no `syntax-rules`.  This is
deliberately deferred — macros add real lexer/parser complexity (special
quote forms, hygienic expansion) and aren't required for a Lisp-precursor
that already proves out the GC and dispatch layers.

## Type model

Twig values come in two kinds:

| Kind           | Examples                | IIR representation                  |
|----------------|-------------------------|-------------------------------------|
| **Immediate**  | integers, booleans, nil | tagged ints (see below)             |
| **Heap**       | cons cells, symbols     | integer handle into the host `Heap` |

### Tagged immediates

Twig's IIR runtime uses Python `int` values everywhere.  We tag them so
the runtime can tell numbers from heap handles from booleans:

```
bit pattern        | value
-------------------|------------------
... xxxx xxx0      | integer (low bit clear)
0000 0001          | nil  (== integer 1, never a real number)
0000 0011          | #f   (== integer 3)
0000 0101          | #t   (== integer 5)
... xxxx 1001      | heap handle (low nibble = 0b1001)
```

The exact tagging scheme matters less than *consistency* — TW00 just
needs to round-trip values through `call_builtin` boundaries without
losing kind information.  TW02 (BEAM) and TW03 (mark-sweep) will
formalise the encoding.  For V1 we use a small dataclass on the host
side (`HeapHandle`) and let `vm-core`'s builtin registry pass them
through without conversion.

### Symbols

Symbols (`'foo`) are heap-interned strings.  The `Heap` keeps a
`symbols: dict[str, int]` table; `quote` of a name returns the existing
handle (or allocates one fresh).  Two `'foo`s compare `eq?`.

### Cons cells

A cons cell is a 2-slot heap object: `(car, cdr)`.  Both slots hold
arbitrary Twig values (immediates *or* handles).  `cons` allocates,
`car`/`cdr` read.

## Pipeline

```
Twig source
   │
   ▼  twig.parse_twig          (this spec — lexer + parser → AST)
ASTNode tree
   │
   ▼  twig.compile_to_iir      (this spec — AST → IIRModule)
IIRModule
   │
   ▼  TwigVM.run               (this spec — vm-core wrapper)
   │   ├── vm-core executes IIR
   │   └── call_builtin → Heap (cons / car / cdr / make_symbol / print)
program output (printed bytes + final value)
```

## Compiler details

### Top-level functions

A top-level `(define (f x y) body)` becomes one `IIRFunction` named
`f` with parameters `[("x", "any"), ("y", "any")]` and `return_type =
"any"` (Twig is dynamically typed).  Top-level *value* defines
`(define x expr)` are assembled into a synthesized `_init` function
that runs before `main`; `main` is a synthesized function holding any
top-level expressions in source order.

### Builtin dispatch

Arithmetic and predicates are emitted as `call_builtin` with the
operator name as `srcs[0]`:

```
(+ a b)       →  call_builtin "+", a, b   → result
(cons a b)    →  call_builtin "cons", a, b → handle
(car p)       →  call_builtin "car", p    → element
(null? x)     →  call_builtin "null?", x  → bool
```

The `TwigVM` registers the matching host callables.

### Name resolution: lexical scope wins

Twig is lexically scoped, like Scheme. A name refers to its innermost
binding. The compiler resolves every name in the same order, whether the
name is used as a value (`helper`) or called (`(helper 1)`):

| Order | Binding                                                         | Call `(name args…)` lowers to |
|-------|-----------------------------------------------------------------|-------------------------------|
| 1     | Lexical: parameter, `let`/`let*` binding, lambda parameter, `match` binding, or a variable a lambda captured from an enclosing scope | `call_closure name, args…`    |
| 2     | Top-level function `define`, or an extern imported from another module | `call name, args…`            |
| 3     | Top-level value `define`                                        | `call_closure` on its value   |
| 4     | Builtin (`+`, `cons`, `string-append`, …)                       | `call_builtin name, args…` (or a typed op) |

`and` and `or` are special forms and are not in this table. A local cannot
shadow them.

There is one known gap between rows 3 and 4, tracked as issue #18438. A
top-level *value* `define` that reuses a builtin's name, such as
`(define car 5)`, wins in value position. In call position the builtin still
wins. This rule is about the lexical row (row 1), which always wins.

So a local hides a global, an extern or a builtin with the same name for
the whole of the local's scope:

```scheme
(define (helper x) (* x 100))
(define (f helper) (helper 1))   ; calls the parameter, not the global
(f (lambda (x) (+ x 1)))         ; => 2
```

A shadowed global also gives up its static facts at that call site: its
direct `call`, its known return type and its parameter types. Free-variable
analysis also treats the name as local, so a nested lambda captures the
local and does not reach the global. (Before issue #18435 was fixed, the
compiler checked rows 2 and 4 before row 1 in call position, and this
program returned 100.)

### Type hints

Every emitted IIR instruction carries `type_hint = "any"` because Twig
is dynamically typed.  This deliberately exercises the
`UNTYPED`/`PARTIALLY_TYPED` branches of the JIT pipeline once we wire
them up later.

### `if`

```
(if c t e)
```

→ standard `jmp_if_false` / `jmp` over labels.  Truthiness: only `#f`
and `nil` are false; everything else is true.  This matches Scheme.

### `let`

`(let ((x e1) (y e2)) body)` → evaluate `e1` into `x`, `e2` into `y`,
then evaluate `body`.  Bindings are mutually independent (no
sequential like `let*`).  Scope is lexical: the `body` sees `x`/`y`
plus the surrounding scope.

Implementation: each binding gets **a register of its own** (#18439).
Reusing the source name as the register does *not* handle scoping: two
bindings of the same name would share one slot, and the inner binding
would overwrite the outer one, so
`(define (f x) (begin (let ((x 5)) x) x)) (f 1)` would return 5 instead
of 1 (and a typed op could be handed a value of the wrong kind).

The compiler keeps, per function, a map from each name to a stack of
registers — the innermost binding on top. Every binder pushes onto it
and pops on scope exit: parameters, `let`, `let*`, lambda parameters,
closure captures and `match` bindings. Every use of the name reads the
register on top.

| Binding of `x` in a function           | Register            |
|-----------------------------------------|---------------------|
| the first one                           | `x`                 |
| any later one (nested or sibling)       | `__r1_x`, `__r2_x`, … |

The first binding keeps the source name so programs that never reuse a
name compile exactly as before. The compiler also tracks every register
name it has used in the function, so a generated name never collides with
a user local (a user local spelled `__r1_x` just pushes the generated
name to the next free one), and a compiler temporary never collides with a
user local spelled like one (`_n1`).

## The `Heap` class (the GC playground)

Twig's heap is a host-side Python object exposed to `vm-core` via
builtins.  V1 uses **reference counting**.  It hosts three object kinds:

```python
class Heap:
    # cons cells
    def alloc_cons(self, car: Any, cdr: Any) -> int: ...
    def car(self, handle: int) -> Any: ...
    def cdr(self, handle: int) -> Any: ...

    # symbols — interned by name
    def make_symbol(self, name: str) -> int: ...
    def symbol_name(self, handle: int) -> str: ...

    # closures — fn_name + captured values
    def alloc_closure(self, fn_name: str, captured: list[Any]) -> int: ...
    def closure_fn(self, handle: int) -> str: ...
    def closure_captured(self, handle: int) -> list[Any]: ...

    # GC plumbing
    def incref(self, handle: int) -> None: ...
    def decref(self, handle: int) -> None: ...
    def is_handle(self, value: Any) -> bool: ...
    def stats(self) -> HeapStats: ...   # { live_objects, peak_objects, total_allocs }
```

V1 rules:

- Allocations get `refcount = 1`.
- Storing a handle into another heap object (cons cell or closure
  capture list) calls `incref` on the inner value.
- The TwigVM walks the result of `main` and decrefs at run end.
- Symbols are *not* refcounted — they're interned for the run.

V1 limitations (intentional, addressed in TW01):

- **Cycles leak.** TW00 doesn't have `letrec`, so building a cycle
  takes deliberate effort, but it's still possible (e.g. via top-level
  defines that reference each other and store closure handles into
  cons cells).  V1 just leaks; TW01's mark-sweep handles it.
- No compaction.  Handle integers are never reused after free — fine
  for the short-lived programs the v1 interpreter targets.

## Tests

- Lexer tests: integers, symbols, parens, quotes, comments (`;`), strings
  out of scope, edge cases (empty input, lone parens).
- Parser tests: simple lists, nested lists, `define`, `define-lambda`
  sugar, error cases (unmatched parens, lone closing).
- Compiler tests: golden IIR for canonical programs.
- Heap tests: alloc / car / cdr round-trip, refcount on overwrite,
  `live_objects` returns to zero after a clean program.
- Execution tests: factorial, range, length, mutual recursion (`even?` /
  `odd?`), arithmetic edge cases (division, comparison), nil / `#t` / `#f`
  truthiness.
- Top-level-only-lambda check: a nested lambda raises a clear
  `TwigCompileError` at compile time.

Coverage target: **≥ 95%**.

## Out of scope

- **Native IIR heap ops + mark-sweep GC + cycle handling + `letrec`** → TW01.
- **Direct BEAM bytecode emission** (no Erlang source intermediary,
  matching the way `wasm-backend` produces WASM bytes) → TW02.
- **Macros, quasiquote, syntax-rules** → indefinite future.
- **Tail-call optimisation in the interpreter.**  vm-core has no TCO;
  recursion is bounded by Python's stack.  Programs that need deep
  recursion can be rewritten with `let` bodies that return their
  recursive call directly — but the spec does not promise TCO until
  TW02 (where BEAM gives it for free).
- **Strings** as a first-class type.  V1 has integers, booleans, nil,
  symbols, cons cells, closures.  Strings can come later as a thin
  wrapper over cons-of-chars or as a new heap object.
- **`call_indirect` as a native IIR op.**  Closures route through
  `call_builtin "apply_closure"` for v1.  Promoting that to a real
  IIR op is a JIT-era optimization.
