# maple-to-semantic-ir

Maple CST → narrow-waist Semantic IR. The **fifth and final** frontend to
target [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
symbolic-expression/pattern-matching domain extension of the SIR10
narrow-waist IR (Stream B of
[HML01](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
`wolfram-to-semantic-ir` (the first), `macsyma-to-semantic-ir` (the
second), `derive-to-semantic-ir` (the third), and `reduce-to-semantic-ir`
(the fourth). This closes Stream B's tracked language list — every
math-CAS language HML01 currently names now has a SIR23 frontend.

## Where this fits

```
Maple source
   │
   ▼  coding_adventures_maple_parser::try_parse_maple(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  maple_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR23)
```

## Usage

```rust
use maple_to_semantic_ir::compile_source;

let module = compile_source("1 + 2;\n", "demo")?;
```

`compile_source` parses and lowers directly, with no worker-thread stack
enlargement — like `macsyma-to-semantic-ir`/`derive-to-semantic-ir`/
`reduce-to-semantic-ir` (and unlike `wolfram-to-semantic-ir::
compile_source`), `maple-parser`'s own `MAX_RULE_DEPTH` (150) is already
documented safe on a bare default (~2 MiB) stack with comfortable margin:
that crate's own doc comment measures SIX independent recursion shapes in
*rule-frame* terms and finds the binding constraint is a `not`-prefix
chain's floor of 218 rule frames — 150 sits about 31.2% below that floor,
a comparable margin to `reduce-parser`'s own ~28.5%. `compile` (taking an
already-parsed `GrammarASTNode`) is pure lowering, exactly like every
sibling frontend's `compile`.

## Design: retargeting `maple-runtime`

`maple-runtime` already walks this exact CST and compiles it to
`symbolic_ir::IRNode` — this crate's dispatch table is a direct retarget
of that lowering's own rule-name dispatch onto `semantic_ir::Expr`'s
SIR23 vocabulary (`SymSymbol`/`SymApply`) instead. Every construct —
arithmetic, comparisons, logic, lists, sets, `if`/`elif`/`else`,
assignment, arrow-operator definitions — lowers to symbolic *data*,
mirroring `symbolic_ir::IRNode`'s "everything is one apply-tree" design.
See `src/lower.rs`'s module doc comment for the full reasoning and the
node-by-node mapping.

### A REAL structural difference from Reduce: the dispatch is SPLIT

`reduce.grammar`'s `expr = if_expr | group_expr | assignment` sits at the
very top of Reduce's expression grammar, so `if`/`:=` are reachable from
*every* position an `expr` can appear. `maple.grammar` draws a hard line
Reduce's own grammar does not: `statement = if_expr | assignment` sits in
its OWN nonterminal, never reachable from `expr` at all — `expr` itself
is just `logical_or` (Chapter 3 "Maple Expressions"), with no alternative
that ever leads back to `if_expr`/`assignment` (Chapter 5 "Maple
Statements"). This crate's `lower_node` is still one shared dispatch
table (mirroring `maple-runtime::lower::lower_node`'s own single `match`,
not two separate Rust functions) — but the grammar's own reachability
graph is what enforces the real divide: `lower_if`/`lower_assignment` are
only ever reached via the top-level statement loop, never nested inside
an arithmetic/comparison/logical operand the way Reduce's can be. `x :=
if a then 1 else 2 end if;` and `a := b := c;` are both syntax errors in
Maple's grammar.

### Assignment: bare `NAME` LHS only, plus a new `arrow_def`/`arrow_params`

`assignment = NAME ASSIGN ( arrow_def | expr ) | expr` — deliberately
NARROWER than Reduce's/Derive's own call-shaped-LHS disambiguation:
Maple's identical-looking `f(x) := expr` means something narrower in real
Maple (a remember-table patch onto an existing procedure, MA09 §1/§4)
than Reduce's/Derive's general-definition idiom, so `maple.grammar` makes
`f(x) := expr` fail to *parse* at all — this crate never needs to check
whether the LHS is call-shaped. Instead, Maple has a SEPARATE production,
`arrow_def = arrow_params ARROW expr` (`f := (x, y) -> x + y` / `f := x
-> x^2`), for general function definition — `lower_arrow_def` lowers this
to `Define[f, List[params...], body]`, the same `Define` shape Derive's/
Reduce's own (differently-spelled) definitions already use.

### `if`/`elif`/`else` — a right-fold, mirroring Macsyma's elif chain

`if_expr = "if" expr "then" statement { "elif" expr "then" statement } [
"else" statement ] ( "end" "if" | "fi" )` — unlike Reduce's simpler
2-or-3-child `if` (no `elseif` repetition at all), Maple's flat `{ "elif"
... }` EBNF repetition folds right-to-left into nested `If` applications,
the same shape Macsyma's own elif-chain fold uses. Because Maple requires
an explicit close (`end if` or `fi`) for every `if_expr`, there is no
dangling-else ambiguity the way Reduce's `if`/`else` chain has to resolve
by convention.

### `Set` — a canonical head genuinely new to this repo (MA09 §5)

Maple is the first language here with TWO distinct bracketed aggregate
literals — `[a, b, c]` (ordered → `List`, a shared existing head) and
`{a, b, c}` (unordered → `Set`, MA09 §3/§5). `symbolic-vm`'s shared
handler table has no handler for `Set` — so `SET` is a `pub const`
defined LOCALLY in this crate, exactly the pattern
`reduce-to-semantic-ir`/`reduce-runtime` used for their own new
`COMPOUND_EXPRESSION`/`CONS`/`FIRST`/… constants (not added to shared
`symbolic-ir`, since it's not a `Backend`-agnostic canonical head).

### `diff`/`int` — thin bridges to the shared `D`/`Integrate` handlers

Same idea as Derive's `DIF`/`INT` bridge, just lowercase surface spelling
(MA09 §3: Maple's builtin names are conventionally lowercase). A plain
name→canonical-head bridge table entry, no new logic — MA09 §3 documents
no other function-call bridge for this subset (list/set construction
already has literal syntax; elementary-function names like `sin` are
deliberately not bridged, same as Reduce).

### Booleans — the first literal `true`/`false` tokens in this CAS family

Neither Derive's nor Reduce's grammar has a dedicated boolean literal
token — `maple.grammar`'s `atom` rule is the first to include `"true"`/
`"false"` as their own alternatives. These bridge to the shared backend's
pre-bound `True`/`False` symbols, the same bridge `macsyma-compiler`
already uses for its own boolean keywords. `symbolic-ir` exports no
`TRUE`/`FALSE` constants (verified directly), so this uses bare string
literals, matching `maple-runtime`'s own bridge.

### `postfix` is NOT chainable

`maple.grammar`'s `postfix = atom [ LPAREN [ arglist ] RPAREN ] ;` has a
single OPTIONAL call suffix — `f(x)(y)` is not valid Maple syntax in this
subset at all (confirmed by a regression test asserting `compile_source`
rejects it as a parse error), unlike Reduce's/Derive's REPEATED `{
LPAREN [arglist] RPAREN }` chain. So there is no
`check_postfix_chain_length`-equivalent guard anywhere in this crate —
the axis it would guard is structurally impossible here, not merely
bounded by a cap.

### `;`-vs-`:` is a runtime/session concept, not something this frontend tracks

MA09 §3's own statement-separator row is explicit this is "a display flag
on the surrounding session, not an IR node" — `maple-runtime`'s own
`LoweredStatement`/`Display` types exist only for its REPL's
result-printing decision. This SIR23 frontend has no interactive-session
concept at all (mirrors how neither `derive-to-semantic-ir` nor
`reduce-to-semantic-ir` replicate their own native runtimes' prompt/
display machinery either) — every statement lowers to a plain
`Stmt::ExprStmt`, and the `;`/`:` distinction is ignored entirely.

### Recursion-depth hardening

Carried over proactively from `wolfram-to-semantic-ir`'s (four rounds of
security review), `macsyma-to-semantic-ir`'s, `derive-to-semantic-ir`'s,
and `reduce-to-semantic-ir`'s established pattern, even though neither
`maple-parser` nor `maple-runtime` applies any of these guards themselves:

- `MAX_EXPR_DEPTH` (256) bounds this crate's own lowering recursion,
  independent of `maple-parser`'s own grammar-nesting guard (150).
- `check_chain_length` caps every flat, same-precedence operator-chain
  fold (`additive`/`multiplicative`/`logical_or`/`logical_and`) before
  any tree is built.
- `check_elif_chain_length` caps the `elif`-arm count in an `if_expr`
  before the right-fold runs — the same "flat repetition folds into a
  deep tree" shape, applied to the construct genuinely new relative to
  Reduce's simpler `if`.
- **No `check_postfix_chain_length`-equivalent guard** — see above for
  why: chained application is structurally impossible in this grammar.
- `check_apply_arg_count` caps `arglist` element counts (shared by call
  arguments, list literals, and set literals) AND `arrow_params`'s flat
  parameter-name count — flat-`Vec` allocation-size backstops, not stack
  guards.
- `measure_depth_iterative`/`drop_iterative` — the authoritative,
  construction-composition-independent iterative depth check and
  iterative teardown, run once per top-level statement.

Maple's six genuinely self-referential (right-recursive or
prefix-recursive) productions — parenthesised nesting, list-/set-literal
nesting, a `not`-prefix chain, a unary-minus-prefix chain, the power
(`^`) chain, and nested `if`/`end if` (or `fi`) — need no additional
lowering-side guard beyond the ordinary recursion-depth parameter:
`maple-parser`'s own `MAX_RULE_DEPTH` (150) already bounds how deep any
of these can nest in the CST this crate ever receives.

Also carried over proactively: every branch that constructs a `FloatLit`
calls `self.observed.add(Feature::Floats)` immediately — a confirmed,
previously-shipped bug in both `matlab-to-semantic-ir` and
`wolfram-to-semantic-ir` (their number-literal helpers were free
functions with no access to the feature-tracking state).

### Testing

- `tests/test_lower.rs` — unit tests asserting exact `Expr` shapes for
  every grammar production (arithmetic, comparisons including `<>`,
  logic, `if`/`elif`/`else`/`end if`/`fi`, arrow-definitions with
  zero/one/multiple parameters, `List`-vs-`Set` distinction, boolean
  literals, `diff`/`int` bridging), regression tests for every
  statement/expression-split syntax error (chained assignment, the
  remember-table spelling, `if` as an assignment RHS, chained postfix
  application), plus DoS-guard regression tests (flat operator chains, a
  wide list/set literal, a wide `arrow_params` list, a wide `elif`
  chain, a deeply parenthesised expression, a deep `not` chain, a deep
  nested-`if` chain — all confirmed to fail cleanly, never crash), and
  exact-boundary tests.
- `tests/test_validator.rs` — every lowered module passes
  `semantic_ir::validate` (manifest declares exactly the SIR23 features
  used, never `Feature::PatternMatching`) and is **accepted** by
  `semantic-ir-to-javascript`'s capability check, including the `Set`
  construct with no shared-VM evaluation handler.
- `tests/e2e_node.rs` — compiles and runs representative Maple programs
  (arithmetic, an arrow-definition+call, assignment, lists/sets,
  `if`/`elif`/`else`, boolean literals, `diff`/`int` calls, a
  multi-statement program) through `node`, proving the SIR23 codegen
  path is genuinely executable end-to-end, not just statically accepted.
