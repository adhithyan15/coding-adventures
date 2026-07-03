# ADJ46 — ACS Rulebook on the Existing `logic-engine`: Toolchain Shakedown and Awkwardness Catalogue

> **Headline.** Encode the ADJ36 ACS chest-pain rulebook + Jane Doe
> patient case directly against the production Rust `logic-engine`
> crate, run the query end-to-end, and reproduce ADJ36's published
> posterior to within 0.04% absolute. The point is not the posterior;
> it is the **catalogued list of 10 awkwardnesses** the encoding had
> to invent, which now form the design inputs to ADJ47 (Adj-Lang).
> The hand-coded Python LR multiplier in `adj36-execute.py` is
> superseded by a Rust binary running on the real engine; the next
> milestone (ADJ47) collapses the 10 awkwardnesses into language
> primitives.
>
> **Code**: [`data/adj46/`](data/adj46/) — Cargo binary, awkwardness
> log, output, README.

## Why this milestone exists

After ADJ45 demonstrated empirically that the framework's recursive
resolution loop earns its keep on open-ended factual lookup
(SimpleQA 51 → 94 correct, 47% → 6% hallucination), the natural
next step was to make the *executor* side of the framework real. The
existing Python `adj36-execute.py` hard-coded LR multiplication: it
worked, but it was a Python script that knew about LRs and ACS, with
no separation between rulebook authoring and engine execution. The
audit trail was a `print` statement, not the proof DAG.

ADJ46 takes the same rulebook and case, but routes them through the
production `logic-engine` crate. That forces a concrete encoding
question for every part of the rulebook that doesn't naturally fit
the engine's `Fact / Rule / Probability` shape — which turns out to
be most of it. The mismatches are the catalogue.

## What the encoding had to do

The `logic-engine` crate, per its `lib.rs` documentation, is a
weighted-model-counting probabilistic logic engine. Clauses carry
`Probability` annotations in [0, 1]; the engine computes P(query) by
summing the probability mass of satisfying worlds, walking a proof
DAG along the way. The ACS rulebook, by contrast, is in LP19e's
LR-aggregation form: each contribution is a likelihood ratio (range
(0, ∞)) on log-odds, the prior is a Bayesian prior probability,
contributions sum in log-odds space rather than multiply in
probability space.

The encoding strategy:

1. **Each LR contribution** becomes a deterministic `Rule` whose head
   is a synthetic `contrib(<atom>)` marker and whose body is the
   condition under which the contribution fires.
2. **The LR magnitudes** live in a parallel `HashMap<&str, LrEntry>`
   side-table because `Probability::Value` cannot hold values > 1.
3. **The provenance** (citation strings) lives in the same side-table,
   joined onto each `RuleId` at proof time.
4. **The prior** is a bare `const PRIOR_P_ACS: f64`.
5. **The joint contribution** is encoded as a multi-body rule that
   fires when both atomic conditions hold; the engine doesn't know
   it's an interaction term.
6. **The "no clear precipitator" uncertainty marker** is lossy: we
   omit all three competing `precipitator` facts.
7. **The WMC posterior the engine computes is discarded** — it's the
   wrong quantity for LR aggregation. We walk the `ProofDAG`, look up
   the LR for each fired contribution, and aggregate in log-odds
   space ourselves.

That last point matters: the engine *does* give us what we need,
which is the structured `Proof::via_rules: Vec<RuleId>` telling us
which contributions fired. We're not subverting the engine; we're
using its output as a substrate for a different aggregation operator
than the one it ships with.

## Result

```
ADJ36 reference posterior:  P(acs) = 0.2810
This binary's posterior:    P(acs) = 0.2806
Absolute delta:             0.0004 (OK — encoding reproduces ADJ36)
```

The math is right. The encoding cost is the catalogued awkwardness.

## The catalogued awkwardnesses

(Full prose in [`data/adj46/AWKWARDNESS.md`](data/adj46/AWKWARDNESS.md);
summarized here.)

| # | What the rulebook wants | What the engine forces | Adj-Lang primitive |
|---|---|---|---|
| **A1** | LR contributions: `contributes(2.5, X, acs)` | Side-table; `Probability::Value` can't hold LR > 1 | `LikelihoodRatio` type, `contributes <lr> from <ev> to <target>` |
| **A2** | Citations on every clause | Side-table joined via `RuleId` after proof | `provenance` as a first-class clause field |
| **A3** | Bayesian prior `prior(0.10, acs)` | Bare `const`; engine's `Probability` ≠ prior log-odds | `prior <p> for <atom>` clause kind |
| **A4** | `contributes_jointly(1.3, [a, b], target)` | Multi-body rule indistinguishable from independent AND | `interacts <lr> when [<evs>] for <target>` |
| **A5** | "No clear precipitator" | Omit facts (lossy) or fabricate priors | `uncertain <atom> over [<domain>] prior <dist>` |
| **A6** | LR aggregation | WMC posterior discarded; aggregate by hand | `SearchMode::LRAggregate` per LP19e |
| **A7** | "I'm not confident; here's why" | Compare against threshold in harness | `SearchResult::Kickback { required_resolutions }` |
| **A8** | "What if X were true?" | Clone KB, mutate, re-run | `query counterfactual <atom>=<v> for <target>` |
| **A9** | Source X says LR=2.5, source Y says 2.0 | Pick one, comment | `contributes <lr> from <ev> to <target> via <source>` (multi-source aggregation) |
| **A10** | Domain-expert-readable rulebook | Hand-written Rust | The Adj-Lang surface syntax itself |

Of these 10, items A1, A2, A3, A6 are *engine-layer* — they want new
data types and a new search mode in `logic-engine`. Items A4, A5, A7,
A8, A9, A10 are *language-layer* — they want syntax + a compiler that
lowers to (extended) engine primitives. The inventory in ADJ45's
follow-up estimate (9–13 person-weeks) covers both.

## What this changes in the codebase

- **`adj36-execute.py` is superseded.** Same posterior, but routed
  through the production engine. Future ACS work should run against
  this binary, not the Python script.
- **`logic-engine` gets its first non-test consumer outside its own
  crate.** The encoding pattern (synthetic `contrib(<id>)` head + side
  tables) becomes the documented baseline for "doing LR aggregation
  before LP19e ships."
- **AWKWARDNESS.md becomes the design spec for Adj-Lang.** The 10
  items, each with "what the rulebook wants / what the engine forces /
  what primitive is needed," are the falsifiable design inputs.
  Nothing in Adj-Lang should be invented without a corresponding
  catalogued awkwardness; nothing in the catalogue should be left
  unaddressed.

## What ADJ47 inherits

The infrastructure inventory (lexer / parser / logic-engine /
constraint-vm / compiler-ir / WMC) plus this catalogue lets ADJ47
proceed without further speculation. The five new components
estimated at 9–13 pw map to the awkwardness items as follows:

| Adj-Lang component | Addresses awkwardnesses |
|---|---|
| Probabilistic syntax frontend (lexer grammar + parser grammar + AST) | A1, A3, A4, A5, A9, A10 |
| Provenance term compiler | A2 |
| VOI engine (per ADJ18) | A5, A7 |
| Counterfactual query evaluator | A8 |
| Source-disagreement aggregator | A9 |
| **+ engine extension: `SearchMode::LRAggregate`** | A1, A3, A6 |

The engine extension is properly the LP19e implementation, which
lives in the `logic-engine` crate rather than in Adj-Lang. It belongs
in ADJ47 as a co-deliverable because the Adj-Lang compiler will lower
LR-bearing programs to that search mode.

## What's deliberately not done in ADJ46

- **No language design.** The surface syntax of Adj-Lang is for
  ADJ47. The point of ADJ46 is to suffer enough in the existing
  engine that the language requirements are evidence-driven.
- **No LP19e implementation.** The catalogued A1/A3/A6 want this, but
  implementing it during the shakedown would have hidden the
  awkwardness rather than documenting it.
- **No VOI / counterfactual / kickback in the binary.** Same reason
  — surface them as awkwardnesses, defer to ADJ47.
- **No second domain (legal, M&A).** ADJ48 is medical (full
  MYCIN-2026), ADJ49 is M&A. ADJ46 stays narrow on ACS to keep the
  catalogue focused.

## Cost summary

| Metric | Value |
|---|---|
| LOC of Rust | ~300 (src/main.rs) |
| Build time | ~1.1s |
| Run time | <50ms |
| Posterior delta vs ADJ36 | 0.0004 |
| Awkwardnesses catalogued | 10 |
| New engine features required | 1 (`SearchMode::LRAggregate`) |
| New language components required | 5 |
| Estimated cost to dissolve all 10 awkwardnesses (ADJ47) | 9–13 person-weeks |

## See also

- [ADJ45](ADJ45-three-way-blind-judge-experiment.md) — the empirical
  evidence (SimpleQA 94/100) that motivated continuing to invest in
  the executor side rather than ablating it.
- [ADJ36](ADJ36-end-to-end-clinical-demo.md) — the original ACS
  rulebook and case this binary reproduces.
- [LP19e](LP19e-likelihood-ratio-aggregation.md) — the spec for the
  `SearchMode::LRAggregate` that would dissolve A1/A3/A6 at the
  engine layer.
- [ADJ18](ADJ18-active-sensing-voi-on-proof-dag.md) — the spec for
  VOI computation that addresses A5/A7.
- [ADJ14](ADJ14-probabilistic-ir-semantics.md) — the source-of-truth
  for what "probabilistic IR" means in the framework.

## Status

- 2026-06-02: ADJ46 binary written and run. Posterior matches ADJ36.
  Awkwardness catalogue at 10 items.
- Next: ADJ47 — design and build Adj-Lang. Frontend + provenance
  compiler + VOI engine + counterfactual evaluator + source-disagreement
  aggregator + LP19e engine extension. Estimated 9–13 pw.
