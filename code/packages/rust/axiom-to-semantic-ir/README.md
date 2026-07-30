# axiom-to-semantic-ir

Axiom CST → narrow-waist Semantic IR. The **sixth** frontend to target
[SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
symbolic-expression/pattern-matching domain extension of the SIR10
narrow-waist IR (Stream B of
[HML01](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
`wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`, `derive-to-semantic-ir`,
`reduce-to-semantic-ir`, and `maple-to-semantic-ir`.

This is **MA-13e**, the last item in Axiom's own native pipeline per
[MA13](../../../specs/MA13-axiom-language.md) §6:

```
spec (MA-13a) → axiom.tokens + axiom-lexer (MA-13b)
              → axiom.grammar + axiom-parser (MA-13c)
              → axiom-runtime + axiom-repl (MA-13d)
              → axiom-to-semantic-ir (MA-13e, THIS crate)
              → oracle/golden testing (native axiom-runtime vs. SIR→JS→node)
                — a SEPARATE follow-on task, not part of this crate
```

## Where this fits

```
Axiom source
   │
   ▼  coding_adventures_axiom_parser::try_parse_axiom(src)
parser::grammar_parser::GrammarASTNode   (generic CST, rooted at `program`
                                           -- exactly ONE expression, see below)
   │
   ▼  axiom_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR23)
```

## Usage

```rust
use coding_adventures_axiom_to_semantic_ir::compile_source;

let module = compile_source("1 + 2", "demo")?;
```

`compile_source` parses and lowers directly, with no worker-thread stack
enlargement — `axiom-parser`'s own `MAX_RULE_DEPTH` (140) is already
documented safe on a bare default (~2 MiB) stack with comfortable margin
(measured at ~33.6% below its own binding-constraint floor of 211 rule
frames, for nested function calls). `compile` (taking an already-parsed
`GrammarASTNode`) is pure lowering, exactly like every sibling frontend's
`compile`.

## `program` is a SINGLE expression, not a repeated worksheet

Unlike every prior SIR23 frontend (`derive.grammar`/`reduce.grammar`/
`maple.grammar` each parse a whole multi-statement file in one call),
`axiom.grammar`'s own `program = expr` parses **exactly one** expression per
call. Axiom is modeled in this repo as a numbered, per-line interactive
session (`axiom-repl` tracks its own step counter, MA13 §5), not a batch
worksheet file — this crate's `compile`/`compile_source` mirror that shape,
lowering one top-level statement into `main`'s body.

## Design: retargeting `axiom-runtime`

`axiom-runtime` already walks this exact CST — but it is a single-phase
**tree-walking interpreter**, not a two-phase "lower once, evaluate once"
runtime like Derive/Reduce/Maple's own, because `::`/`:`/`has` have no
`IRNode` representation at all in its own reduced value model. This crate is
nonetheless a direct structural retarget of `axiom-runtime::eval::eval_expr`'s
own rule-name dispatch, building `semantic_ir::Expr` data instead of
evaluating. See `src/lower.rs`'s module doc comment for the full reasoning
and the node-by-node mapping.

## The central design decision: how `:` / `::` / `has` lower to SIR23

MA13 §2's own finding is that `symbolic_ir::IRNode` has no domain/category
concept at all — and this crate's own research confirms
`semantic_ir::Expr`/`SirType`/`Feature` (SIR23's actual addition) has none
either. **Decision: `:`, `::`, and `has` lower as ordinary `Expr::SymApply`
nodes with three new, locally-defined reserved head-name constants** —
`__axiom_declare`, `__axiom_coerce`, `__axiom_has` — never added to shared
`semantic-ir`/`symbolic-ir`. This is the same "new construct, no shared-crate
change, a local `pub const` head name" pattern this repo's SIR23 family
already established twice: `reduce-to-semantic-ir`'s `CompoundExpression`/
`Cons`/`First`/… and `maple-to-semantic-ir`'s `Set`.

- `a : T` / `(a, b, c) : T` → `Apply(__axiom_declare, [List(names...), T])`
- `e :: T` → `Apply(__axiom_coerce, [e, T])`
- `D has C` → `Apply(__axiom_has, [D, C])`

A `type_expr` position (`Polynomial(Integer)`, `Fraction Integer`) is
structurally just "a NAME, optionally applied to further arguments" — the
same shape an ordinary function call already has — so it lowers to the exact
same `SymSymbol`/`SymApply` shapes, via a dedicated `lower_type_expr`
function rather than a new node kind.

**Runtime-shim status: deferred to the follow-on oracle-testing task, not
shipped in this PR.** This crate never evaluates anything (the "everything is
data" design every SIR23 frontend shares), so it does not need a working
evaluator to emit correct SIR. Verified directly (not assumed from a
suggestion to extend `sir-runtime-symbolic`): the JS backend's real SIR23
evaluator (`Symbolic.evalTerm`, `HELD_HEADS`, …) lives **inline** inside
`semantic-ir-to-javascript/src/runtime.rs`'s own emitted runtime blob, not in
the published `@coding-adventures/sir-runtime-symbolic` npm package — that
package (confirmed by reading its own `CHANGELOG.md`) only re-exports the
structural pattern matcher and leaf-term constructors, with no evaluator at
all, and only backs the TypeScript backend, which no Stream B oracle test
exercises yet. So extending it would not, on its own, make `:`/`::`/`has`
evaluate through the path this repo's `node`-execution oracle tests actually
use. Given oracle/golden testing for Axiom is this task's own named separate
follow-on item, and every prior "new reserved head, no evaluator yet"
precedent in this exact family (`Set`, `CompoundExpression`, `Cons`, `First`,
`Second`, `Third`, `Rest`, `Part`, `Append`, `Reverse`) shipped its frontend
first and left the evaluator as later work, this crate follows the identical
sequence. See `src/lower.rs`'s module doc comment for the full disclosure.

## A disclosed widening relative to `axiom-runtime`'s own function bodies

`axiom-runtime::eval::lower_pure_body` rejects `:=`/`:`/`::`/`has`/a
`;`-sequenced block inside a held function body, because none of those
constructs have any representation in that crate's own reduced `IRNode`
value model. This crate imposes **no equivalent restriction** — since
everything is data here, all of those constructs already have an ordinary
`SymApply` representation, so a function body containing any of them lowers
exactly like a top-level statement would. This is a real, disclosed widening
relative to `axiom-runtime`'s own current scope, not a bug: the native
runtime's restriction is an artifact of its own two-phase-incompatible
evaluation design, not a limitation of Axiom-the-language or of this SIR
target.

## Declared function definitions: type annotations are dropped, not validated

`declared_define`'s typed parameter list and return-type annotation are
dropped entirely at lowering time — only each parameter's bare NAME is kept,
producing the same 3-argument `Define(name, List(params...), body)` shape
Derive's/Reduce's/Maple's own definitions already use. `axiom-runtime`
resolves each annotation against the fixed domain table at
definition-evaluation time (but never enforces it against call arguments —
MA13 §4's own "duck-typed" note) — reproducing that check here would mean
either duplicating the fixed domain table a second time in a purely
syntactic frontend, or changing `Define`'s established shape. This crate
takes the narrower path and drops the annotations, matching every sibling
frontend's `Define`.

### No logical operators, no `elif`

`axiom.grammar` has no `and`/`or`/`not` production and no `elif` repetition
at all (MA13 §4's own table lists neither) — a genuinely smaller grammar than
Maple's/Macsyma's, needing no `check_elif_chain_length`-equivalent guard.

### `postfix` is NOT chainable

`axiom.grammar`'s `postfix = atom [ call_args ]` allows at most ONE call
suffix — `f(x)(y)` is not valid Axiom syntax in this subset — so, mirroring
`maple-to-semantic-ir`'s identical finding, there is no
`check_postfix_chain_length`-equivalent guard anywhere in this crate.

### Recursion-depth hardening

Carried over proactively from every prior SIR23 frontend's established
security-review history, even though `axiom-parser`/`axiom-runtime` apply
none of these guards themselves:

- `MAX_EXPR_DEPTH` (256) bounds this crate's own lowering recursion,
  independent of `axiom-parser`'s own grammar-nesting guard (140).
- `check_chain_length` caps every flat, same-precedence operator-chain fold
  (`additive`/`multiplicative`) before any tree is built.
- `check_apply_arg_count` caps every flat-`Vec` production's element count
  (call arguments, list literals, typed-parameter lists, tuple-declaration
  name lists, type-constructor argument lists, and a `;`-block's statement
  count) — an allocation-size backstop, not a stack guard, since a
  `;`-block lowers to one FLAT n-ary `CompoundExpression`, never a folded
  tree.
- `measure_depth_iterative`/`drop_iterative` — the authoritative,
  construction-composition-independent iterative depth check and iterative
  teardown, run once per top-level statement.

### Testing

- `tests/test_lower.rs` — unit tests asserting exact `Expr` shapes for every
  grammar production (literals including the first SIR23-family `STRING`
  literal, calls in both call forms, lists, arithmetic/comparison operators,
  `:=` assignment, declared/undeclared `==` definitions with type-annotation
  dropping, `if`/`then`/`else` with mandatory `else` and dangling-else
  resolution, the grouping-vs-block `;`-count distinction, `:`/`::`/`has`
  including the paren-optional type shorthand and precedence relative to
  `additive`/`comparison`, deeply nested type constructors), error-path
  regressions, and DoS-guard regression tests.
- `tests/test_validator.rs` — every lowered module passes `semantic_ir::
  validate` and is **accepted** by `semantic-ir-to-javascript`'s capability
  check, including the three new reserved-head constructs with no shared-VM
  evaluation handler.
- `tests/e2e_node.rs` — compiles and runs representative Axiom programs
  (arithmetic, declared/undeclared function definition + call, assignment,
  lists, `if`/`then`/`else`, a multi-statement block, and `:`/`::`/`has` as
  inert data construction) through `node`, proving the SIR23 codegen path is
  genuinely executable end-to-end.
- No `tests/oracle.rs` in this PR — oracle/golden testing against
  `axiom-runtime` is an explicitly separate follow-on task (see
  `CHANGELOG.md`).
