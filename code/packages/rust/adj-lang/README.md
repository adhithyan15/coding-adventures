# adj-lang (Rust)

Surface-syntax frontend for the adjudication framework. Lexes,
parses, and lowers a small domain-expert-readable rulebook DSL into
a `logic-engine` `KnowledgeBase`.

## What this is

Adj-Lang is the language layer of the adjudication framework. Its
v0.1 grammar covers the five clause kinds the LP19e LR-aggregation
engine exposes:

- `prior <p> for <conclusion>` — Bayesian baseline.
- `contributes <lr> from <evidence> to <conclusion>` — atomic LR.
  `<evidence>` is either a term (`pmh(hypertension)`) or a numeric
  **predicate** over a valued slot (`gross_income >= 14600`).
- `interacts <lr> when <e1> and <e2> [and ...] for <conclusion>` —
  joint-evidence interaction term.
- `observe <term>` — assert a Certain Fact. Terms may carry numeric
  arguments (`observe gross_income(18000)`) — the *valued facts* that
  predicates read.
- `? <conclusion>` — query the engine.

### Predicate-gated contributions — deterministic = saturating probabilistic (v0.5)

A **deterministic** rule is just the saturating limit of a probabilistic
one. Write a numeric predicate as the evidence and give it a large LR:

```
prior 0.10 for required_to_file
contributes 1000000 from gross_income >= 14600 to required_to_file
  source "IRS Pub 501 (2024)" trust authoritative
observe gross_income(18000)
? required_to_file
```

The engine evaluates `gross_income >= 14600` on the CPU at decision time —
the model that authored the rulebook never ran the comparison. The proof
step records the literal comparison that fired (`slot`, `op`, `threshold`,
`observed`), so the audit trail shows the numbers, not a model's claim.
DETERMINATE / INDETERMINATE / CONFLICT still fall out of the differential
(leader / insufficient-evidence / kickback) — **one engine, not two**.
Operators: `>= <= > < ==`.

### `let` + arithmetic — computed values (v0.6)

The model writes the **formula**; the engine computes it on the CPU and a
predicate fires over the result like any observed slot:

```
observe csf_glucose(quantity(40, mg_dl))
observe serum_glucose(quantity(100, mg_dl))
observe line_item(12000)
observe line_item(6000)

let csf_ratio = csf_glucose / serum_glucose     % = 0.4
let total     = sum(line_item)                  % = 18000

contributes 1000000 from csf_ratio <= 0.4 to bacterial
contributes 1000000 from total    >= 14600 to required_to_file
```

`<expr>` is `+ - * /` (standard precedence, parentheses), references to
observed slots and earlier `let`s, numeric literals, and aggregations
`sum/count/min/max/avg(slot)`. Every computed value carries a **derivation
tree** back to the cited facts, so a reviewer can audit the arithmetic — the
model never evaluates it. **Space your operators** (`a - 5`, not `a-5`): a `-`
glued to a digit lexes as a negative literal.

### Constraints — `symbol` / `constrain` / `solve` / `check` (v0.7)

The model extracts the policy's **unknowns and constraints**; the engine solves
them (the solver backends land in the next slice). The surface:

```
symbol premium : money(usd)
observe base_rate(1200)
observe cap(2000)

constrain premium >= base_rate
constrain premium <= cap

solve for { premium }          % find a value satisfying the constraints
% or:  check                   % is the constraint set satisfiable?
```

- `symbol <name> : <sort>` declares an unknown (`sort` = `scalar`, `money(usd)`, …).
- `constrain <expr> <relop> <expr>` with `relop ∈ { >= <= > < == = != }`;
  operands are arithmetic exprs over symbols, observed slots, earlier `let`s,
  and numbers. Compare against a typed value by `observe`-ing it and using its
  name (constraint operands are arithmetic exprs, not term literals).
- `solve for { … }` / `check` drive the solver. The lowerer builds a
  `ConstraintSystem` (on `LoweredProgram.constraints`) with each constraint's
  sides kept as unevaluated expression trees.

### Differential over the `?` queries (v0.4)

A program's `? h` lines are read as the set of **competing hypotheses**.
`compile_and_decide(src)` (or `decide(&lowered)`) runs
`logic_engine::differential` over them: ranks by posterior, picks the argmax,
reports the between-hypothesis margin, and kicks back when an open uncertainty
could flip the ranking. A multi-`?` program is therefore a differential
(bacterial vs viral vs fungal); a single `?` yields a determinate result. No
grammar change — the competing set is already the `?` lines.

Every clause can carry annotations:

- `source "<text>"` — citation string.
- `locator "<text>"` — page / section / paragraph within the source.
- `trust <tier>` — one of `consensus | authoritative | empirical |
  inferred | unattributed`.

## The ACS rulebook in Adj-Lang

```adj
% chest-pain ACS risk rulebook (ADJ36)

prior 0.10 for acs
  source "Pope JH et al., NEJM 1995;342(16):1163-70"

contributes 1.5 from pmh(hypertension) to acs
  source "HEART Score; Six AJ et al., Neth Heart J 2008"
  trust empirical

contributes 1.8 from pmh(smoker) to acs
  source "HEART Score; Six AJ et al., Neth Heart J 2008"
  trust empirical

contributes 2.5 from symptom_quality(pressure_like) to acs
  source "Panju AA et al., JAMA 1998;280(14):1256-63"

contributes 2.0 from associated_symptom(diaphoresis) to acs
  source "Panju AA et al., JAMA 1998"

contributes 0.5 from vital_signs(within_normal_limits) to acs
  source "Panju 1998"

contributes 0.4 from denied(ecg_acute_st_changes) to acs
  source "Pope 1995"

interacts 1.3 when symptom_quality(pressure_like)
               and associated_symptom(diaphoresis)
               for acs
  source "[empirical] synergy"
  trust empirical

% The case — Jane Doe vignette from ADJ36
observe pmh(hypertension)
observe pmh(smoker)
observe symptom_quality(pressure_like)
observe associated_symptom(diaphoresis)
observe vital_signs(within_normal_limits)
observe denied(ecg_acute_st_changes)

? acs
```

Compared to the hand-written Rust encoding in ADJ46
(`code/specs/data/adj46/src/main.rs`, ~390 LOC), the Adj-Lang
source above is ~30 lines of readable English. The ACS rulebook is
no longer addressed to the engine; it's addressed to the ED
physician who wrote it.

## How it fits

```
   adj-lang source
        │ [lex] → [parse] → [lower]
        ▼
   logic-engine::KnowledgeBase     ← (this crate's output)
        │ [search, SearchMode::LRAggregate]
        ▼
   posterior + proof DAG + warnings
```

## API at a glance

```rust
use adj_lang::compile;
use logic_engine::{search, SearchMode, SearchResult};

let lowered = compile(source_text)?;
for query in &lowered.queries {
    match search(query, &lowered.kb, SearchMode::LRAggregate) {
        SearchResult::LRAggregateResult { posterior, dag, .. } => {
            println!("P({query:?}) = {posterior:.3}");
            // dag.proofs[0].steps enumerates the prior + every
            // active contribution, with provenance reachable via
            // step.origin's clause_id.
        }
        _ => unreachable!(),
    }
}
```

## What v0.1 covers — and what's deferred

Adj-Lang dissolves ADJ46 awkwardness items **A4** (joint
contributions syntactically distinct from atomic, via the `interacts`
keyword) and **A10** (rulebook surface is hand-written Rust).

Not yet covered (all language-layer follow-ups):

- **A5** — uncertainty markers (`uncertain X over {a,b,c}`).
- **A7** — kickback as a query-result variant.
- **A8** — counterfactual queries (`? acs given pmh(htn)=true`).
- **A9** — source-disagreement aggregation (multiple `via "<source>"`
  clauses per `(conclusion, evidence)`).

These are small additive extensions of the grammar; each adds one or
two arms to `parser::parse_statement` and one new variant to the
lowering pass.

## Tests

24 unit tests across `lexer`, `parser`, and `lower`. Headline test:
`lowers_full_acs_rulebook_and_reproduces_adj36_posterior` — the ACS
program above compiles, runs through `SearchMode::LRAggregate`, and
reproduces ADJ36's 28.1% posterior end-to-end through the production
engine.

## See also

- [ADJ46 — awkwardness catalogue](../../../specs/ADJ46-acs-rulebook-on-logic-engine-toolchain-shakedown.md)
- [LP19e — LR aggregation spec](../../../specs/LP19e-likelihood-ratio-aggregation.md)
- [logic-engine](../logic-engine/README.md) — the inference layer
  Adj-Lang lowers to.
