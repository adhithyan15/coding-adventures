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
| `f(x: T, ...): T == e`, `f x == e` | Held-body function definition, registered via the shared VM's own `Define`/user-function-call mechanism (reused unchanged — see "Function bodies" below) |
| `if p then e1 else e2` | `p` must evaluate to `Boolean`; only the chosen branch is evaluated |
| `( e1; e2; ...; eN )` | Sequencing; each statement's side effects (bindings, declarations, definitions) persist; the block's value is the last statement's |

## Function bodies: a disclosed, narrower subset

A held function body is registered via `symbolic_vm`'s own `Define`/
user-function-call mechanism, **reused completely unchanged** — the exact
mechanism Derive/Reduce/Maple already use for their own user-defined
functions, which means Axiom's own user functions get exactly the same
(lack of an extra) recursion-depth guard every sibling CAS-family runtime
here already has, rather than a new bespoke one.

The trade-off: since `::`/`:`/`has` have no `IRNode` representation at all,
a function body **cannot** contain them (there is nothing for the shared
VM's substitution mechanism to evaluate them *as*). Bodies are restricted to
the arithmetic/comparison/`if`/call/list subset — matching MA13 §4's own
single confirmed function-definition example
(`power(x: Integer, n: NonNegativeInteger): Integer == x ** n`, a pure
arithmetic expression) exactly. Writing `:=`/`:`/`::`/`has`, or a
`;`-sequenced block, inside a body is a clean `EvalError`, not a silent
mis-lowering.

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

`feed`/`eval_to_output` are the trust boundary for arbitrary Axiom source:

1. **Deeply nested source** (`((((…))))`) — already rejected by
   `axiom-parser`'s own `MAX_RULE_DEPTH`.
2. **A long flat chain** (`1+1+1+…`) — evaluated **iteratively**, one small
   `VM::eval` call per fold step, sidestepping the "flat chain folds into a
   deep tree" DoS vector by construction. `MAX_STATEMENT_TOKENS` (measured
   against the real lexer token stream) still exists as defense-in-depth.

Evaluation runs on a worker thread with a large bounded stack inside
`catch_unwind`, so a reused-handler panic becomes a clean `Err` and the
session (VM environment *and* declared-domain table) is rebuilt rather than
left corrupted.

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
(including recursion through a defined function); the disclosed
function-body restriction; and both robustness guards.
