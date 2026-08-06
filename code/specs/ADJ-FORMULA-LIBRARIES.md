# ADJ-FORMULA-LIBRARIES — the graded, byte-provenanced *formula* standard library (the real ladder)

**Status:** Spec-first. Reframes and grounds the "standard library" that
[ADJ-LADDER](ADJ-LADDER.md) always named but never built. Supersedes the
inline-`items.json` rung treadmill (rungs 0–125): those stay as a *question bank*, but
the arithmetic they contain is **harvested into importable libraries** here.
**Author:** direction-correction pass, 2026-07-11.

**North star (unchanged, now made real):** a small, non-frontier model (Gemma-/Haiku-class,
fully local) passes a Medical Licensing Exam the model *alone* cannot — because the
**reasoning lives in the framework**: a **content-addressed standard library** of grounded
formulas the model *imports* rather than re-derives, and the engine computes over the CPU,
exactly, with **zero math performed by the LLM**. Built as a curriculum that climbs from
**what a kindergartner learns → medical school**, MLE as the apex you build *up to*.

---

## 1. What went wrong, precisely

ADJ-LADDER's Arm B was supposed to be: *the model decomposes a question into an ADJ program
that imports a grounded library; the engine computes.* In practice every rung shipped a
**self-contained** program that **inlines** the formula:

```
observe motor_response(18)
observe baseline_amplitude(7)
observe decrement_drop(3)
observe facilitation_gain(2)
let answer = motor_response / (baseline_amplitude - decrement_drop + facilitation_gain)   % ← inlined, per item
? answer
```

Nothing is a library; nothing is imported; no term enters a dictionary with provenance; the
model would be *handed* the formula rather than **recalling** it. So the "ladder" degenerated
into enumerating denominator arrangements (`a/(b-c+d)`, `a/(b-c-d)`, …) — 125 rungs proving the
same narrow claim ("the engine can evaluate arithmetic shape N"), with **no reusable asset** and
no exercise of the import-and-recall step the thesis depends on.

**Root cause (confirmed against the grammar):** ADJ has `dictionary`/`define` (terms),
`rulebook`/`relate` (ground relational facts — the 127 recall libraries), `import`/`use`, and
inline `let <name> = <expr>` — but **no construct for a reusable, parameterized formula that
lives in a library and is imported and applied.** The formula could only ever be inline. Fixing
that is rung-0 of the *real* ladder.

---

## 2. The hypothesis this spec operationalizes

> Most fields decompose into a series of **terms** (a dictionary). Terms with a mathematical
> foundation compose into **formulas** with variables. Expressed in ADJ, a formula becomes an
> **importable library**. At answer time the small model does only two things: **decompose the
> input with byte-provenance** and **bind the variables** to the extracted values. The ADJ
> engine then imports the library and **reasons over the CPU**, emitting an answer (or abstains).

Two consequences the design must honor:

1. **Write-once, use-many & compositional.** A clinical formula (e.g. Cockcroft–Gault
   creatinine clearance) *imports* lower-level libraries (a `ratio`, a `product`, a
   unit-conversion) rather than restating them. The curriculum is a DAG, not a flat list.
2. **The libraries themselves carry byte-provenance.** A formula is a *claim about the world*
   ("BMI is defined as mass ÷ height²"). It is not human-asserted; it is grounded in a citable
   source span with a trust tier — exactly like a `relate` edge. At answer time the engine can
   show the **full chain**: the formula's cited definition **+** the model's byte-provenance for
   every bound variable. (Ties to the standing rule: *nothing enters the CAS human-authored;
   facts arrive spider→provenance→adversarial-gate.*)

The scope is **elementary school → medical school**, and it is *not only numeric formulas*.
"Term with a mathematical foundation" is the narrow case; a term can also be a **fact** (water is
two hydrogen + one oxygen), a **symbolic relationship** (solve `v = IR` for `R`), or a
**probabilistic** one. Each is an importable, provenanced library; the curriculum layers them.

---

## 2A. The modalities — ADJ is Prolog + ProbLog + a full CAS + a constraint solver

The libraries exercise **every** faculty the ADJ engine has (or will, once the CAS is wired), so a
med-school question that mixes a recalled fact, an algebraic rearrangement, a numeric plug-in, and a
feasibility check is answered by *composing libraries*, not by one monolith.

| Modality | What a library expresses | ADJ surface | Substrate status |
|----------|--------------------------|-------------|------------------|
| **Relational (Prolog)** | grounded facts & relations — `water` ⟶ 2 `hydrogen` + 1 `oxygen`; anatomy, taxonomy, physiology edges | `rulebook` / `relate` + SLD binding query | **exists** (the 127 recall libs) |
| **Numeric formulas** | plug-in-the-numbers compute — BMI, FENa, dose = mg/kg × weight | `formulabook` / `formula` (§3) | **rung-0, this spec's FL-2** |
| **Symbolic (CAS)** | algebra & calculus *symbolically* — rearrange a formula, `solve`, `simplify`, differentiate, series | a `symbolic`/CAS surface wired to the engine | **engine exists (`symbolic-vm`+`cas-*`) but UNWIRED — substrate rung §3A** |
| **Constraints** | (in)equalities & optimization — `constrain … <relop> …`, `solve for {…}`, `check` satisfiability, minimize/maximize; word problems, systems, dosing feasibility, `INFEASIBLE` first-class | `constrain` / `solve` / `check` (already keywords) → `adj-constraint-solver` → `constraint-vm` | **first-class & wired**; the *solver VM* may need capability growth (tactics/nonlinear) — deepen, not wire |
| **Probabilistic (ProbLog)** | likelihood-weighted facts / evidence, honest uncertainty & abstention | `prior` / `likelihood` / `contributes` | **exists** (uncertainty primitive) |

Note the asymmetry: **constraints are already first-class in the language and wired to a solver VM**
(the AST has `Constrain{lhs relop rhs}`, `solve for {…}`, `check`, minimize/maximize); the open work
there is the *VM's* solver breadth/robustness, tackled as its own capability rung when a curriculum
library needs a tactic the VM lacks. **CAS is the reverse** — a complete engine that is not yet
reachable from the language (§3A). Both are engines to *compose/strengthen*, never to rebuild.

**Relational fact libraries need no new substrate.** "Water = 2H + 1O" is a `relate` library today
(relations already carry arity; a numeric multiplicity is an argument or a small structured term).
Foundational-science fact libraries (elements, compounds, cell biology, anatomy) sit alongside the
recall libraries on the existing mechanism — they are *content*, gated by the same provenance write-gate.

---

## 3A. The CAS substrate — wire the existing engine, don't rebuild it

Symbolic math is the one modality **not yet reachable from the language**. The workspace already
ships a **complete CAS**: `symbolic-ir` + `symbolic-vm` (a *policy-free* evaluator with a `Backend`
trait, explicitly built to be embedded) and the `cas-*` suite — `cas-solve`, `cas-simplify`,
`cas-algebraic`, `cas-substitution`, `cas-factor`, `cas-summation`, `cas-trig`, `cas-matrix`,
`cas-multivariate`, `cas-ode`, `cas-laplace`, `cas-fourier`, `cas-limit-series`, `cas-number-theory`,
… But **`adj-lang` depends on none of them.** A separate rung (sequenced *after* rung-0 so the numeric
loop is proven first) adds a `symbolic`/CAS surface to adj-lang that **embeds `symbolic-vm`** (adj-lang
supplies a `Backend`) — turning a provenanced `formula` into something the engine can *rearrange and
solve*, not merely evaluate. This is wiring an existing, tested engine to the language, not writing a CAS.
It unlocks the algebra/calculus layers of the curriculum (solve for an unknown, symbolic simplification,
symbolic → numeric once variables bind). Same provenance discipline: a symbolic identity is a sourced claim.

---

## 3. Rung-0 — the substrate: `formulabook` + `formula name(params) = expr`

A new top-level construct, a sibling of `dictionary` and `rulebook`, importable via `use`/`import`.
(A third sibling, the `table` construct for native tabular reference data — unit conversions,
reference ranges, dose charts — is specified in [`ADJ-TABLES`](ADJ-TABLES.md) (RS-5); a looked-up
table value composes into a `formula` exactly as an observed slot does.)

```adj
% code/…/stdlib/clinical/bmi.adj — the body-mass-index formula library.
dictionary bmi_vocab {
    define body_mass : quantity(mass)    surface "weight", "body weight", "mass"
    define height    : quantity(length)  surface "height", "stature"
    define bmi       : quantity          surface "body mass index", "BMI"
}

formulabook body_metrics {
    use bmi_vocab

    formula bmi(body_mass, height) = body_mass / (height * height)
        source "BMI is defined as body mass divided by the square of body height … kg/m²."
        locator "https://www.who.int/…/body-mass-index"
        trust authoritative
}
```

### 3.1 Grammar (additive; regenerated via `regen_grammars`, never hand-edited)

- `formulabook <ident> { use <dict>… <formula>… }` — a named collection; `use` brings a
  dictionary's terms into scope so the formula's parameter names are typed vocabulary, not free
  strings.
- `formula <name>(<param>, …) = <expr>` — `<expr>` is **the existing `let` expression grammar**
  (`ExprAst`: `Num`, `Ref`, `Bin(ArithOp,…)`, `Call(NamedFn,…)`, aggregations, the transcendental
  set). The **only** new idea: the leaves that name a `<param>` are **formal parameters**, bound at
  apply time, instead of `Ref`s to already-`observe`d facts.
- Each `formula` carries the **same provenance envelope as `relate`**:
  `source "<byte-quotable span>" locator "<url/isbn/loc>" trust <tier>`
  (AST `Cites { source, locator }` + `TrustTierName`). Provenance is **required** on a `formula`
  in a shipped library — the linter rejects an unsourced formula, mirroring the recall-library gate.

### 3.2 AST & lowering

- `FormulaDef { name, params: Vec<Term>, body: ExprAst, cites: Cites, trust: TrustTierName }`.
- The body is a **parameterized `ExprAst`**: a leaf `Ref(p)` where `p ∈ params` is a *parameter
  reference*; any other `Ref` must resolve to a term/fact in scope (a validation error otherwise).
- **Apply semantics** (`? bmi(body_mass, height)` in a consumer): the engine binds each parameter to
  the correspondingly-named `observe`d fact (or to the argument expression), substitutes into the
  body, and evaluates it through the **existing `ComputeExpr` evaluator** — the same exact/dimensional
  CPU path `let` already uses. No new evaluator; a formula is a *named, importable, reusable `let`*.
- The derivation tree records: the `FormulaDef`'s `cites`/`trust`, plus each parameter's binding and
  *its* provenance (the observed fact's source span). One computed answer ⇒ one auditable chain.

### 3.3 Consumer surface (what the model writes at answer time)

```adj
import "stdlib/clinical/bmi.adj"

observe body_mass(quantity(70, kg))     % ← bound by the model from byte-provenance decomposition
observe height(quantity(1.75, m))       % ← "…height of 1.75 m…"  (span cited)

? bmi(body_mass, height)                % engine imports the library, applies the cited formula → 22.86 kg/m²
```

The model states **no arithmetic** — it recalls *which* library applies and binds the variables.
The formula (and its source) live in the library; the numbers (and their spans) come from the input.

### 3.4 Non-goals for rung-0

No recursion, no higher-order formulas, no user-defined control flow. A `formula` is a pure,
total, parameterized expression over the already-supported op set — plain arithmetic, or (FL-8,
§3B) a single trailing comparison. Piecewise/threshold behavior stays in the existing
`contributes … from <pred>`/`constrain` machinery, composed *around* formulas, not baked into them.

---

## 3B. FL-8 — comparison formulas (`a relop b`)

**Status: shipped.** The K-2 curriculum's `compare` family (§4's K/early MATH-track row) needs a
citable, importable, `?`-queryable "is A greater than B" — and rung-0 as originally specified
couldn't express it: `formula`'s body was arithmetic-only (`+ - * /` and friends), while the
language's *only* comparison surface, `constrain`/`check`, has no provenance tail (a `constrain`
statement cannot carry `source`/`locator`/`trust`) and isn't reachable via `?` — it is a scratch-pad
solver for one program's own numbers, not a library format. Enumerating every valid pair as `table`
rows was the fallback, but it doesn't generalize (a bounded 1-10 comparison is 45 rows; nothing
about "greater than" is actually enumerable knowledge).

**The fix, additive, no new construct:** `formula_body`'s final expression — `formula_relation` —
is now `expr [ relop expr ]` (`code/grammars/adj_lang/adj_lang.grammar`), reusing the exact `relop`
`constrain`/`sm_guard` already parse. Every formula shipped before this rung still parses unchanged
(the trailing comparison is optional). A comparison formula carries the **identical**
`source`/`locator`/`trust`/`quote` provenance envelope as an arithmetic one, and is applied and
queried through the **same** path:

```adj
% code/…/stdlib/mathematics/comparison.adj
formulabook comparisons {
    formula greater_than(a, b) = a > b
        source "A quantity a is said to be greater than b if a is larger than b, written a>b."
        locator "https://mathworld.wolfram.com/Greater.html"
        trust authoritative
}
```

```adj
import "comparison.adj"
observe a(5)
observe b(3)
? greater_than(a, b)          % engine → 1 (true), carrying the cited definition
```

**Semantics:** the result is a dimensionless **1** (true) / **0** (false), always **exact** when
both operands carry an exact-rational sidecar — a comparison is exactly decidable whenever the
value it compares is, so it is never approximated through `f64` first. Dimensionally a comparison
combines like addition (both operands must share a dimension — `5 kg > 3 usd` is the same category
error as `5 kg + 3 usd`, a clean `DimensionMismatch`, never a silently-wrong answer) but the
**result** collapses to `Scalar` rather than carrying that shared dimension, mirroring how
`ComputeOp::Sign` already collapses a dimensioned unary operand to a dimensionless magnitude. A
comparison formula composes into further arithmetic like any other formula application (its `1`/`0`
is an ordinary `Ref`-able value), and into the existing `contributes … from <app> <op> <thr>`
branch-on-formula machinery — comparisons and piecewise thresholds remain two different tools, not
merged.

**What did NOT change:** `ExprAst`, `ComputeExpr`, and every existing operator/variant are
byte-identical; the change is purely additive (`ExprAst::Compare`, six new `ComputeOp::Cmp*`
variants). No CAS-provenance/byte-pinning code was touched — a comparison formula ships at
`status.provenance: source_labeled` exactly like every other freshly-authored formula; flipping it
to `fully_verified` is ordinary Wave 1 migration work, unblocked but not done by this rung.

---

## 3C. FL-9 — `floor(x)`/`mod(a, b)` on the plain-arithmetic surface

**Status: shipped.** Base-ten place value's DECOMPOSE direction (a two-digit number → its tens
and ones digits, CCSS 1.NBT.B.2's other half — the COMPOSE direction shipped in
`mathematics/place-value.adj`) needs integer floor and modulo: `tens(n) = floor(n / 10)`,
`ones(n) = mod(n, 10)`. Both already exist engine-side — `ExprAst::Floor` and
`ArithOp::Mod`/`ComputeOp::Mod` are used today by the `latex "…"` frontend (`\lfloor x\rfloor`,
`a \bmod b`) — but neither was reachable from the **plain** (non-LaTeX) arithmetic surface a
`formula` body normally uses, so a stdlib formula could not express "how many tens" without
dropping into LaTeX for one sub-expression.

**The fix, additive, no grammar change:** `floor`/`mod` join `RUNTIME_BUILTIN_FORMULAS`
(`code/packages/rust/adj-lang/src/lower.rs`) — the same recognized-by-name built-in mechanism
`round_to`/`round_sig`/`to_scientific`/`to_percent`/`to_currency` already use. The plain grammar's
`factor` production already includes `apply` (an ordinary `name(args, …)` call), so no
`.grammar`/`.tokens` edit or parser regen is needed — `expand_rec` recognizes the name **before**
consulting the user-formula map and maps it directly onto the existing node:

```adj
formula tens(n) = floor(n / 10)
formula ones(n) = mod(n, 10)
```

`floor(x)` takes exactly one argument and lowers to `ExprAst::Floor`; `mod(a, b)` takes exactly
two and lowers to `ExprAst::Bin(ArithOp::Mod, a, b)` — both pre-existing nodes, so no new
`ExprAst`/`ComputeOp` variant, no new exhaustive-match site, and no change to `plan_expr`'s JSON
export or the CAS-provenance replay surface. `RUNTIME_BUILTIN_FORMULAS`'s existing reserved-name
collision check (`LowerError::ReservedFormulaName`) covers `floor`/`mod` automatically, since it
already consults this same list — a `formulabook` declaring its own `floor`/`mod` formula is
rejected the same way declaring its own `round_to` already is.

---

## 3D. FL-10 — rung-0 of the CAS-wiring rung: `symbolic … for <var>` (bind-then-solve)

**Status: shipped, deliberately narrow.** §3A named "wire the existing engine, don't rebuild it" as
the CAS-wiring goal — letting a curriculum question like "V = I·R, given V and I, find R" be
answered by *rearranging* a cited equation, not just plugging into one. This section is rung-0 of
that rung: the smallest slice that is genuinely useful and genuinely wires the existing
`cas-solve` crate, deferring everything else to later rungs.

**What rung-0 does NOT attempt, on purpose (research findings, not guesses):**

- **No free (unbound) non-target variables.** The curriculum table's own headline example
  ("rearrange `v = I·R` for *any* variable") implies leaving one input symbolically free — e.g.
  solving for `R` while `I` stays an unbound symbol. `cas-solve`'s linear-system entry point,
  `solve_linear_system(equations: &[IRNode], variables: &[IRNode]) -> Option<Vec<IRNode>>`, cannot
  do this either: every `Symbol` node the equation contains that is NOT in `variables` is treated
  as an unresolvable term by its coefficient extraction (`linear_eval` in
  `cas-solve/src/linear_system.rs`), not as a free parameter to carry through symbolically — the
  caller must have **already** reduced every non-target identifier to a numeric IR leaf before the
  equation reaches the solver. So rung-0 is **bind-then-solve**: every variable except the one
  named after `for` must already be `observe`d (bound to a concrete number) before `?`-querying the
  `symbolic` construct. This is a real, smaller capability than the spec's own headline example —
  documented here as an explicit, intentional gap, not a silent one.
- **No new `DerivationNode`/audit-trail concept.** `ADJ-REASON-MATH.md` §E already reserves
  `StepKind::FromRewrite` as the future home for "this value came from an algebraic rewrite, not a
  plug-in" — that spec, and the `ReasoningTrace`/`adj-verify` tooling it belongs to, is Wave-1
  territory owned by a separate concurrent agent (excluded from this loop's `delivery_scope`).
  Rung-0 deliberately reuses the **exact same** `DerivationNode::Op` shape a `formula` application
  already produces for its result — the audit trail does not yet distinguish "solved" from
  "computed." That is an accepted rung-0 limitation, not a workaround for a bug: inventing a
  second, parallel provenance shape here would be exactly the kind of encroachment on Wave-1's
  reserved design space this loop must not do.

**How the solve is wired.** `symbolic-vm::Backend`/`VM` is the seam every CAS dialect in this
workspace uses to reach `cas-solve` — `macsyma-runtime`'s `MacsymaBackend`/`solve_handler` is the
one existing example. Rung-0 goes through that same seam via a new, minimal
`crate::symbolic_backend::RungZeroBackend`: it registers exactly one handler, for the `Solve` head,
and holds no other state (no bindings, no other handlers, no rewrite rules) — rung-0 needs exactly
one capability, so the backend is scoped to exactly one capability, not built out as a general CAS
surface ahead of a second one (`simplify`, `differentiate`) actually needing rewrite-rule dispatch.
A later rung can grow this backend (or introduce a different one) without disturbing this one.

**What rung-0 DOES do — the actual wiring:**

```adj
formulabook electricity_laws {
    use electricity_vocab

    symbolic resistance_from_ohms_law(voltage, current) { voltage == current * resistance } for resistance
        source "For many conductors of electricity, the electric current which will flow through
                them is directly proportional to the voltage applied to them."
        locator "https://hyperphysics.gsu.edu/hbase/electric/ohmlaw.html"
        trust authoritative
}
```
```adj
import "electricity.adj"
observe voltage(12)
observe current(3)
? resistance_from_ohms_law(voltage, current)   % engine → 4, same Ohm's-law citation, solved for R
```

A new top-level `symbolic <name>(<params>) { <lhs> == <rhs> } for <target>` construct — a sibling
of `formula` inside a `formulabook` (not a new top-level `*book`; it reuses `formula`'s
`use`/import/provenance scaffolding verbatim, minimizing new grammar surface). `<lhs>`/`<rhs>` are
the **existing** `expr` grammar's arithmetic subset (`+ - * /`, refs, literals — no transcendental
calls in rung-0); `<target>` names which formal parameter is the unknown. `==` reuses the same
`relop` token `constrain`/FL-8's `Compare` already lex.

**Lowering, concretely:**

1. Every parameter *other than* `<target>` must resolve to an observed value at apply time — the
   same "bind at apply time" contract a `formula`'s parameters already have. `<target>` itself must
   **not** be independently observed (it's the unknown, not an input); a program that both
   `observe`s it and asks the `symbolic` construct to solve for it is a clean compile error.
2. Substitute every non-target parameter's bound value into `lhs`/`rhs` (identical substitution
   machinery `formula` application already uses), leaving an expression that is linear in exactly
   one remaining name: `<target>`.
3. **IR translation (the actual new code):** walk the substituted `lhs`/`rhs` and translate each to
   a `symbolic_ir::IRNode` (`lower.rs`'s `expr_to_irnode`) — `<target>` becomes `Symbol(<target>)`;
   every OTHER identifier is already gone (step 2 substituted it to a bound value); a target-free
   sub-expression is evaluated to a plain number through the SAME `compute` path a `let`/`formula`
   uses, then narrowed to an `Integer`/`Rational` IR leaf; `Add`/`Sub`/`Mul` nodes translate
   structurally. This is deliberately much smaller than a general expression-to-IR bridge — rung-0's
   grammar only admits `+ - * /`, and `Div` by a target-free divisor is rewritten to `Mul` by its
   reciprocal (`solve_linear_system`'s own linear extraction has no `Div` case — only `Mul` by a
   constant factor — so this rewrite is what makes `x / 5 == y` reach the solver at all, not just
   `5 * x == y`).
4. **The solve (the actual wiring):** build `Equal(lhs_ir, rhs_ir)`, wrap it `Solve(equation,
   Symbol(<target>))`, and evaluate through `symbolic_vm::VM::eval` — which dispatches, via
   `RungZeroBackend`, to a handler that calls `cas_solve::solve_linear_system(&[equation],
   &[target])` (a 1×1 system is the fully-general degenerate case) and returns its `Rule(<target>,
   value)` node. **This is the actual wiring** — the moment where the answer is decided is
   delegated to the existing, tested crate through the same seam every CAS dialect uses, not
   reimplemented and not called as a bare free function.
5. A `Rule(<target>, Integer(n))` or `Rule(<target>, Rational(n, d))` result → the ordinary derived
   value, exact, carrying the `symbolic` construct's own `source`/`locator`/`trust` (grounding the
   **equation as stated** — "Ohm's law is V=IR" — exactly the way a `formula`'s citation grounds its
   definition; the fact that this particular application *solves* it rather than *evaluates* it
   directly is not a separate claim needing separate citation, the same reasoning `percent_of.adj`
   already relies on when it composes primitives under one citation). Anything else — including the
   unevaluated `Solve(...)` fallback the handler returns for a SINGULAR system (no solution, or
   every value satisfies it; `solve_linear_system`'s `None` cannot distinguish the two) — **abstains**
   with a named reason, never a fabricated number.

**Dependency:** `adj-lang`'s `Cargo.toml` gains `symbolic-vm = { path = "../symbolic-vm" }` (the
`Backend`/`Handler`/`VM` seam), `cas-solve = { path = "../cas-solve" }` (`solve_linear_system`/
`SOLVE`), and `symbolic-ir = { path = "../symbolic-ir" }` **directly** (not merely transitively) —
both the equation IR and its solved-back `Rule`/`Integer`/`Rational` result need the type nameable
in `adj-lang` itself; `cas-solve` does not re-export it. These are the first crates `adj-lang`
depends on outside its existing `logic-engine`/parsing-frontend family.

**One gap this rung's implementation surfaced that the design above did not anticipate:**
`adj-lang`'s closed-vocabulary gate (`enforce_vocabulary`'s `check_query`, MYCIN-2026 M1/M2) special-
cased the `formulas` registry so a `? bmi(...)`-shaped formula application isn't rejected as an
undefined hypothesis/relation — but had no equivalent case for the new `symbolics` registry. A
`symbolic` application query *lowered* correctly (the main per-statement pass runs first and
already computed the right answer), but the vocabulary-enforcement pass runs afterward over the
same program and rejected the query text retroactively wherever a program uses `use`/`dictionary`
scoping — exactly the shape every shipped library (`electricity.adj` included) uses, so this only
surfaced against real content, not the initial in-crate unit tests (which don't `use` a
dictionary). Fixed by threading `symbolics` into `enforce_vocabulary` alongside `formulas`.

---

## 4. The curriculum — kindergarten → medical school (a DAG of libraries)

Each library = **grounded dictionary terms** (provenanced where the term itself is a claim) +
its content — a **fact set** (`relate`), a **formula** (`formula`), and/or a **symbolic identity**
(CAS) — + a **decompose-and-bind worked query** + an **end-to-end test**. Higher layers `import`
lower ones (write-once-use-many). The DAG spans **two intertwined tracks — a MATH track and a
SCIENCE/KNOWLEDGE track — that meet in medicine.** Complexity is added *slowly*, one layer per PR.

| Layer | MATH track (formula / symbolic) | SCIENCE & KNOWLEDGE track (facts, relations) |
|------|--------------------------------|----------------------------------------------|
| **K / early** | `count`, `add`, `subtract`, `compare` | `shapes`, `colors`, `bigger_smaller` |
| **Elementary** | `multiply`, `divide`, `fraction`, `ratio`, `percent`, `average` | **`chemistry/water` — `composed_of(water, hydrogen, 2)`, `composed_of(water, oxygen, 1)`**; `elements`, `states_of_matter`, `plant_parts` |
| **Middle / HS** | `rate`, `proportion`, `unit_convert`, `power`, `root`, `area`, `volume`; **`algebra/solve_linear` (SYMBOLIC — rearrange `v = I·R` for any variable)** | `compounds`, `periodic_groups`, `cell_organelles`, `body_systems` |
| **Pre-clinical** | `concentration`, `dosage`, `clearance`, `mean_arterial_pressure`, `bsa`; **`calculus/rate_of_change` (SYMBOLIC)** | `stoichiometry`, `enzyme_kinetics`, `physiology` relations (composes the 127 recall libs) |
| **Clinical (MLE apex)** | `bmi`, `anion_gap`, `corrected_calcium`, `fena`, `cockcroft_gault_crcl`, `egfr`, `winters_formula`, weight-based dosing | the recall domains (MICRO/PHARM/CARDIO/…) — knowledge the same query composes with the formulas |

Fact/relation libraries ride the **existing** `relate` substrate (no new language rung); numeric
formulas need **rung-0** (§3); symbolic entries need the **CAS-wiring rung** (§3A); and
**constraint** problems (systems of equations, word problems, dosing feasibility, optimization) ride
the **existing** `constrain`/`solve`/`check` surface — e.g. a "two trains" or "how much of a 20% and
a 50% solution to mix" word problem is a small provenanced constraint library the model binds and the
VM solves (or returns `INFEASIBLE`). An MLE item that needs a recalled fact **and** an algebraic
rearrangement **and** a numeric plug-in **and** a feasibility check is answered by *composing four
libraries* — the whole point.

The **arithmetic content already exists** in rungs 0–125 (BMI@30, FENa@15, TTKG@13, MCV/MCH/MCHC@19,
De Ritis@20, lipid/iron indices, Starling@31, …); §6 harvests it. The **knowledge** content largely
exists in the 127 recall libraries; foundational-science fact libraries (chemistry, cell biology)
extend that track downward toward the elementary layers.

The **knowledge** standard library already exists too: the **127 recall `.adj` libraries** (MICRO,
PHARM, CARDIO, …). Formula libraries are the **compute** standard library. Together they are the
importable stdlib a model draws on; an MLE item that needs *both* recall and compute is the apex.

---

## 5. Byte-provenance for the libraries (the non-negotiable)

- **Every shipped `formula` is sourced.** `source` is a byte-quotable span from an authoritative
  reference (WHO/NIH/textbook/primary literature); `locator` resolves it; `trust` tiers it. The
  recall-library **adversarial write gate** applies unchanged: a formula enters the stdlib only via
  spider→provenance→gate, never by fiat. Humans *correct*, they don't author.
- **Term definitions that are themselves claims** (a named clinical quantity, a reference range)
  carry provenance too; purely structural vocabulary (`define addend : number`) does not.
- **Answer-time chain.** `? formula(...)` yields, alongside the value, a derivation object citing:
  (a) the formula's `source`/`locator`/`trust`, and (b) for each bound parameter, the observed
  fact's byte-provenance span in the input. An independent checker (`adj-verify`) re-verifies the
  computation and the citations without the model. This is the paper thesis in miniature:
  *hallucination is an accounting failure; every step is auditable.*

---

## 6. Harvest plan — turn the 125 rungs into libraries (no work discarded)

For each existing rung, lift its FAMILY headline formula into the curriculum:

1. Map the rung's four observed quantities → **dictionary terms** (with dimensions + synonyms).
2. Lift the headline formula → a provenanced **`formula`** in the right layer (source it to the
   real definition; e.g. rung-30 BMI → WHO; rung-15 FENa → a nephrology reference).
3. Replace the rung's per-item inline `let` with an **`import` + bind + `? formula(...)`** query;
   the 21 items become **decompose-and-bind** exercises over the library (the model must recall the
   library and bind the numbers, not be handed the expression).
4. Keep the two-arm scoring — but Arm B now measures *recall-the-library + bind*, the real skill.

Shapes that were pure benchmark filler (the denominator-arrangement rungs with no named clinical
formula) collapse into the small set of **primitive** libraries (`ratio`, `sum_over_difference`, …)
they were all instances of — one library, not fifteen rungs.

---

## 7. The evaluation — decompose → bind → import → compute (converges with the pipeline proof)

The instrument stays two-arm (ADJ-LADDER §3) and reuses the existing harnesses (board-eval,
`decompose_query`, the offline board pipeline, the two-arm ladder scorer — mapped separately). The
**task changes**: given a natural-language item, the small local model must

1. **recall** which formula library (and/or recall library) applies,
2. **decompose** the input with **byte-provenance**, binding each parameter to a cited span,
3. **import + compute** (engine, CPU, exact) — **or abstain** when it cannot ground a binding.

Metrics: Arm A (model alone) vs Arm B (model + stdlib); the **B − A divergence** is the headline;
**abstention** is first-class (a grounded "I don't know" beats a confident wrong answer). The
end-to-end claim we are driving toward: *a Gemma-class local model + the ADJ formula & recall stdlib
passes an MLE slice it fails alone, with an audit trail an independent checker re-verifies.*

---

## 8. PR staging (specs → tests → impl → provenance-gate → changelog → review)

- **PR-0 (substrate):** `formulabook`/`formula` grammar + AST + validation (parameter scoping) +
  apply-lowering to the existing `ComputeExpr` evaluator + `import`/`use` wiring + `adj-lang-cli`
  render/eval + the provenance-required linter rule. Ships with a worked `bmi.adj` + an end-to-end
  test (`import` → bind → compute → assert value **and** citation chain). **This is the unblocker.**
- **PR-1…:** the curriculum, **bottom-up**, one small library per PR (K → elementary → …), each with
  terms + provenanced formula(s) + a decompose-bind query + an end-to-end test; higher layers import
  lower ones. Interleave the **harvest** of the matching existing rung.
- **PR-clinical:** the MLE-apex libraries, composing pre-clinical + primitive libs; wire into the
  board-eval harness for the two-arm + abstention run.

Each PR is a durable, composable asset — the anti-treadmill property: value compounds instead of
enumerating.

---

## 9. Verification & invariants

- `cargo test`/`cargo clippy` green per crate touched; `adj-lang` grammar regenerated via the CLI
  bin, never hand-edited.
- **Zero LLM arithmetic:** the model emits only `observe … / ? formula(…)`; every number in the
  answer is computed by the engine or cited from the input.
- **Provenance-complete:** no shipped `formula` without `source`/`locator`/`trust`; the linter
  enforces it; `adj-verify` re-checks the answer-time chain offline.
- **Compositional:** clinical libraries `import` primitive ones; no formula is restated across
  layers (a duplication check in CI).
- **Exact/dimensional:** formulas run the existing exact/dimensional CPU path; unit mismatches are
  errors, not silent coercions.
```

---

## 10. Context-aware synonym resolution (bind time)

A term and its everyday names must be **interchangeable in context**: given the BMI library, "weight",
"body weight", "mass", and the canonical `body_mass` are the *same quantity*, and the engine must
compute the right one whichever the input used. This is not the decomposer's job to normalize away —
it is a **language guarantee**: the dictionary is the single source of truth for what counts as the
same term, with provenance.

**What exists:** `define <term> : <kind> surface "syn₁", "syn₂", …` already parses and stores a
per-term synonym list (`Define.surfaces: Vec<String>`), and those surfaces already **feed the
decomposer** (they tell the model which words map to the term).

**The gap (this rung):** the surfaces are *not yet used by the engine to resolve a bound term*. An
`observe weight(70)` against a dictionary that declares `body_mass … surface "weight"` should bind to
`body_mass` — today only the canonical name binds. This rung wires **bind-time synonym resolution**:

- When an `observe`d slot (or a formula argument) names a **synonym** of a term in an in-scope
  (imported) dictionary, it resolves to that term's canonical id, and the engine computes the right
  quantity. The resolution is **context-scoped** — only dictionaries actually `import`/`use`d are
  consulted, so "weight" means `body_mass` in the BMI context and something else in another.
- **Ambiguity is an explicit error, never a silent pick:** a surface form claimed by two in-scope
  dictionaries (or two terms) raises a resolution error naming both candidates — the model must
  disambiguate (or the libraries must). A synonym that matches nothing is the existing
  unknown-term error.
- **The resolution is itself auditable:** the derivation records *which dictionary and which synonym*
  resolved the binding (surface form → canonical term → the dictionary's provenance), so "why did
  `weight` become `body_mass`?" is answerable from the trail, not folklore.

Synonyms are content on the existing `surface` mechanism (no grammar change); the rung is the
engine-side resolver + the ambiguity gate + the audit hook. It is what makes the "the model only
decomposes and binds" contract robust to the words a real question actually uses.

---

## 11. The full audit trail — multi-step reasoning, end to end

> **Scope note.** This section states the *requirement* — why the audit trail exists and
> what it must be true of. The **normative technical contract** (the `ReasoningTrace`
> object, the closed `StepKind` sum, step ordering/addressing, the `quote`/`source`
> split, the typed `AbstentionReason`, the checkability invariant, and `adj-verify`)
> lives in **`ADJ-REASON-MATH.md` §E**, which is where RS-4 is implemented from. This
> rung — **FL-7 — is the same work as RS-4**; the two labels name one deliverable, and
> §E stages it PR-A…PR-D. Read this section for the *why*, §E for the *what*.

The north-star invariant, stated sharply: **for *any* answer — a single formula, a formula that
composes lower formulas, a recalled fact feeding a computation, a constraint solve, or all of them in
one question — the engine can render the complete chain of how it got there, and an independent
checker can re-verify that chain without the model.** Hallucination is an accounting failure; the
audit trail is the accounting.

**What exists:** the engine already builds a **`DerivationNode`** tree (`compute.rs`) — leaves cite a
valued fact by `fact_id` → its `Provenance` → source bytes; `Op` nodes record each arithmetic step;
and **`DerivedRef`** nodes reference a *previously-bound derived value*, i.e. **cross-step chaining is
already representable**. There is a **`proof_dag.rs`** for the probabilistic/relational (ProbLog/SLD)
side — proofs as paths, each carrying the set of clauses used. Provenance carries human-readable
citation spans.

**The gap (this rung):** these pieces are not yet *composed and rendered* for the formula-library,
multi-step, cross-modality case, and there is **no full-explanation renderer** (no `explain` /
`audit_trail` function today; the CLI shows a single derived value's provenance). This rung delivers:

- **One composed proof object per answer.** A formula application chains into the same derivation/proof
  structure as `let`; a formula that imports and applies a lower formula nests its sub-derivation; a
  recalled `relate` fact used as an input attaches its edge provenance; a `solve`/`check` attaches the
  constraint set and the SAT/optimization witness. The result is a single tree/DAG spanning **all four
  modalities** with a leaf-level byte-provenance for every input and a cited definition for every step.
- **A full-explanation renderer.** Given that object, emit a human-readable, ordered narrative —
  *"BMI = body_mass / height²  [WHO, «…», authoritative]; body_mass ← 70 kg from «…weighs 70 kg…»;
  height ← 1.75 m from «…1.75 m…»; = 22.86 kg/m²"* — and the machine form the checker consumes.
- **Offline re-verification (`adj-verify`).** An independent pass re-executes the arithmetic, re-checks
  each citation resolves, re-runs each constraint/solve, and confirms the rendered answer follows —
  **with the model absent**. A trail that does not re-verify is a failed answer, abstained on.
- **Abstention is part of the trail.** When a step cannot be grounded (an unresolvable binding, an
  `INFEASIBLE` constraint, a missing fact), the trail records *where and why* it stopped; a grounded
  "I don't know, because…" is a first-class, auditable outcome.

This rung is what turns "the engine computed 22.86" into "here is the auditable derivation, and it
re-checks" — the property the whole ladder exists to demonstrate at MLE scale.

**Since this section was first written**, two things changed the picture, both recorded in
`ADJ-REASON-MATH.md` §E.7. First, the substrate grew two more answer-producing constructs that a
user can ask "why?" about — `table` lookup (ADJ-TABLES RS-5) and defeasible precedence (ADJ73) — so
the step sum gained `FromTableRow`, `FromRangeBracket`, and `FromGoverning`. Second, and more
importantly, **RS-5e made a table answer cite the row it actually selected** rather than the table's
one shared envelope. That was a hard prerequisite for this rung: an explanation renderer is only as
honest as the citations it renders, and before RS-5e every band of a table quoted the same sentence.
Building `explain` first would have shipped a trail that *looked* rigorous and misattributed every
tabular fact in it.
