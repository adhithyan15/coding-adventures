# ADJ-REASON-MATH — Design Spec: adj-lang as the Universal Adjudicatable-Reasoning Substrate

**Status:** Design / specs-first. No code modified.
**Author:** architecture pass, 2026-06-26.
**North star:** An LLM *decomposes* an HLE-class problem into an ADJ program; the
engine *solves* it (deduction + probability + symbolic algebra + exact/dimensional
numeric math + constraints) on the CPU and emits a single machine-checkable
proof/derivation object that an independent checker can re-verify step-by-step and
use to localize any error. **No mathematics is ever performed by the LLM.**

All claims below are cited `file:line` against the repo at
`/Users/adhithya/Downloads/coding-adventures`.

---

## 0. Executive correction (read this first)

The repo has **far more** than expected, and one of my own exploration agents was
wrong about the CAS. The ground truth, verified by reading `handlers.rs` directly:

- **A real, mature symbolic CAS EXISTS.** It does symbolic **differentiation**
  (`derivative_handler`, `symbolic-vm/src/handlers.rs:1493`, registered
  `handlers.rs:7488`) and symbolic **integration** including integration-by-parts
  (tabular IBP `handlers.rs:2693`–`2787`), rational-function integration
  (`integrate_rational_simple_rp` `handlers.rs:4279`), power-of-x, trig, hyperbolic,
  and series cases (`integrate`/`integrate_handler` `handlers.rs:2372`,`2424`,
  registered `handlers.rs:7489`). The earlier "differentiation/integration MISSING"
  claim was a false negative — the agent read only the public re-exports in
  `cas-*/lib.rs` and missed the calculus that lives in `symbolic-vm/handlers.rs`.
- **The constraint solver IS wired into adj-lang end-to-end** — not inside
  `logic-engine`, but at the **CLI** layer (`adj-lang-cli/src/main.rs:412`–`429`),
  which is the actual runtime entry point for a `.adj` program. An agent reported it
  "NOT wired"; that is true only of `logic-engine` itself.

These two corrections materially change the roadmap: the biggest "missing CAS" item
is **not building a CAS** — it is **wiring the existing CAS into the ADJ surface and
giving it a derivation trail**.

---

## Phase 1 — Inventory (what exists, cited)

### 1. adj-lang surface + engine

**Grammar:** `code/grammars/adj_lang.grammar` (+ `adj_lang.tokens`). The surface today
(one production per AST statement, `adj_lang.grammar:22`–`43`):

| Construct | Grammar line | Engine semantics |
|---|---|---|
| `prior N for T` | `:50` | Seeds LR log-odds prior (`PriorClause`) |
| `contributes N from <evidence> to T` | `:52` | Single-source likelihood ratio; `evidence = predicate \| term` (`:62`) |
| `predicate = IDENT (GE\|LE\|GT\|LT\|EQEQ) expr` | `:64` | **Predicate-gated** (deterministic = saturating LR over a CPU comparison; RHS can be arithmetic/LaTeX) |
| `interacts N when T and T … for T` | `:66` | Joint/synergy/explaining-away LR |
| `uncertain {T,…} for T` | `:68` | Uncertainty marker (what would shift the answer) |
| `observe T` | `:70` | Assert a `Certain` Fact |
| `relate rel(a,b)` | `:83` | Ground relational edge → `Fact`; recall via SLD |
| `rule { head: H when: L,… [priority:] [context:] }` | `:105` | Horn clause; `not L` = NAF (`body_literal :107`) |
| `context_order { a > b }` | `:118` | Defeasible context precedence (`add_context_outranks`) |
| `functional pred(...)` | `:129` | At-most-one-value-per-key (conflict driver) |
| `? T` | `:131` | Query (ground → differential; `$Var` → SLD recall) |
| `let x = expr` | `:145` | CPU arithmetic + aggregations → derivation tree |
| `symbol x : T` | `:163` | Declare a constraint unknown |
| `constrain expr relop expr` | `:165` | Linear (in)equality half-plane |
| `solve for {x,…}` | `:169` | Solve constraint system |
| `check` | `:171` | Feasibility decision |
| `minimize/maximize expr` | `:178` | LP objective |
| `dictionary/define/use/rulebook/import` | `:190`–`235` | Controlled vocab + named rulebooks + cross-file composition |
| `source/locator/trust/cites` | `:247`–`263` | Per-clause provenance (incl. ADJ-A9 corroboration) |

**Engine:** `code/packages/rust/logic-engine/`:
- `lib.rs` — `KnowledgeBase`, `Fact`/`Rule` (with `Provenance`, `Priority`),
  `differential`, `enumerate_all`, `enumerate_governing`.
- `enumerate.rs` — SLD resolution + NAF (`enumerate_all`, exported `lib.rs:58`).
- `differential.rs` — LR differential over hypotheses (`differential(...)`
  `differential.rs:152`).
- `lr_aggregate.rs` — the LR aggregation machinery (priors/contributions/joint/
  predicate-gated).
- `proof_dag.rs` — `DerivationOrigin`/`ProofStep`/`Proof`/`ProofDAG` (the proof object).
- `govern.rs` — defeasible precedence resolution.
- `compute.rs` — `let` arithmetic with a `DerivationNode` tree.
- `dimension.rs` — dimensional/unit algebra.
- `provenance.rs` — `Provenance`, `Citation`, `TrustTier`.

**CLI orchestration (the real runtime):** `adj-lang-cli/src/main.rs`. Order
(`main.rs:404`–`429`): run constraint **solve/check/optimize FIRST**
(`:412`–`422`), convert each outcome to a **status atom** and inject it as an
observed `Fact` (`status_certificates` → `kb.add_fact(Fact::certain(atom(status)))`,
`:424`–`427`), THEN run the differential `decide(&lowered)` (`:429`). A clause like
`contributes <lr> from feasible to <verdict>` then fires in the differential — this
is the existing **"feed-a-verdict"** composition (E2) that lets a solver result
influence a probabilistic verdict *through the ordinary contribution machinery, with
no new engine code*. Output is one JSON object with `queries`, `ranked` (each with a
`proof`), `decision`, `recall`, `governing`, `solve`, `check`, `optimize`
(`main.rs:514`–`520`).

#### THE KNOWN GAP (deduction ↔ evidence)
Exact location: **`logic-engine/src/lib.rs:933`–`955`**, the `observed_evidence`
gate. It returns `FactId`s only for **directly asserted `Certain` Facts**
(`lib.rs:947` filters `f.probability == Probability::Certain && f.term == term`).
The doc-comment is explicit (`lib.rs:937`–`942`): *"only `Certain` Facts gate
contributions. Probabilistic Facts and Rule-derived evidence are deliberately not
yet routed here … deferred to v0.2."* The companion limitation is in the proof
object: `DerivationOrigin::FromContribution.evidence_fact_ids`
(`proof_dag.rs:55`–`62`) carries *"Empty if the evidence was satisfied by a Rule head
— the engine does not currently expose Rule provenance for LR-aggregation evidence;
v0.2 will route Rule-derived evidence through this field too."*

**Consequence:** an atom that is only **derivable** via `rule { … }` (multi-hop SLD)
**cannot** trigger a `contributes … from <that atom> to <verdict>` clause, and even
when an evidence term *is* present, its proof is not threaded into the LR step. So
deduction and the LR differential do not compose in a single query. This is the
single most important seam.

### 2. Symbolic math engine (CAS) — IT EXISTS, and it is substantial

**Core IR:** `symbolic-ir/src/lib.rs`, `enum IRNode` (`lib.rs:104`–`140`):
`Symbol`, `Integer(i64)`, `Rational(i64,i64)` (reduced, `denom>0` invariants
`lib.rs:116`–`125`), `Float(f64)`, `Str`, `Apply(Box<IRApply>)`. **Exact** rationals;
integers are `i64` (BigInt explicitly deferred, `lib.rs:99`–`102`). Standard heads
incl. `D` and `INTEGRATE` (constants exist in symbolic-ir).

**Evaluator + CAS:** `symbolic-vm/` — a generic rewrite/eval VM
(`VM::eval`, dispatch architecture `lib.rs:13`–`30`) plus a large handler table
`build_handler_table` (`handlers.rs:7470`+). Verified capabilities:
- **Differentiation** — `derivative_handler` `handlers.rs:1493` (product/quotient/
  chain/power/exp/log rules, registered `:7488`).
- **Integration** — `integrate_handler`/`integrate` `handlers.rs:2372`,`2424`
  (registered `:7489`): linearity, ∫xⁿ (`integrate_power_of_x :5070`), trig,
  hyperbolic, **integration-by-parts (tabular)** `:2693`–`2787`, **rational-function
  integration** (poly long division + log/atan closing, Cases A/B)
  `:4158`–`4435`, polynomial series. Returns an unevaluated `Integrate(...)` node
  when no closed form is found (honest non-closure).
- **Factoring / partial fractions** — `FACTOR`→`factor_handler`, `APART`→
  `apart_handler` (`handlers.rs:7491`,`:7493`).
- **Assumptions store** — `Assume/Forget/ForgetAll` (`handlers.rs:7501`–`7503`),
  e.g. `Assume(x>0); Sqrt(x^2)`.
- **Comparisons + boolean logic + If/Define/List** (`handlers.rs:7470`+).

**Surrounding CAS crates** (all over `symbolic-ir`, maturity per inventory):
- `cas-simplify` (canonical form, numeric fold, identity rules; `simplify()`) — SOLID.
- `cas-solve` (linear/quadratic/cubic/quartic, linear systems, inequalities,
  table-based transcendental, numeric Durand–Kerner) — SOLID for ≤4, PARTIAL above.
- `cas-substitution` (`subst`, `replace_all` — **evaluate symbolic at a value**) — SOLID.
- `cas-multivariate` (Buchberger Groebner, ideal solve, reduction) — PARTIAL.
- `cas-matrix` (det/inverse/charpoly/eigen/LU/rowreduce/subspaces, exact) — PARTIAL (cofactor-bound).
- `cas-factor` + `cas-algebraic` (univariate Z factoring; quadratic extensions) — PARTIAL.
- `cas-limit-series` (direct limits, polynomial Taylor) — PARTIAL.
- `cas-summation` (Gosper), `cas-trig` (special angles, expand/reduce) — SOLID/PARTIAL.
- `cas-complex`, `cas-laplace`, `cas-fourier`, `cas-number-theory`,
  `cas-pattern-matching`, `cas-ode`/`cas-ode-numeric`, `cas-pretty-printer`.
- **Text frontends already exist:** `macsyma-runtime` / `maxima-runtime` /
  `wolfram-runtime` parse source strings → symbolic-ir and eval through the VM
  (`macsyma-runtime/src/lib.rs:24`–`31` reuses `symbolic_vm::{VM, build_handler_table}`).

**What the CAS CANNOT do today:**
- **No step-by-step rewrite/derivation trail.** The VM produces only final forms; the
  "step"/"history" grep hits in `cas-simplify`/`symbolic-vm` are iteration counters,
  not an emitted provenance trail. `cas-pretty-printer` formats final expressions
  only. **This is the key CAS gap for HLE auditing.**
- **No BigInt** (i64 overflow → Float in places).
- **Not wired into adj-lang at all.** `adj-lang/Cargo.toml` and
  `logic-engine/Cargo.toml` have **zero** `cas-*`/`symbolic-*` dependency edges
  (verified). The CAS is reachable only from Rust / the macsyma-family REPLs.

### 3. Value-based / numeric math engine

`logic-engine/src/compute.rs` (header `compute.rs:1`–`22`): evaluates `let`
formulas on CPU into a **`DerivationNode` tree** (`ComputeOp` `:56`, `ComputeExpr`
`:88`, `DerivationNode` Leaf/DerivedRef/Lit/Op `:105`–`134`, result `Derived`
`:144`). Aggregations Sum/Count/Min/Max/Avg reduce *every* observation of a slot.
Errors (`ComputeError` `:155`) are surfaced into the audit (`lower.rs:113`–`115`).
**Provenance through math already works** — each leaf points back to the source fact.
Arithmetic is `f64` (not exact rationals — a gap vs. the symbolic IR's exact
rationals).

`logic-engine/src/dimension.rs` (`:1`–`56`): `enum Dimension` (Scalar/Money/Unit/
Percent…) with `combine` enforcing add/sub same-dimension, mul/div composite-unit
algebra; `dimensioned_value` infers dimension from typed wrappers. `datetime.rs` +
`conversion.rs` handle dates/durations/unit conversion. `numeric_magnitude`
(`lib.rs:979`) extracts the leading number of a typed value for predicate compares.

### 4. Constraint + solver backends

`adj-constraint-solver/src/lib.rs`:
- `solve()` `:68` — exact Gaussian over rationals for square `=` systems; returns
  `SolveOutcome::Solved{from_constraints}` (provenance indices `:41`).
- `solve_univariate_poly()` `:101` — closed-form roots via `cas-solve`.
- `check()` `:194` — feasibility: integer via `LiaTactic` (Cooper, delegated to
  `constraint-engine` `:219`) + real via **Fourier–Motzkin** (`real_feasibility`
  `:701`, `fourier_motzkin` `:801`); witness verification `:838`.
- `minimal_unsat_core()` `:290` — **IIS** (deletion filter).
- `optimize()` `:1037` — integer (binary-search `optimize_integer :1088`) + real LP
  (FM projection); `OptimizeOutcome::Optimal{binding}` / `Infeasible{core}`
  `:1000`–`1022`.
- `solve_setcover_sat()` `:1147` — Sinz-encoded set-cover via SAT.

**Invocation from ADJ:** wired at the **CLI** (`adj-lang-cli/main.rs:412`–`422`
calls `solve/check/optimize`), not in `logic-engine`. So a `.adj` program run through
the CLI *does* solve constraints and feed verdicts. `constraint-core`/
`constraint-engine`/`constraint-vm` are the lower SAT/LIA/SMT stack; `adj-constraint-
solver` reuses `constraint-engine` (`:29`,`:219`,`:1147`). **Audit trail:** IIS core
(infeasible), binding constraints (optimal), witness (feasible) — index-based
provenance back to source constraints. No SAT-fragment UNSAT cores.

---

## Phase 2 — Language + engine evolution

Design principle throughout: **every new construct threads `Provenance` and a
machine-checkable step into the one proof object.** The proof object becomes the
union of: SLD steps (`FromFact`/`FromRule`), LR steps (`FromPrior`/`FromContribution`/
`FromJoint`/`FromPredicate`), CAS rewrite steps (**new**), numeric `DerivationNode`s,
and solver certificates (IIS / binding / witness).

### A. Deduction ↔ evidence bridge (closes the known gap) — **engine seam, highest leverage**

**Problem:** `observed_evidence` (`lib.rs:943`) only matches asserted `Certain`
Facts; rule-derived atoms can't gate contributions, and rule provenance isn't
threaded into LR steps (`proof_dag.rs:55`–`62`).

**Design:** change the evidence gate from "is this term asserted?" to "is this term
**provable**?", reusing the existing SLD engine.
1. Replace the body of `observed_evidence` with: first try the asserted-`Certain`
   fast path (unchanged); if empty, call `enumerate_all` on the evidence term. If a
   proof exists, the evidence holds.
2. Generalize the return type so a contribution can record *either* `FactId`s *or* a
   `Proof` (the SLD derivation that established the evidence). Extend
   `DerivationOrigin::FromContribution` with an optional
   `evidence_proof: Option<Box<Proof>>` (additive; existing readers ignore it).
3. **Attenuated confidence through chains.** A derived evidence atom carries a
   confidence ≤ 1 = the product of the rule/fact probabilities along its proof
   (`Rule::with_probability` already exists, `lib.rs:299`). The LR step multiplies
   its `log(LR)` by that confidence (so a weakly-derived premise contributes a
   weaker shove). For `Certain` facts the factor is 1 — fully backward compatible.
4. **Provenance flows** by taking the proof's per-step `Provenance` (every `FromRule`/
   `FromFact` already references clauses that carry `Provenance`) and surfacing the
   `min` trust tier across the chain (the `TrustTier: Ord` reduction is exactly what
   `provenance.rs:79`–`84` was built for).

**Surface:** *no grammar change.* `contributes 10 from infection_present to sepsis`
already parses; today `infection_present` must be `observe`d. After the bridge, if
`rule { head: infection_present when: positive_culture, fever }` derives it from
per-case facts, the contribution fires and the proof shows the derivation that
licensed it. This is the unification that makes deduction + probability one query.

**Cost:** small, localized to `lib.rs:943` + `lr_aggregate.rs` evidence lookup +
`proof_dag.rs` (one optional field). High risk-adjusted leverage.

**Implementation note (2026-06-28):** the core bridge is now implemented in
`logic-engine`: direct `Certain` facts remain the fast path, otherwise
`observed_evidence` enumerates SLD proofs, selects the strongest proof, attenuates
the LR delta by the fact/rule probability product, and stores the nested proof under
the LR step. `adj-lang-cli` renders that nested `"evidence_proof"` so the `.adj`
program path can show the deduction that licensed the probabilistic contribution.
The later unified-proof pass can still collapse trust-tier summaries and verifier
checks across SLD + LR + solver/CAS steps.

### B. Symbolic algebra as first-class ADJ — **wiring + new trail (CAS already exists)**

**B1 — surface.** Add statements that name the existing CAS operations. New grammar
productions (additive to `statement`, `adj_lang.grammar:22`):
```
math_decl   = "math" IDENT EQUALS math_expr ;            # bind a symbolic expr
solve_sym   = "solve_symbolic" math_expr "for" IDENT ;   # equation solving
simplify_d  = "simplify" math_expr ;
diff_decl   = "differentiate" math_expr "wrt" IDENT [ "order" NUMBER ] ;
integ_decl  = "integrate" math_expr "wrt" IDENT [ "from" math_expr "to" math_expr ] ;
prove_id    = "prove_identity" math_expr EQEQ math_expr ;
eval_at     = "evaluate" IDENT "at" "{" IDENT EQUALS math_expr {COMMA …} "}" ;
```
`math_expr` is a small infix grammar (the macsyma/maxima parsers already exist and
lower to `symbolic-ir` — reuse, do **not** hand-write; per repo rule "no handwritten
lexers/parsers"). Concrete example:
```
math f = x^3 - 6*x^2 + 11*x - 6
differentiate f wrt x            # engine: 3*x^2 - 12*x + 11
solve_symbolic f == 0 for x      # engine: {1, 2, 3}, factored proof
```

**B2 — engine semantics.** Lower `math_expr` → `IRNode`, call `symbolic-vm`
`VM::eval` (differentiate/integrate/simplify) or `cas-solve` (`solve_symbolic`).
Add `adj-lang-cli` deps on `symbolic-ir`, `symbolic-vm`, `cas-solve` (the first
`cas-*` edges into the ADJ stack).

**B3 — the missing audit trail (NET-NEW, the real CAS work).** The VM emits only
final forms. Add a **rewrite-provenance channel**: a `RewriteStep { rule_name:
&'static str, before: IRNode, after: IRNode, at_path: Vec<usize> }` log. Thread an
optional `&mut Vec<RewriteStep>` through `VM::eval` / the handler signature so each
named rewrite (product rule, IBP, ∫xⁿ, identity x+0→x) appends one step. This is a
**checkable** trail: an independent checker re-applies each named rule to `before`
and asserts it yields `after`, and that the chain's endpoints match the query. This
is the CAS analogue of the proof DAG and the heart of "catch any error." Map each
`RewriteStep` into a new `DerivationOrigin::FromRewrite{rule_name, before, after}` so
the **single** proof object carries algebra steps alongside deductive ones.

### C. Exact + dimensional numeric math — **make compute exact + interoperate with CAS**

1. **Exact arithmetic in `compute.rs`.** `let` math is `f64` today; the symbolic IR is
   exact rational. Add an exact mode: `DerivationNode::Op` carries an exact
   `Rational` result when all leaves are integers/rationals, falling back to `f64`
   only on irrational ops. Reuse `symbolic-ir::Rational` so value-math and
   symbol-math share one number type.
2. **Dimensional checking on `let`.** `compute.rs` already has the value; route every
   `Op` through `dimension.rs::combine` so `usd + days` is a clean audit error, not a
   silent number. (Seam: `dimension.rs` exists; just call it from `compute`.)
3. **Symbolic ↔ value interop.** `evaluate <symbolic> at {x=…}` lowers to
   `cas-substitution::subst` then `simplify`/numeric fold — *the* bridge from a
   symbolic solution to a value, with the substitution recorded as `RewriteStep`s.
   Conversely a `let` value can be injected as an `IRNode::Rational` leaf into a
   `math_expr`. This makes "solve symbolically, then evaluate at the case's numbers,
   dimension-checked" a single audited flow (the physics worked example below).

### D. Constraint/optimization as first-class queries — **mostly done; unify under one `ask`**

The CLI already calls solve/check/optimize and feeds verdicts. Remaining work is
*surfacing* them uniformly under the typed ASK surface (§E) and **promoting the
nonlinear seam**: `solve_univariate_poly` (`adj-constraint-solver:101`) already bridges
to `cas-solve`; extend the constraint sublanguage so a `constrain` may reference a
`math_expr` (nonlinear residual) routed to the CAS, with the CAS rewrite trail
appended to the optimality/feasibility certificate. Emit the IIS / binding / witness
as proof-object nodes (`DerivationOrigin::FromSolver{certificate}`) so a checker
re-verifies feasibility from the witness and infeasibility from the minimal core.

### E. Typed QUESTION / ASK surface + unified proof object — **the HLE lever**

**Surface.** One `ask` statement carries the *answer shape* so the LLM decomposer
has a fixed target and the engine knows which solver to run and what to verify:
```
ask compute    <math_expr>                  # → numeric/exact value + DerivationNode
ask solve      <eq> for <var>               # → symbolic roots + factor/rewrite trail
ask prove      <goal>                        # → SLD/precedence proof (boolean)
ask most_likely among {h1,…}                 # → differential ranking + LR proof
ask optimal    (minimize|maximize) <expr>    # → assignment + binding/IIS certificate
ask explain    <conclusion>                  # → the full proof DAG, no recompute
```

**Unified proof object.** Generalize `ProofDAG` (`proof_dag.rs:147`) into a
`ReasoningTrace` whose steps are the closed sum:
```
StepKind = FromFact | FromRule                       # deduction (exists)
         | FromPrior | FromContribution | FromJoint | FromPredicate   # probability (exists)
         | FromRewrite { rule_name, before, after }  # algebra (B3, new)
         | FromCompute { DerivationNode }            # numeric (C, exists, lift in)
         | FromSolver  { certificate }               # constraints (D, lift in)
```
Every step already (or will) carry `Provenance`. **The checkability invariant:** a
standalone re-checker, given the `ReasoningTrace` and the KB, can re-verify each step
independently —
- `FromRule`/`FromFact`: re-run unification;
- `FromContribution`: re-multiply `log(LR)`, confirm the cited evidence is provable;
- `FromRewrite`: re-apply the named rule to `before`, assert `== after`;
- `FromCompute`: re-evaluate the `DerivationNode` exactly;
- `FromSolver`: re-check the witness against constraints / the IIS minimality.
The first step whose re-check fails **localizes the error** to a single clause +
citation. This is the machine that "catches any error."

### F. The decomposition contract — **enforced by the existing faithfulness/coverage gate**

The LLM is allowed to emit **problem structure only**:
- `relate`/`rule`/`prior`/`contributes`/`interacts` (the model's *reading* of the
  premises, each `source/locator`-cited),
- `symbol`/`constrain`/`math`/`define`/`dictionary` (the unknowns, the equations as
  written, the vocabulary),
- one `ask …` naming the answer shape.

The LLM is **forbidden** from emitting: any computed number, any solved root, any
simplified/differentiated/integrated form, any posterior, any feasibility verdict.
**Enforcement** reuses the framework's existing total-coverage discipline (ADJ02 "every
token represented or lowered", `feedback_no_byte_arithmetic_for_llm`): the
faithfulness gate already (a) requires every clause to cite source bytes and (b)
forbids LLM-computed offsets. Extend the gate with a **"no-result-literals" check**:
reject a decomposition if a `math`/`constrain`/`ask` RHS contains a literal that is
*not* present in (or directly lowered from) the cited source span — i.e. the model
tried to pre-compute. The engine's result must come from the engine, and the proof
object proves it did (every numeric leaf traces to either a source byte or a
`FromCompute`/`FromRewrite`/`FromSolver` step).

---

## Phase 3 — Staged roadmap (specs-first, one PR per slice)

Legend: **[seam]** small, localized; **[new]** net-new build. Each PR ships with a
spec under `code/specs/`, tests >80%, CHANGELOG, README per repo convention.

| PR | Title | Kind | Depends on | Notes |
|---|---|---|---|---|
| **1** | **Deduction→evidence bridge** (A): `observed_evidence` falls back to SLD; attenuated confidence; rule provenance into LR steps | **[seam]** | — | **HIGHEST LEVERAGE. Do this first.** No grammar change; unifies the three engines; closes the documented v0.2 gap (`lib.rs:937`, `proof_dag.rs:60`). |
| 2 | Unify proof object into `ReasoningTrace` with `StepKind` enum; lift `FromCompute`/`FromSolver` into it (E, structural only) | [seam] | 1 | Pure refactor of `proof_dag.rs`; sets the shape everything else fills. |
| 3 | Exact arithmetic + dimensional checking in `compute.rs` (C1, C2) | [seam] | — | Reuse `symbolic-ir::Rational` + `dimension.rs::combine`. Parallel to 1. |
| 4 | CAS rewrite-provenance channel: `RewriteStep` + `&mut log` through `VM::eval`/handlers (B3) | **[new]** | — | The real CAS work; standalone in `symbolic-vm`. Parallel to 1/3. |
| 5 | `ask` surface + typed answer shapes (E); CLI routes each shape; faithfulness "no-result-literals" gate (F) | [seam] | 2 | Grammar additions + CLI dispatch + gate. |
| 6 | Symbolic surface: `math`/`differentiate`/`integrate`/`simplify`/`solve_symbolic` lowering to `symbolic-vm`/`cas-solve` (B1, B2); first `cas-*` dep edge into adj | **[new]** | 4,5 | Reuse macsyma/maxima parser for `math_expr`. |
| 7 | Symbolic↔value interop: `evaluate <expr> at {…}` via `cas-substitution` (C3) | [seam] | 6 | Bridges symbolic solution → dimensioned value. |
| 8 | Nonlinear constraints via CAS + solver certificates into `ReasoningTrace` (D) | [new] | 2,6,8? | Extend `constrain` to accept `math_expr`; lift IIS/binding/witness as steps. |
| 9 | Independent re-checker binary (`adj-verify`): re-verifies every `StepKind`, localizes first failure | [new] | 2,4,5 | The "catch any error" tool; the HLE-audit deliverable. |
| 10 | BigInt in symbolic-ir / compute (overflow-safety) | [new] | 3,4 | Lower priority; gates only large-number HLE items. |

**Single highest-leverage first PR: PR 1 (deduction→evidence bridge).** It is a small
seam at a documented gap, requires no grammar change, is fully backward compatible
(confidence factor = 1 for `Certain`), and it is the structural unification that makes
"deduce a premise, then weigh it probabilistically, with one cited proof" expressible
— the precondition for almost every multi-step HLE question.

### Worked examples expressible at the end

**(1) Calculus (pure symbolic, audited).**
```
math f = x^3 - 6*x^2 + 11*x - 6   source "problem statement" locator "line 1"
ask solve f == 0 for x
```
→ `cas-solve` factors `f = (x-1)(x-2)(x-3)`; `ReasoningTrace` has `FromRewrite`
steps {factor, rational-root} ending in roots `{1,2,3}`; `adj-verify` re-applies each
rule. No number was emitted by the LLM.

**(2) Mixed symbolic + numeric + units (physics).**
```
math v = u + a*t                      source "kinematics premise" locator "p.2"
symbol u : quantity(m_s) ; symbol a : quantity(m_s2) ; symbol t : duration(s)
evaluate v at { u = quantity(0, m_s), a = quantity(9.8, m_s2), t = duration(3, s) }
ask compute v
```
→ `cas-substitution::subst` then exact/dimensional `compute`: `m_s2 · s = m_s`
(checked via `dimension.rs::combine`), result `quantity(29.4, m_s)`; trace shows the
substitution `FromRewrite`s + the `FromCompute` derivation tree; a `usd + s` typo
would be a localized audit error, not a wrong number.

**(3) Multi-hop reason-with-uncertainty.**
```
prior 0.2 for bacterial_meningitis     source "Tunkel 2004" trust authoritative
rule { head: csf_suggestive when: csf_neutrophilic, csf_low_glucose }
                                        source "IDSA 2024" locator "§3.2"
contributes 12 from csf_suggestive to bacterial_meningitis   source "Tunkel 2004"
observe csf_neutrophilic ; observe csf_low_glucose
ask most_likely among { bacterial_meningitis, viral_meningitis }
```
→ PR 1 lets the **derived** `csf_suggestive` (proved by the rule from the two
observations) gate the `contributes` clause; the differential ranks bacterial; the
`ReasoningTrace` interleaves `FromRule(csf_suggestive)` → `FromContribution(LR 12,
evidence_proof=…)` → posterior, every step cited (Tunkel 2004 §, IDSA 2024 §3.2),
trust tier = `min` across the chain. `adj-verify` re-runs each step.

---

## Brutally honest scope summary

- **A real symbolic engine EXISTS and is mature** (diff + integration incl. IBP and
  rational-function integration, factoring, partial fractions, assumptions, solvers,
  exact rationals; `symbolic-vm/handlers.rs`). It is **not wired into adj-lang** and
  **has no derivation trail**. So the CAS work is *wiring + a rewrite-provenance
  channel + a re-checker*, **not** building a CAS from scratch.
- **The biggest genuinely-missing pieces** are: (1) the deduction↔evidence bridge
  (engine seam, documented gap); (2) the CAS rewrite-provenance trail (net-new but
  contained); (3) the unified `ReasoningTrace` + independent re-checker; (4) exact +
  dimensional `let` math; (5) the `ask`/decomposition contract + the no-result-literals
  gate.
- **What already works end-to-end** through the CLI: deduction (SLD+NAF), the LR
  differential with a proof DAG and per-clause provenance, defeasible precedence,
  relational recall, `let` numeric math with a derivation tree, dimensional algebra,
  and constraint solve/check/optimize feeding verdicts with IIS/binding/witness
  certificates.
