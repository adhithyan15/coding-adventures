# Step 3 — `let` + arithmetic + the derivation tree (provenance-through-math)

> Build plan of record for the third slice of [DESIGN.md](DESIGN.md). Grounded in an actual
> read of the reuse crates (symbolic-vm, symbolic-ir, cas-algebraic, logic-core/engine). Two of
> the DESIGN's reuse assumptions were **wrong** and are corrected here before any code is written.

## 0. Goal (unchanged from DESIGN)

Let an `.adj` rulebook name a derived value with a **formula** and have the engine compute it
deterministically on the CPU, recording a **derivation tree** from the result back to the source
facts' byte spans. The model writes the formula; it never evaluates it.

```
observe csf_glucose = quantity(40, mg_dl)
observe serum_glucose = quantity(100, mg_dl)
observe line_item(12000)
observe line_item(6000)

let csf_ratio = csf_glucose / serum_glucose      % arithmetic over two slots
let total     = sum(line_item)                    % aggregation over repeated slot

contributes 1000000 from csf_ratio <= 0.4 to bacterial_meningitis   % predicate over a DERIVED value
contributes 1000000 from total     >= 14600 to required_to_file
? bacterial_meningitis
? required_to_file
```

The predicate machinery from step 1 already compares `slot <op> value`; step 3's job is to make a
`let`-bound name behave like an observed valued slot whose value is **computed, with provenance**.

## 1. Corrections to the DESIGN reuse map (grounded read)

| DESIGN assumed | Reality (verified) | Consequence |
|---|---|---|
| `cas-algebraic` solves equations (`solve(eq, var)`) | It is a **quadratic/quartic factoring helper over Q[√d]** (`try_split_quadratic`, `factor_over_extension`) — **no general solve API** | **Step 6** (equation solving) must build a small solver (linear + quadratic-formula) itself; cas-algebraic is only useful for the irrational-root case. Re-scope step 6; out of step-3 scope. |
| `symbolic-vm` handlers can emit a derivation tree | `Handler = Arc<dyn Fn(&mut VM, IRApply) -> IRNode>` returns **only** an `IRNode`; no side-channel for "record this op" | The derivation tree is **not** free from symbolic-vm. See §3 decision. |
| value IR = `symbolic-ir::IRNode`, facts = `logic_core::Term` | They are **separate, non-interoperable** types (Var vs Symbol, Num vs Integer/Rational/Float, Compound vs Apply) | Reusing symbolic-vm requires a `Term ↔ IRNode` bridge (~100 LOC) on every eval. |

**Verified signatures** (for whoever implements):
- `symbolic_vm::VM::new(Box<dyn Backend>)`, `VM::eval(&mut self, IRNode) -> IRNode` (vm.rs:42,51)
- `Backend::handler_for(&self, &str) -> Option<&Handler>`; `Handler = Arc<dyn Fn(&mut VM, IRApply) -> IRNode + Send + Sync>` (backend.rs:47,109)
- symbolic-vm already ships ADD/SUB/MUL/DIV/NEG/… handlers (handlers.rs:1445+)
- `symbolic_ir::IRNode { Symbol|Integer(i64)|Rational(i64,i64)|Float(f64)|Str|Apply(Box<IRApply>) }`; `IRApply { head, args }` (lib.rs)
- `logic_core::Term { Atom|Num(Number)|Str|Var|Compound{functor,args} }`, `Number{Int(i64)|Float(f64)}`

## 2. The two engine job-halves

- **Parsing the formula** (adj-lang): `let <name> = <expr>` where `<expr>` is arithmetic over slot
  names, numeric literals, and aggregation calls. New AST `Statement::Let { name, expr }` + an
  `Expr` tree (`Ref(slot) | Lit(f64) | Bin(op, a, b) | Agg(fn, slot)`).
- **Evaluating with provenance** (logic-engine): a value carrying a `Derivation` is bound to `name`
  as a synthetic valued fact, so the existing predicate path reads it via `observed_value`.

## 3. DECISION — build a small adjudication evaluator over `Term`, NOT a symbolic-vm bridge (for v1)

The DESIGN said "reuse symbolic-vm." Given the grounded read, **for step 3 we do not**, because:
1. symbolic-vm gives no derivation capture — the whole point of this slice — so we'd be writing the
   derivation walker anyway.
2. The `Term ↔ IRNode` bridge is pure friction (every fact converts on every eval), and `IRNode`'s
   `Rational` adds an arithmetic model we don't need yet.
3. A self-contained evaluator over `logic_core::Term` arithmetic compounds is ~150 LOC, fully
   provenanced, and has zero new cross-crate coupling.

**What we DO reuse:** `logic_core::Number` semantics and the `numeric_magnitude` reader from step 2
(so an operand can be a typed value `quantity(40, mg_dl)`), and the existing predicate/differential
machinery unchanged. **Symbolic-vm/cas-algebraic are deferred to step 6** (equation solving), where
their algebra is actually load-bearing — recorded here so we don't bridge prematurely.

### The derivation tree type (new, in logic-engine)

```
pub enum DerivationNode {
    Leaf  { slot: String, value: f64, fact_id: FactId },      // an observed fact (cites its bytes via fact provenance)
    Lit   { value: f64 },                                      // a structural constant in the formula
    Op    { op: ComputeOp, operands: Vec<DerivationNode>, result: f64 },  // add/sub/mul/div/sum/count/min/max/avg/ratio
}
pub struct Derived { pub name: String, pub value: f64, pub tree: DerivationNode }
```
- Every `Leaf` carries the `FactId` so the audit trail descends from a verdict → predicate over a
  derived value → the `Op` tree → the `Leaf` facts → each fact's `Provenance` (source bytes). This is
  the provenance-through-math invariant.
- `Lit` is the **no-magic-numbers** seam: a literal that is neither a fact nor a declared structural
  constant is a gate violation (§5).

### Binding a derived value

`let name = expr` evaluates to a `Derived`; the engine stores it so `observed_value("name")` returns
`Derived.value` (extend `observed_value` to also consult a `derived: Vec<Derived>` table, magnitude =
`value`). The derived name is then usable in any predicate exactly like an observed slot. Keep the
`Derived.tree` reachable by name for the proof DAG + gates.

### Proof origin

Add `DerivationOrigin::FromComputation { derived_name: String, tree_summary: ... }` (or have the
predicate step that fires over a derived value carry an optional reference into the `Derived` table).
The CLI renders the `Op` tree under the predicate step so the audit shows `csf_ratio = 40 / 100 =
0.4` with each operand's citation.

## 4. Surface syntax + grammar (coupled, one PR — the step-1 lesson)

```
let_decl  = "let" IDENT "=" expr ;
expr      = term_expr { ( PLUS | MINUS ) term_expr } ;
term_expr = factor { ( STAR | SLASH ) factor } ;
factor    = agg | NUMBER | IDENT | LPAREN expr RPAREN ;
agg       = ( "sum" | "count" | "min" | "max" | "avg" ) LPAREN IDENT RPAREN ;
```
- New tokens: `PLUS = "+"  MINUS = "-"  STAR = "*"  SLASH = "/"  EQUALS = "="`. **Ordering caution
  vs step 1:** `EQUALS` (`=`) must come AFTER `GE/LE/EQEQ` so `>=`/`<=`/`==` still win maximal munch;
  `-` (MINUS) vs the NUMBER regex's leading `-` needs a test (a bare `-` between factors must lex as
  MINUS, not get eaten into a negative number — likely fine since NUMBER needs a digit/`.` after `-`,
  but **add a lexer test** `a - 5` → IDENT MINUS NUMBER).
- Precedence via the standard `expr/term/factor` cascade; left-assoc by the `{ }` repetition.
- adapter: `adapt_let` builds the `Expr` tree; lower: evaluate `Expr` → `Derived`, bind by name.
- Regenerate with `cargo run -p adj-lang --bin regen_grammars`; crate must compile first.

## 5. Audit gates (port from `provenance_program.py`, run over the derivation tree)

- **faithfulness** — a `Leaf`'s value equals its cited fact's magnitude (modulo unit; cross-unit ops
  like `mg_dl / mg_dl` cancel, `usd + days` is a **unit error** the gate rejects). Needs the unit to
  travel with the fact (step 2 keeps it on the fact term — read `args[1]` of the typed wrapper).
- **no-magic-numbers** — every `Lit` in a formula is either flagged as a declared structural constant
  (a small allowlist syntax, e.g. `const filing_threshold = 14600` with its own provenance) or it is a
  violation. Thresholds in predicates are policy bytes (provenanced); literals inside `let` formulas
  are the risk surface.
- **coverage** — every quantity-bearing observed slot is either consumed by some `let`/predicate or
  explicitly discarded (the discard read from the MYCIN pipeline). Reuse the closed-vocabulary idea.

Gates run as engine checks over the `Derived` trees + facts, returning structured violations (not
panics). They are **off by default / advisory** in v1; the Haiku run (step 8) turns them into hard
gates.

## 6. Build sub-PRs (each green + security-reviewed + babysat; stack or land after the base merges)

1. **3a — derivation tree + evaluator + binding (logic-engine only).** `DerivationNode`/`Derived`,
   a `compute(expr_over_terms)` evaluator for `+ - * /` over `Number`/typed magnitudes, `observed_value`
   consults the derived table, `FromComputation` proof origin. Unit-tested with hand-built `Expr`. No
   grammar yet — pure engine, lowest risk, mergeable alone.
2. **3b — adj-lang surface (`let` + arithmetic grammar + AST + adapter + lower).** Coupled grammar PR;
   wires 3a. Tokens `+ - * / =`, the expr cascade, `Statement::Let`.
3. **3c — aggregations** `sum/count/min/max/avg(slot)` over repeated `observe slot(v)` facts (this is
   step 4 of DESIGN folded in once arithmetic lands).
4. **3d — faithfulness + no-magic-numbers gates** over the derivation tree; CLI renders the `Op` tree
   under the firing predicate step.

## 7. Invariants (carry from DESIGN §7)

- Model emits no computed numbers — only `let` **formulas** + extracted values.
- Every derived value reconstructable from its `DerivationNode` tree without the model.
- Every `Leaf` cites a `FactId` → `Provenance` → source bytes; every `Lit` is provenanced-or-declared.
- One engine: a predicate over a derived value is still a saturating LR; verdict families from the
  differential. No new verdict logic.

## 8. Open questions to settle at implementation time

- Bind-by-name table vs. synthesizing a real `Fact` for the derived value (a `Fact` reuses
  `observed_value` for free but needs a place to hang the `Derivation`). Leaning: a separate
  `derived: Vec<Derived>` table on the KB + `observed_value` checks it last.
- Integer vs float result typing: keep `Derived.value: f64` for the predicate comparison, but retain
  the exact `Number`/ratio in the tree for the audit render (avoid `40/100` showing as `0.4000001`).
- Unit algebra depth for faithfulness: v1 = same-unit add/sub, free mul/div with unit cancel-or-tag;
  full dimensional analysis is a later gate.
