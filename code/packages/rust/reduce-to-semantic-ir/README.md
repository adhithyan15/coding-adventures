# reduce-to-semantic-ir

Reduce CST → narrow-waist Semantic IR. The **fourth** frontend to target
[SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
symbolic-expression/pattern-matching domain extension of the SIR10
narrow-waist IR (Stream B of
[HML01](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
`wolfram-to-semantic-ir` (the first), `macsyma-to-semantic-ir` (the
second), and `derive-to-semantic-ir` (the third).

## Where this fits

```
Reduce source
   │
   ▼  coding_adventures_reduce_parser::try_parse_reduce(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  reduce_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR23)
```

## Usage

```rust
use reduce_to_semantic_ir::compile_source;

let module = compile_source("1 + 2;\n", "demo")?;
```

`compile_source` parses and lowers directly, with no worker-thread stack
enlargement — like `macsyma-to-semantic-ir`/`derive-to-semantic-ir` (and
unlike `wolfram-to-semantic-ir::compile_source`), `reduce-parser`'s own
`MAX_RULE_DEPTH` (128) is already documented safe on a bare default
(~2 MiB) stack with comfortable margin: that crate's own doc comment
measures FIVE independent recursion shapes in *rule-frame* terms and
finds the binding constraint is a cons (`.`) chain's floor of 179 rule
frames — 128 sits about 28.5% below that floor, a comparable margin to
`derive-parser`'s own ~33%. `compile` (taking an already-parsed
`GrammarASTNode`) is pure lowering, exactly like every sibling frontend's
`compile`.

## Design: retargeting `reduce-runtime`

`reduce-runtime` already walks this exact CST and compiles it to
`symbolic_ir::IRNode` — this crate's dispatch table is a direct retarget
of that lowering's own rule-name dispatch onto `semantic_ir::Expr`'s
SIR23 vocabulary (`SymSymbol`/`SymApply`) instead. Every construct —
arithmetic, comparisons, logic, lists, cons, `if`, `<< ... >>` group
statements, `:=` assignment/definition — lowers to symbolic *data*,
mirroring `symbolic_ir::IRNode`'s "everything is one apply-tree" design.
See `src/lower.rs`'s module doc comment for the full reasoning and the
node-by-node mapping.

Reduce, like Derive, has no `f[x]`-universal-application syntax (ordinary
parens double as grouping, call, and array-subscript read) and no
pattern-matching/rewrite-rule syntax in this subset (MA08 §4 defers `let`
rules). Unlike Derive, Reduce has three genuinely new constructs: an
**expression-shaped `if`**, a **group statement** `<< s1; s2; ... >>`
(MA08 §3's `CompoundExpression`), and **cons** (`a . b`) — all three
retarget `reduce-runtime::lower`'s own `lower_if`/`lower_group_expr`/
`lower_cons`/`fold_cons` logic directly.

### A REAL divergence from MA08 §3's own prose: arithmetic head names

MA08 §3's table spells arithmetic's "Lowers to" column as `Plus`/
`Subtract`/`Times`/`Power`, even expanding `a / b` to
`Times[a, Power[b, -1]]` and `-a` to `Times[-1, a]`. **None of those
spellings exist in `symbolic-ir`** (confirmed directly:
`grep -n '"Plus"\|"Subtract"\|"Times"\|"Power"' symbolic-ir/src/lib.rs`
returns nothing). The REAL heads — what `symbolic_vm::handlers::
build_handler_table` actually wires, and what `reduce-runtime::lower`
itself uses — are `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`, the exact same
heads `derive-to-semantic-ir` and `macsyma-to-semantic-ir` already use.
This crate uses those same real heads, reusing the identical
`symbolic_ir` constants, so all four symbolic-CAS SIR23 frontends agree
on every arithmetic result. This is a disclosed, deliberate divergence
from the spec's literal prose (already corrected in MA08's own
changelog-style note), not new-head invention — see `src/lower.rs`'s
module doc comment for the full discussion.

### A REAL gap: several MA08 §3 heads have no handler in `symbolic-vm` at all

`CompoundExpression`, `First`, `Second`, `Third`, `Rest`, `Part`,
`Append`, `Reverse` (and the non-folding shape of `Cons`) have **no**
evaluation handler in the shared `symbolic_vm::handlers::
build_handler_table` — `reduce-runtime` reuses the shared backend
unchanged rather than building a bespoke one, so these calls evaluate as
an ordinary unknown-head no-op fallback at runtime. This is largely moot
for this crate: it never evaluates anything (the "everything is data"
design every SIR23 frontend shares), so a `SymApply{head: "First", ...}`
node is valid, executable SIR23 data regardless of whether any *runtime*
currently has a handler for it — confirmed directly by reading
`semantic-ir-to-javascript`'s SIR23 codegen (`emit.rs`), which lowers
`Expr::SymApply` to `__Sir.Symbolic.apply(head, [args...])` uniformly
for ANY head spelling, with no per-head special-casing at all. What DOES
matter is spelling consistency: this crate reuses the exact head names
`reduce-runtime` uses, via its own locally-defined `pub const`s
(`COMPOUND_EXPRESSION`, `CONS`, `FIRST`, `SECOND`, `THIRD`, `REST`,
`PART`, `APPEND`, `REVERSE` — not exported by `symbolic-ir`, and this
crate does not depend on `reduce-runtime` itself, so each is redefined
locally, spelled to match).

### `:=` disambiguation has no operator to branch on

Reduce's grammar has exactly ONE assignment token, `ASSIGN` (`:=`) — `x
:= 5` and `h(l, m) := l - 2*m` are syntactically identical until
lowering. `lower_assignment` disambiguates purely by the *lowered LHS's
shape*: `SymApply{head: SymSymbol(_), ..}` → `Define`, anything else →
`Assign` — exactly mirroring `reduce-runtime::lower::lower_assignment`'s
identical logic. Unlike Derive's self-referential `assignment = logical_or
[ ASSIGN assignment ]`, Reduce's right-hand side is the WIDER `expr`
production, so `x := if a>0 then 1 else -1` and
`x := << a:=1; a+1 >>` both parse and lower directly — Reduce's `if`/
`<<...>>` are genuinely usable as expressions (MA08 §3), with no Derive
analogue.

### Lists (MA08 §3) — flat only, curly braces

`{a, b, c}` (curly braces, NOT Derive's square-bracket `[a,b,c]`
vector/matrix literal) lowers to a flat `List(elems…)`. Reduce's list is
always flat (no row/matrix-literal shape — matrices are out of scope,
MA08 §4), so `lower_list_literal` reuses `lower_arglist` directly instead
of Derive's row-counting split.

### Cons (`.`, MA08 §3)

`a . {b, c}` folds directly into `List(a, b, c)` — the ONE shape MA08 §3
documents a fold for. A right-hand side that isn't structurally a literal
`List` at lowering time (`a . b`) lowers to a bare `Cons(a, b)` — the same
"structurally correct, but no handler evaluates it further" gap as the
list accessors. `fold_cons` mirrors `reduce-runtime::lower::fold_cons`'s
identical logic exactly.

### Recursion-depth hardening

Carried over proactively from `wolfram-to-semantic-ir`'s (four rounds of
security review), `macsyma-to-semantic-ir`'s, and `derive-to-semantic-ir`'s
established pattern, even though neither `reduce-parser` nor
`reduce-runtime` applies any of these guards themselves:

- `MAX_EXPR_DEPTH` (256) bounds this crate's own lowering recursion,
  independent of `reduce-parser`'s own grammar-nesting guard (128).
- `check_chain_length` caps every flat, same-precedence operator-chain
  fold (`additive`/`multiplicative`/`logical_or`/`logical_and`) before any
  tree is built — confirmed these ARE flat EBNF repetitions (not
  right-recursion) directly against `reduce-parser`'s own doc comment,
  which measured an uncapped parser accepting one million repeated items
  with zero crashes for exactly this shape.
- `check_postfix_chain_length` caps chained call application
  (`f(x)(y)(z)…`) — like Derive's postfix (and unlike Wolfram's), Reduce's
  `postfix` has only ONE suffix shape, so a plain per-chain group count is
  already an exact bound.
- `check_apply_arg_count` caps `arglist`/`list_literal` element counts AND
  `group_expr`'s flat statement-sequence length — flat-`Vec`
  allocation-size backstops, not stack guards.
- `measure_depth_iterative`/`drop_iterative` — the authoritative,
  construction-composition-independent iterative depth check and
  iterative teardown, run once per top-level statement.

Reduce's five genuinely self-referential (right-recursive) productions —
parenthesised nesting, the `:=` chain, the `if`/`else` chain, the cons
(`.`) chain, and the power (`^`) chain — need no additional lowering-side
guard beyond the ordinary recursion-depth parameter: `reduce-parser`'s own
`MAX_RULE_DEPTH` (128) already bounds how deep any of these can nest in
the CST this crate ever receives.

Also carried over proactively: every branch that constructs a `FloatLit`
calls `self.observed.add(Feature::Floats)` immediately — a confirmed,
previously-shipped bug in both `matlab-to-semantic-ir` and
`wolfram-to-semantic-ir` (their number-literal helpers were free functions
with no access to the feature-tracking state).

### Testing

- `tests/test_lower.rs` — unit tests asserting exact `Expr` shapes for
  every grammar production (arithmetic with the REAL `Add`/`Sub`/`Mul`/
  `Div`/`Pow`/`Neg` heads, comparisons including `neq`, logic, `if`,
  `<< ... >>` group statements, cons folding/non-folding, lists,
  assignment vs. definition disambiguation, lowercase builtin bridging),
  plus DoS-guard regression tests (flat operator chains, chained call
  application, a wide list literal, a wide group statement, a deeply
  parenthesised expression, a deep cons chain, a deep `if`/`else` chain,
  all confirmed to fail cleanly, never crash), exact-boundary tests
  (256/257), and the `Feature::Floats` regression test.
- `tests/test_validator.rs` — every lowered module passes
  `semantic_ir::validate` (manifest declares exactly the SIR23 features
  used, never `Feature::PatternMatching`) and is **accepted** by
  `semantic-ir-to-javascript`'s capability check, including constructs
  with no shared-VM evaluation handler (`CompoundExpression`, a
  non-folding `Cons`) — confirmed structural acceptance is independent of
  runtime evaluability.
- `tests/e2e_node.rs` — compiles and runs representative Reduce programs
  (arithmetic, a procedure definition+call, assignment, list accessor
  calls, lists/cons, `if` expressions, a group statement, a
  multi-statement program) through `node`, proving the SIR23 codegen path
  is genuinely executable end-to-end, not just statically accepted.
