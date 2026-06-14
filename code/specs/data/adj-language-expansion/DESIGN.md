# ADJ language expansion — typed slots, computation, and provenance-through-math

> The design of record for growing adj-lang into a complete **adjudication computation language**:
> the LLM only **extracts byte-grounded typed values**; the deterministic engine does **all** the
> math — arithmetic, aggregation, date/duration, percentages, threshold comparisons, and equation
> solving — with **provenance flowing through every derived value**. Sequenced as a loop of small PRs.

## 1. Why

Adjudication is full of computation: *sum these line items; compute the CSF:serum ratio; is the claim
within 365 days of purchase; prorate the bonus; solve for the break-even threshold.* If the **model**
does that math, the answer is un-auditable and wrong-by-arithmetic (the HLE failure mode E2 measured).
The fix: the model **decomposes** the messy input into typed facts + recognizes the policy's *formula
and thresholds as structure* — it never computes. The **CPU engine** computes deterministically, and
every derived value carries a **derivation tree** back to the source bytes. A reviewer audits the tree;
the model is never in the arithmetic loop.

This composes with the standing principle ([[feedback_deterministic_is_probabilistic_special_case]]):
**one engine** — deterministic adjudication is the saturating limit of the probabilistic differential.

## 2. The model's job vs. the engine's job

| the model (Haiku) DOES | the model NEVER does |
|---|---|
| extract a value with its **byte span**: `gross_income = quantity(18000, usd)` [span] | add, subtract, multiply, divide |
| recognize the policy's **formula** as structure: `total = sum(line_items)` | evaluate `sum(...)`, compute a ratio |
| recognize a **threshold/operator**: `>= 14600` from the policy bytes | decide whether 18000 ≥ 14600 |
| mark **discards** + **inference justifications** | solve an equation; do date math |

Everything in the right column is the engine's, on CPU, deterministic, provenanced.

## 3. The typed value + computation model (reusing existing Rust)

- **Typed values** = `logic-core` Compound terms / `symbolic-ir` IRNodes:
  `quantity(value, unit)`, `money(amount, currency)`, `date(y,m,d)`, `duration(days,…)`,
  `percentage(p)`, `boolean(true|false)`, `list([...])`. Reuse **symbolic-ir** (`Integer/Rational/
  Float/Symbol/Str/Apply`) as the value IR; `logic-core::Term::Compound` already carries them.
- **Computation** = **symbolic-vm** (`VM { backend }`, tree-walking `eval`, `Backend` trait with
  per-head `handlers`). We add an **adjudication backend** with handlers for `add/sub/mul/div`,
  `sum/count/min/max/avg`, `ratio/percent`, `date_add/days_between/before/after`, and `solve`
  (delegating to **cas-algebraic** for roots). Each handler returns the value **plus a derivation
  node** (op + operands).
- **Derivation tree (provenance-through-math)** — the new piece. A derived value carries
  `Derived { op, operands: [value-or-ref], result, provenance }`; leaf operands cite **byte spans**
  (the extracted facts), internal nodes cite the **op + child provenances**. The engine's proof DAG
  ([`logic-engine/proof_dag.rs`](../../packages/rust/logic-engine/src/proof_dag.rs)) gains a
  `FromComputation` origin so a verdict's audit trail descends into the arithmetic.
- **Audit gates** (port from [`provenance_program.py`](../adj101-defensibility-100crossdomain/provenance_program.py)):
  **faithfulness** (a stated value matches its cited span, modulo unit conversion), **no-magic-numbers**
  (every literal in a formula is either a provenanced fact or a structural constant), **coverage**
  (every quantity-bearing span is used or discarded-with-reason). These run over the typed IR/derivation
  tree, not over emitted Python.

## 4. Surface syntax (adj-lang additions)

```
% typed facts the model extracts (with spans tracked out-of-band)
observe gross_income = quantity(18000, usd)
observe line_item    = quantity(12000, usd)
observe line_item    = quantity(6000, usd)
observe purchase_date = date(2025, 01, 15)
observe claim_date    = date(2026, 02, 01)

% derived values the engine computes (model only writes the FORMULA, never the result)
let total      = sum(line_item)                 % aggregation
let elapsed    = days_between(purchase_date, claim_date)
let csf_ratio  = csf_glucose / serum_glucose    % arithmetic

% rules as saturating contributions over predicates on typed/derived values
contributes 1000000 from total >= 14600   to required_to_file
contributes 1000000 from elapsed <= 365   to within_warranty

? required_to_file
? within_warranty
```

DETERMINATE/INDETERMINATE/CONFLICT still fall out of the `differential` (saturating LR = hard rule;
missing dispositive value → insufficient-evidence/kickback). No new verdict logic.

## 5. The build loop (small PRs, each green + babysat)

1. **Predicate-gated contributions** ✅ **DONE (PR #5340)** — `contributes <LR> from <slot> <op> <value>
   to <verdict>` over a valued fact `observe slot(value)`. Tokens `>= <= == > <`,
   `evidence = predicate | term`, AST `Evidence` enum, adapter, lower, `PredicateContributionClause` in
   logic-engine, CLI. (One coupled PR — see `../adj-deterministic/PLAN.md`; the grammar-regen tool
   already landed.)
2. **Typed value literals** ✅ **DONE (stacked PR)** — `quantity(value, unit)`, `money`, `percentage`,
   `duration`, `count` as first-class observed values. No grammar change was needed: a typed wrapper
   already parses as a nested compound (`gross_income(quantity(18000, usd))`) under step 1's grammar.
   The engine's `numeric_magnitude`/`observed_value` read the **leading numeric argument** as the
   magnitude (uniform rule, no hard-coded functor set), so predicates fire over typed values while the
   unit stays attached to the fact for the faithfulness gate. `date(y,m,d)` magnitude is deferred to the
   date/duration slice (step 5), where it needs day-ordinal semantics rather than a leading scalar.
3. **`let` + arithmetic** — `let name = <expr>` over slots/literals via the symbolic-vm adjudication
   backend; the **derivation tree** + `FromComputation` proof origin; the **faithfulness** +
   **no-magic-numbers** gates.
4. **Aggregations + percentages** — `sum/count/min/max/avg(list)`, `ratio`, `percent`.
5. **Date / duration math** — `days_between`, `date_add`, `before/after`, deadline checks.
6. **Equation solving** — `solve(equation, var)` via cas-algebraic, provenanced.
7. **Audit-gate port** — faithfulness / no-magic-numbers / coverage as engine checks over the IR.
8. **The Haiku run** — Haiku decomposes each run100/run100b case (values + formula structure, NO math)
   → unified `.adj` → `adj-lang-cli` executes on CPU at **0 answer-time calls** → score vs gold across
   20 domains; report the amortization + the derivation-tree audit. The thesis at scale: *dumb model +
   heavy pipeline, CPU-bound, every number provenanced.*

## 6. Reuse map (build on, don't rebuild)

| need | reuse | path |
|---|---|---|
| expression value IR | **symbolic-ir** | `code/packages/rust/symbolic-ir/` |
| tree-walking evaluator + backend trait | **symbolic-vm** | `code/packages/rust/symbolic-vm/` |
| equation solving (roots/factoring) | **cas-algebraic** | `code/packages/rust/cas-algebraic/` |
| typed terms + unification + proof DAG | **logic-core / logic-engine** | `code/packages/rust/{logic-core,logic-engine}/` |
| the differential + kickback + saturating-rule semantics | **logic-engine::differential** | (already built) |
| typed-quantity coverage contract | **ADJ22/ADJ24** | `code/specs/ADJ22-*.md`, `ADJ24-*.md` |
| faithfulness / no-magic-numbers / coverage gates | **provenance_program.py** | `code/specs/data/adj101-.../provenance_program.py` |
| grammar regeneration | **regen_grammars** | `adj-lang/src/bin/regen_grammars.rs` |

## 7. Invariants

- The model emits **no computed numbers** — only extracted values (with spans) + formula structure.
- Every derived value is **reconstructable from the derivation tree** without the model.
- A literal in a formula is provenanced or a declared structural constant (no magic numbers).
- One engine: deterministic = saturating probabilistic; verdict families from the differential.
