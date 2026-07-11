# ADJ-RULE-SUBSTRATE — one substrate: a formula IS a rule, and a rulebook IS a state machine

**Status:** Spec-first. Foundational substrate unification underneath
[ADJ-FORMULA-LIBRARIES](ADJ-FORMULA-LIBRARIES.md). Supersedes the "formula as a
separate construct" framing: `formulabook`/`formula` (shipped rung-0) becomes
**surface sugar over the rule substrate**. Retires the compute-vs-resolve split.
**Author:** substrate-unification pass, 2026-07-11.

**Decision (owner):** *full unification — a formula IS a rule*; *explicit state machine
now* for control flow. This spec defines both.

**North star (unchanged):** a small local model decomposes a question with byte-provenance
and binds variables; the **engine reasons over the CPU** — recall, compute, branch, loop —
and renders a complete, re-verifiable audit trail. This spec makes "reason" *one* engine
instead of two, and gives it explicit control flow.

---

## 1. Why unify (grounded in the code)

Today there are **two engines side by side**:

- **The rule resolver** (`logic-engine`) already does **recursive backward-chaining** (SLD)
  over rule clauses — `enumerate.rs`: *"recursively solve the rule's body literals"* — with a
  proof DAG. Rule-calls-rule and recursion already work.
- **The compute evaluator** (`compute.rs`) is a **separate**, depth-guarded arithmetic
  evaluator (`MAX_EVAL_DEPTH = 256`), disjoint from the resolver. A **formula** (rung-0) is a
  *computed value*, not a rule. A formula body cannot call another formula; a rulebook cannot
  branch on a formula application except by inlining a `let`.

Keeping them separate means composition, recursion, and audit have to be re-built twice and
kept coherent across a seam. The fix is the same principle already in the project canon —
**deterministic reasoning is a special case of probabilistic reasoning; one engine** — applied
to compute: **arithmetic is a rule-body primitive, and a formula is a deterministic rule.**

Then the user's three composition requirements **fall out of the resolver that already
recurses**:

| Requirement | Becomes | Already supported by |
|---|---|---|
| formula-calls-formula (recursion) | rule-calls-rule | the recursive SLD resolver |
| rulebook branches off a formula | a rule body references another rule's computed head | rule-body literals |
| multi-step inside a formula | a rule body is a conjunction of sub-goals | Horn-clause bodies |

What is genuinely **new**: (a) the formula→rule desugaring + arithmetic-as-a-rule-body
primitive; (b) a formula application usable as a sub-expression anywhere the compute grammar
appears; and (c) an **explicit state-machine control-flow surface** with exit criteria and a
termination budget.

---

## 2. A formula IS a rule

`formula name(p₁,…,pₙ) = <expr>` **desugars** to a deterministic, single-headed **computed
rule**: head `name(p₁,…,pₙ)` binds the value of `<expr>` on the CPU; the body is arithmetic
that **may itself apply other rules/formulas**. Concretely:

- **Arithmetic is a rule-body primitive.** The `compute.rs` evaluator is invoked *from inside*
  rule resolution to evaluate an arithmetic sub-goal, producing a `DerivationNode` sub-tree —
  not a parallel engine. `MAX_EVAL_DEPTH` becomes the arithmetic sub-budget of the unified
  step budget (§4).
- **Formula application is a first-class expression.** `name(args)` is valid wherever the
  compute grammar appears — inside a formula body (→ formula-calls-formula), inside a `let`,
  and inside `contributes … from <expr> <op> <thr>` / rule predicates (→ a rulebook branches
  on a formula). Resolution finds the callee among imported rules; the nested application is a
  nested sub-derivation.
- **The `formulabook`/`formula` surface is retained as sugar.** Shipped libraries
  (`clinical/bmi.adj`, `arithmetic/arithmetic.adj`) keep their exact surface; they now lower
  to computed rules in the unified substrate. Break-compat freely where the lowering demands
  it (nothing is released).
- **Provenance is unchanged and now composes.** Each applied rule/formula attaches its
  `source`/`locator`/`trust`; a composed answer carries the *nested* chain (§5).

Worked composition (what FL-4/clinical libraries become):

```adj
% ratio composes the primitive quotient; cockcroft_gault composes several
formula ratio(numerator, denominator) = quotient(numerator, denominator)
formula cockcroft_gault(age, weight, creat, sex_factor) =
    quotient( product(difference(constant_140, age), product(weight, sex_factor)),
              product(constant_72, creat) )
```

and a rulebook that **branches on a computed formula**:

```adj
rulebook bmi_classification {
    use bmi_vocab
    % a rule fires on a FORMULA application, not just an inline let:
    contributes 1000000 from bmi(body_mass, height) >= 30 to obese
        source "WHO: a BMI ≥ 30 kg/m² is classified as obesity." locator "https://www.who.int/…" trust authoritative
}
```

---

## 3. A rulebook IS a state machine (explicit control flow)

A `rulebook` gains an explicit **state-machine** surface — the control-flow model for
recursion and looping, with proper exit criteria. This makes a multi-step clinical *process*
(triage → work-up → decision; titrate-until-target; iterate-until-converged) a first-class,
auditable object rather than an implicit resolution order.

```adj
statemachine <name> {
    use <dict>…
    initial <state>

    state <state> {
        % guarded transitions; the first whose guard holds fires.
        transition on <condition-expr> to <state'> do <action>…
        …
    }

    exit when <criterion-expr> yield <result-expr>     % explicit exit criteria
    budget <n> steps                                    % termination guard (required)
}
```

- **States** are named; the machine starts at `initial`.
- **Transitions** are guarded rules: `on <condition>` (a predicate over observed slots,
  computed formulas, or recalled facts) `to <next-state>` `do <actions>` (assert a fact,
  bind a computed value, apply a formula). The guard/condition and actions are the *same*
  expression/rule grammar as everywhere else — a transition can compute a formula, branch on
  a recalled `relate` fact, or solve a constraint.
- **Exit criteria** are explicit: `exit when <criterion>` halts the machine and yields a
  result. A machine with no reachable exit within its budget is a clean `NonTerminating`
  error, never a hang.
- **Recursion / looping** are expressed *through transitions* (a transition back to an
  earlier state loops; a transition to a sub-machine recurses), bounded by the step **budget**.
- **Determinism & audit:** transition selection is deterministic (first-guard-wins, with a
  conflict diagnostic on overlap unless a priority is given — reuse the existing rule
  `priority` tiers); every firing is one entry in the execution trace (§5).

The state machine is **built on the existing resolver + compute**, not a new evaluator: a
guard is a rule/compute evaluation; an action asserts into the `KnowledgeBase`; a step is one
forward-chaining transition. Recursion into a sub-machine reuses the resolver's recursion.

---

## 4. Termination — always halt or error, never hang

One unified **step budget** guarantees totality:

- A global `budget <n> steps` per state-machine run (required in the surface); each transition
  firing and each rule/formula application consumes steps.
- Arithmetic keeps its `MAX_EVAL_DEPTH` sub-guard; rule/rule recursion gets a depth guard;
  the state machine gets a transition-count guard. Exceeding any is a typed error
  (`StepBudgetExceeded` / `RecursionTooDeep` / `NonTerminating`) with the partial trace, so
  the model can **abstain with a reason** rather than the process hanging.
- Cycle detection on identical (state, bindings) pairs short-circuits a livelock into a clean
  error.

This is the engineering price of Turing-adjacent expressiveness done safely: the substrate is
**total by construction** — every run terminates in a value, an abstention, or a typed error.

---

## 5. Provenance & the audit trail ARE the execution trace

The unification makes the FL-7 audit trail natural: **the multi-step audit trail is the
engine's execution trace.**

- Every rule firing, every formula application, every transition, every arithmetic step is a
  node carrying its `source`/`locator`/`trust` (for rules/formulas) and its byte-span (for
  bound inputs). Reuse the existing `DerivationNode` tree, `DerivedRef` cross-step chaining,
  and `proof_dag`.
- A **full-explanation renderer** walks that trace into a human-readable narrative *and* the
  machine form (the FL-7 renderer, now over the unified trace).
- **`adj-verify`** re-executes the whole trace offline — re-runs the state machine, re-resolves
  every rule, re-computes every formula, re-checks every citation — **with the model absent**.
  A trace that does not re-verify is a failed answer, abstained on.
- **Abstention is a first-class trace outcome** (unresolved binding, `INFEASIBLE` constraint,
  `StepBudgetExceeded`, missing fact): a grounded *"stopped here, because …"*.

---

## 6. What is reused vs. new

**Reused (do not rebuild):** the SLD resolver + its recursion (`enumerate.rs`), the compute
evaluator (`compute.rs`, now a rule-body primitive), `MAX_EVAL_DEPTH`, `proof_dag.rs`,
`DerivationNode`/`DerivedRef`, `Provenance`/`TrustTier`, rule `priority` tiers, the
`import`/`use` resolver, the four modalities (relate/ProbLog/CAS-later/constraints).

**New:** formula→rule desugaring + arithmetic-as-rule-body-primitive; formula-application as a
first-class sub-expression (in bodies, `let`, and `contributes … from`); multi-step rule
bodies; the `statemachine` construct + its deterministic forward-chaining driver; the unified
step budget + `NonTerminating`/`StepBudgetExceeded` errors + cycle detection; the
execution-trace renderer + `adj-verify` (shared with FL-7).

---

## 7. Rung staging (each: spec-sync → tests → impl → provenance-gate → security-review → babysit)

Built as small, layered PRs; each keeps the whole test suite green and the shipped
`formulabook`/`formula` surface working.

- **RS-1 — formula IS a rule (the unification core).** Desugar `formula`/`formulabook` to
  computed rules; make arithmetic a rule-body primitive invoked from resolution; make a
  **formula application a first-class sub-expression** (formula-calls-formula + `contributes …
  from <formula-app>` rulebook branching), with the recursion depth guard and nested
  derivation. Subsumes the earlier FS-1. Worked composite (`ratio = quotient(...)`) + a
  branch-on-formula rulebook + e2e.
- **RS-2 — multi-step rule/formula bodies.** `let`-steps inside a body (conjunction of
  sub-goals), each auditable via `DerivedRef`. Subsumes FS-2. Worked `cockcroft_gault` with
  named intermediate steps + e2e.
- **RS-3 — the `statemachine` construct + driver + termination.** States, guarded transitions,
  explicit exit criteria, the step budget, cycle detection, deterministic first-guard-wins with
  priority tiebreak. A worked multi-step clinical process (e.g. a titrate-to-target or a
  triage→work-up→decision machine) + e2e incl. a `NonTerminating`/budget-exceeded test.
- **RS-4 — execution-trace audit renderer + `adj-verify` (with FL-7).** The full-explanation
  renderer over the unified trace + the offline re-verifier + abstention-in-trace.

**Then** the ADJ-FORMULA-LIBRARIES curriculum resumes on this substrate: FL-4 leaf libraries now
*compose* (ratio calls quotient), FL-5 clinical apex composes primitives and *branches* in a
classification state machine, FL-6 synonym resolution, MLE apex.

---

## 8. Verification & invariants

- `cargo test`/`cargo clippy` green per crate touched; grammar regenerated via
  `regen_grammars`, never hand-edited; the shipped BMI/arithmetic libraries keep computing the
  same values with the same citations (a golden pin).
- **One engine:** no answer is produced by a compute path disjoint from rule resolution; a
  formula is a rule.
- **Total by construction:** every state-machine run and every recursion terminates in a value,
  an abstention, or a typed error within its budget — verified by a non-terminating-input test.
- **Provenance-complete & re-verifiable:** every rule/formula shipped is sourced; every answer
  renders a trace that `adj-verify` re-checks offline with the model absent.
- **Zero LLM arithmetic / reasoning-in-the-framework:** the model decomposes and binds; the
  engine recalls, computes, branches, loops, and explains.
