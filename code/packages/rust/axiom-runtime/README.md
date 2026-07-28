# coding-adventures-axiom-runtime

Evaluates Axiom (the MA-13-scoped, consumer-view subset of the strongly
typed category/domain CAS) by walking the `axiom-parser` (MA-13c) CST as an
interpreter, delegating every ordinary arithmetic/comparison sub-expression
to [`symbolic-vm`](../symbolic-vm)'s shared `SymbolicBackend` — unchanged,
no custom `Backend` — and adding one genuinely new, `axiom-runtime`-internal
layer: a fixed, non-extensible domain/category table for `:` (declare), `::`
(coerce), and `has` (category query). See
[`code/specs/MA13-axiom-language.md`](../../../specs/MA13-axiom-language.md).

## Where this fits

```
axiom.tokens + axiom-lexer     (MA-13b)
        |
axiom.grammar + axiom-parser   (MA-13c)
        |
axiom-runtime                  (MA-13d, this crate)
        |            \
        |             \-- crate::domains / crate::builtins: the fixed
        |                 AxiomDomain/AxiomCategory table + type_expr
        |                 resolution
        |
axiom-repl                     (MA-13d)  <- crate::value::print_axiom
        |
axiom-to-semantic-ir           (MA-13e, next)
```

## Why this crate is an interpreter, not "lower, then evaluate"

Every other CAS-family runtime here (`derive-runtime`, `reduce-runtime`,
`maple-runtime`) lowers a whole parsed tree to `symbolic_ir::IRNode` first,
then hands it to `symbolic_vm::VM::eval` once — every one of their surface
constructs has a direct `IRNode` head. Axiom's `::`/`:`/`has` do **not** —
`symbolic-ir` has no domain/category concept at all (MA13 §2) — and `::` can
appear nested anywhere inside ordinary arithmetic (`a + b :: Float` is legal,
MA13 §4), so there is no clean lower-then-evaluate split. `crate::eval`
instead walks the tree **evaluating eagerly**, so that by the time it
reaches a `coercion`/`declaration`/`has_query` node it already has a
concrete, evaluated `AxiomValue` to check against the fixed domain table —
"evaluated entirely within `axiom-runtime`'s own dispatcher, never inside
`symbolic-vm` itself" (MA13 §2/§5). See `crate::eval`'s own module doc
comment for the full rationale, including why this sidesteps the "flat
chain folds into a deep tree" DoS vector by construction (each arithmetic
fold step is its own small `VM::eval` call).

## The fixed domain/category table (MA13 §3/§4)

```
Domains:    Boolean, Integer, PositiveInteger (x > 0),
            NonNegativeInteger (x >= 0), Float, String,
            Fraction(Integer), Polynomial(Integer), List(T)
Categories: Ring        -- Integer, Fraction(Integer), Polynomial(Integer)
            OrderedSet  -- Integer, Float, PositiveInteger, NonNegativeInteger
```

Confirmed against the book: `Polynomial(Integer) has Ring` is `true`;
`List(Integer) has Ring` is `false`. `crate::domains::coerce_value` is the
subdomain-predicate-plus-conversion function both `::` and a `:`-declared
`:=` consult — most domains are a pure membership check (the value's own
`IRNode` shape already *is* the target representation), but `Float` actually
**converts** (`3 :: Float` produces `3.0`, not a re-tagged `Integer`).

**Deliberately not parameterized over each other beyond this** (no
`Polynomial(Fraction(Integer))`, no `Complex`, no `Matrix` this cut) — the
fixed, finite table is itself part of MA13 §3's scoping decision, not an
oversight.

## Coercion-failure error shape

Adapted from the book's own confirmed phrase (MA13 §3), quoted verbatim for
the assignment-mismatch case:

> Cannot convert right-hand side of assignment ... to an object of the type
> Integer of the left-hand side.

`crate::eval::eval_assignment` reproduces this closely for a `:`-declared
variable whose `:=` right-hand side doesn't fit; `crate::eval::eval_coercion`
adapts the same phrase for a standalone `::` (which has no "left-hand side"
of its own to name) — disclosed here as an adaptation, not an
independently-verified-to-the-byte second quotation from the book.

## What each construct does at runtime (MA13 §4)

| Surface | Runtime behaviour |
|---|---|
| `123` | Domain-inferred `PositiveInteger` if positive, else `Integer` (MA13 §4's own literal-inference row lists only these three; a zero/negative integer literal is conservatively `Integer`, not `NonNegativeInteger`) |
| `1.5` | `Float` |
| `1/3` | `Fraction(Integer)` (`IRNode::Rational`) |
| `a : T`, `(a, b, c) : T` | Records a domain constraint for the name(s); consulted at the next `:=` |
| `e :: T` | Coerces now; the book's error shape on failure |
| `D has C` | Looked up in the fixed category table, `Boolean` result |
| `+ - * / ^ ** = ~= < <= > >=` | Delegated to the shared, unmodified `symbolic-vm` handler table |
| `x := e` | Evaluates `e`, domain-checks against any `a : T` constraint, binds |
| `f(x: T, ...): T == e`, `f x == e` | Held-body function definition, registered in this crate's own function table and dispatched by this crate's own call mechanism (see "Function bodies" below) |
| `if p then e1 else e2` | `p` must evaluate to `Boolean`; only the chosen branch is evaluated |
| `( e1; e2; ...; eN )` | Sequencing; each statement's side effects (bindings, declarations, definitions) persist; the block's value is the last statement's |

## Function bodies: a disclosed, narrower subset, and a depth-guarded call mechanism

A held function body is registered in this crate's *own* function table
(`name -> (params, body)`, in `AxiomSession`), and a call to it is dispatched
by this crate's own `crate::eval::call_user_function`/`eval_ir` — **not**
`symbolic_vm`'s own `Define`/user-function-call mechanism. This is a
deliberate, security-motivated design, not a layering preference: an
earlier version of this crate *did* register functions via `symbolic_vm`'s
own mechanism and hand a call straight to `VM::eval`, and that mechanism's
own recursive-call handling runs **inside `VM::eval_apply`'s own Rust call
stack**, which this crate cannot instrument or cap without modifying
`symbolic-vm` itself (ruled out, MA13 §2). A self-recursive function with no
terminating base case (`fact(n) == ... fact(n - 1)`, called with a huge `n`)
would recurse natively through `symbolic-vm`'s own call stack with **no
depth limit at all**, and a genuine native stack overflow is **not**
catchable by `catch_unwind` — Rust's runtime response to one is to abort
the whole process, not unwind a thread. A large worker-thread stack does not
fix this either: it only raises how deep the recursion must go before
crashing.

Dispatching calls in this crate instead lets `crate::eval::MAX_CALL_DEPTH`
(re-exported as `coding_adventures_axiom_runtime::MAX_CALL_DEPTH`) be
enforced on every user-function invocation, at *any* nesting position
inside a body (an `if` branch, an arithmetic operand, …), turning unbounded
recursion into a clean `Err` instead of a process abort.

Separately, since `::`/`:`/`has` have no `IRNode` representation at all, a
function body **cannot** contain them (there is nothing for a stored body
to represent them *as*). Bodies are restricted to the
arithmetic/comparison/`if`/call/list subset — matching MA13 §4's own single
confirmed function-definition example (`power(x: Integer, n:
NonNegativeInteger): Integer == x ** n`, a pure arithmetic expression)
exactly. Writing `:=`/`:`/`::`/`has`, or a `;`-sequenced block, inside a
body is a clean `EvalError`, not a silent mis-lowering.

## Usage

```rust
use coding_adventures_axiom_runtime::AxiomSession;

let mut s = AxiomSession::new();
assert_eq!(s.feed("a : PositiveInteger").unwrap(), "(1) true : Boolean\n");
assert_eq!(s.feed("a := 5").unwrap(), "(2) 5 : PositiveInteger\n");
assert!(s.feed("a := -1").is_err()); // the book's own confirmed error shape
assert_eq!(
    s.feed("Polynomial(Integer) has Ring").unwrap(),
    "(3) true : Boolean\n"
);
assert_eq!(s.feed("3 :: Fraction Integer").unwrap(), "(4) 3 : Fraction(Integer)\n");
```

`coding_adventures_axiom_runtime::eval(src)` is a one-shot convenience for
callers that don't need a persistent session.

## Robustness

`feed`/`eval_to_output` are the trust boundary for arbitrary Axiom source —
**four** independent recursion/panic vectors are closed, not three:

1. **Deeply nested source** (`((((…))))`) — already rejected by
   `axiom-parser`'s own `MAX_RULE_DEPTH`.
2. **A long flat chain** (`1+1+1+…`) — evaluated **iteratively**, one small
   `VM::eval` call per fold step, sidestepping the "flat chain folds into a
   deep tree" DoS vector by construction. `MAX_STATEMENT_TOKENS` (measured
   against the real lexer token stream, and now checked *inside* the same
   worker thread/`catch_unwind` boundary as evaluation itself, not on the
   caller's own thread) still exists as defense-in-depth.
3. **Unbounded recursive-call depth** — a self- or mutually-recursive
   user-defined function (`fact(n) == ... fact(n - 1)`, called with a huge
   `n`) is a *third*, independent vector from the two above: neither
   `MAX_RULE_DEPTH` (static source nesting) nor `MAX_STATEMENT_TOKENS` (one
   submission's token count) bounds how many times a function calls itself
   at *evaluation* time, since that depends on the runtime value passed in.
   `MAX_CALL_DEPTH` closes it — see "Function bodies," above, for the full
   incident this was added to close in review (this crate used to delegate
   function calls to `symbolic_vm`'s own uncapped mechanism, and a genuine
   native stack overflow is not catchable by `catch_unwind` at all).
4. **Unwinding panics** from the reused shared handler table run inside
   `catch_unwind` on a worker thread with a large bounded stack; the session
   (VM environment, declared-domain table, *and* function table) is rebuilt
   afterward rather than left corrupted. This is a narrower guarantee than
   point 3 — `catch_unwind` only ever catches an unwinding `panic!`, never a
   genuine stack overflow, which is exactly why point 3 needs its own,
   independent depth cap.

## Tests

```sh
cargo test -p coding-adventures-axiom-runtime
```

Covers: domain inference for every literal shape; `:`/`::`/`has` against
every built-in domain/category pair (including the confirmed
`Polynomial(Integer) has Ring` → `true` / `List(Integer) has Ring` → `false`
examples); the `PositiveInteger`/`NonNegativeInteger` subdomain predicates;
the coercion/declaration-mismatch error shape; arithmetic/comparison
correctness delegated to the shared engine; `:=`/`==`/`if`/block evaluation
(including recursion and mutual recursion through defined functions); the
disclosed function-body restriction; the `MAX_CALL_DEPTH` guard (both on a
generously-sized worker-thread stack, confirming the cap — not the stack —
is what trips, and on a deliberately small one, confirming the cap trips
*before* any native overflow risk); and all four robustness guards.
