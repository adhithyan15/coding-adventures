# LP19 — Probabilistic Logic Core: Probability as a Uniform Algebra

## Overview

This spec extends the Logic VM described by `LP00..LP18` with **probabilistic
facts and rules** as a first-class feature of the engine — not as a
separate layer bolted on top.

The central claim is one sentence:

> Probability is a uniform algebra over the engine's terms; deterministic
> Prolog is the degenerate case where every probability is exactly 1.0.

A single engine handles both modes. Programs with no probabilistic facts
run in find-first-proof mode, exactly like classical Prolog, at no extra
cost. Programs with at least one probabilistic fact run in proof-enumeration
mode and a weighted-model-counting backend computes the query's probability.

This design choice has three consequences:

1. **One implementation, one set of optimizations.** No bifurcation between
   "the Prolog engine" and "the ProbLog engine"; the maintenance burden
   stays linear, not multiplicative.
2. **The deterministic and probabilistic worlds share a single audit
   trail format.** Proof DAGs are produced for both. Provenance tooling
   composes uniformly across the two regimes.
3. **Higher-level frameworks (in particular `ADJ`) consume one engine.**
   The Adjudication framework's rule subtypes (`definitional`,
   `constraint`, `default`, `probabilistic`) all lower to the same backend.

This is also how real ProbLog implementations are built. ProbLog 2 from
KU Leuven sits on top of YAP Prolog; the deterministic engine is the
substrate, and probability lives as an annotation that triggers proof
enumeration. LP19 names that design choice explicitly and puts it at the
foundation.

## Why a Uniform Algebra

The user-facing reframing — *"ProbLog is like optional typing; Prolog is
the statically known case"* — captures the right flavor. The cleaner
mathematical framing is:

| Algebra | Degenerate case |
|---|---|
| Integers | Rationals with denominator 1 |
| Reals | Complex numbers with imaginary part 0 |
| Probabilistic logic | Boolean logic with probability ∈ {0, 1} |

Boolean logic is what classical Prolog operates over: a fact is either
provable (1) or not (0). Probabilistic logic generalizes this to
probabilities in [0, 1] and replaces "find a proof" with "compute the
probability that a proof exists, marginalized over the truth values of
probabilistic facts."

The two regimes share the same algebraic structure. Conjunction
(`fact1, fact2`) is product of probabilities (under independence);
disjunction over proofs is inclusion-exclusion. The engine's job, in both
regimes, is to enumerate proofs and combine probabilities. In the all-1.0
case the combination collapses to Boolean conjunction-and-disjunction —
which is exactly what classical Prolog backtracking computes, one proof
at a time, before stopping.

## Layer Position

```
   LP00 Logic Core            ← terms, variables, substitutions, unification
        │
        ▼
   LP01..LP18                  ← engine, instructions, bytecode, builtins
        │
        ▼
   LP19 Probabilistic Logic Core   ← this spec
        │
        ├── Prolog frontends (PR00..PR90) — surface syntax for the all-deterministic case
        └── ProbLog frontend (forthcoming) — surface syntax for probability annotations
              │
              ▼
   ADJ11 — connector spec that emits LP19-shaped probabilistic rules from
           the Adjudication rule pipeline
```

LP19 does *not* deprecate any of LP00..LP18. It extends them with a
probability dimension that defaults to 1.0 (and short-circuits to the
existing engine paths when that default holds throughout).

## The Probability Type

```text
Probability :=
    Certain                       -- semantic 1.0, short-circuit optimizable
  | Value(real in [0, 1])

Two probabilities are equal under the engine's well-formedness check iff:
    Certain == Certain                                       (true)
    Certain == Value(p)     iff p == 1.0                     (within ε)
    Value(p) == Value(q)    iff p == q                        (within ε)
```

`Certain` is **not** a syntactic sugar for `Value(1.0)`. It is a distinct
variant that the engine recognizes structurally. The reason is purely
operational: programs that use `Certain` everywhere can be detected at
compile time, and the engine emits the deterministic search path without
ever materialising a proof DAG.

The `ε` tolerance for floating-point equality is engine-configurable.
The recommended default is `1e-12`. Configurations are versioned with
each adjudication's audit trail.

## Facts and Rules

The data shapes that the engine reasons over:

```text
Fact := {
    term:        Term,         -- a logic-core term (LP00)
    probability: Probability,  -- defaults to Certain
    id:          FactId,       -- stable, monotonic; used as a Boolean
                                   variable in the formula compiler
}

Rule := {
    head:        Term,
    body:        [BodyLiteral],
    probability: Probability,  -- defaults to Certain
    id:          RuleId,
}

BodyLiteral :=
    Pos(Term)                  -- positive goal: this term must hold
  | Neg(Term)                  -- negation-as-failure: this term must NOT hold
```

Two things to note:

1. **Probability rides on the *clause*, not on the term.** A
   `probabilistic(head, body)` rule with probability `p` means: "given
   that the body succeeds, the head holds with probability `p`." Terms
   themselves remain probability-free; this keeps the LP00 layer
   unchanged.
2. **Every fact and rule has a stable id.** The id functions as the
   Boolean variable name for that clause in the propositional formula the
   engine compiles for weighted model counting. Ids are required: an
   anonymous probabilistic fact cannot be distinguished from another
   independent occurrence of the same term.

For deterministic programs, the user-facing API never exposes the
`probability` field. It defaults to `Certain` and is invisible. The
existing Prolog frontends (`PR00..PR90`) can continue to emit facts and
rules without modification.

## Proof DAGs

The engine's return type is no longer "a stream of bindings" but **a proof
DAG**: a directed acyclic graph capturing every successful derivation of
the query.

```text
ProofDAG := {
    root_query: Term,
    nodes: [
        {
            id:        ProofNodeId,
            term:      Term,                 -- the goal proved by this node
            origin:    DerivationOrigin,
            children:  [ProofNodeId],        -- subgoals of this proof step
            via_facts: [FactId],             -- probabilistic facts used here
            via_rules: [RuleId],             -- probabilistic rules used here
        }
    ],
    success_paths: [[ProofNodeId]],          -- one path per complete proof
}

DerivationOrigin :=
    FromFact(FactId)
  | FromRule(RuleId)
  | Unification                  -- subgoal succeeded via a unification step
```

Notes:

- For deterministic queries, the proof DAG has the structure of a tree
  whose first successful leaf is the answer; the engine may terminate as
  soon as it finds that leaf if it has detected the all-`Certain` regime.
- For probabilistic queries, the engine traverses every branch. Each
  success path contributes a term to the Boolean formula the
  weighted-model-counting backend will count over.
- `via_facts` and `via_rules` lists are the propositional variables that
  must be true for that proof step. They are the bridge between the DAG
  and the formula compiler.

## Search Semantics

```text
search(query, kb, mode) -> ProofDAG

mode :=
    FindFirst              -- terminate at the first success path
  | EnumerateAll            -- traverse every branch, return the complete DAG
  | AutoDetect              -- inspect kb, choose FindFirst iff every Fact
                              and every Rule has probability = Certain
```

`AutoDetect` is the default and is the path Prolog programs follow without
the user knowing the engine is "probabilistic underneath." Programs that
introduce a single `Value(p)` fact get `EnumerateAll` semantics
automatically.

A subtle but important guarantee: `FindFirst` and `EnumerateAll` produce
*the same* first proof when the knowledge base is all-`Certain`. The
modes differ only in continuation behaviour. This means that the engine
in `AutoDetect` mode is a strict superset of the classical Prolog engine
in observable behaviour for deterministic programs.

## Weighted Model Counting

Given a proof DAG over a knowledge base that contains probabilistic facts
and/or rules, the probability of the query is:

```text
P(query) = WMC( φ )

where:
    φ                = disjunction over success paths,
                       each path = conjunction of fact/rule indicators
                       that must be true for that proof to succeed
    indicator(F)     = Boolean variable, true with probability F.probability
    indicator(R)     = Boolean variable, true with probability R.probability
    indicators are mutually independent unless declared otherwise
    WMC              = weighted model count of φ
```

For small knowledge bases (≤ 20 probabilistic clauses, conservatively),
`WMC(φ)` is computed by direct enumeration over `2^n` possible worlds.
The engine emits the formula `φ` as a propositional structure and counts
straightforwardly. This is naïve but correct, and is the implementation
in the first Rust PR landing alongside this spec.

For larger knowledge bases, `WMC` is computed by compiling `φ` to a
**d-DNNF** (decomposable deterministic negation normal form) or **SDD**
(sentential decision diagram) and evaluating the weighted count in time
linear in the diagram's size. d-DNNF compilation is `LP19a`, a planned
follow-up sub-spec.

## The Shared-Fact Trap

When two proofs of a query share a probabilistic fact, naïve probability
arithmetic gives the wrong answer.

Concrete example:

```text
0.5 :: edge(a, c).
0.9 :: edge(a, b).
0.8 :: edge(b, c).
0.7 :: edge(a, d).
0.6 :: edge(d, c).

path(X, Y) :- edge(X, Y).
path(X, Y) :- edge(X, Z), path(Z, Y).
```

`path(a, c)` has three proofs:

1. `edge(a, c)`
2. `edge(a, b), edge(b, c)`
3. `edge(a, d), edge(d, c)`

If `edge(a, b)` also appeared in proof 3, the proofs would *share* a fact
and would no longer be independent events. Multiplying their individual
probabilities and adding by inclusion-exclusion would over- or
under-count, depending on the structure.

The weighted model count handles this correctly *because it counts
worlds, not paths*. Each possible world is a Boolean assignment to the
probabilistic facts; the probability mass of worlds in which `query` is
provable is the answer. Shared facts get the same truth value in every
world that contains them, so no double-counting can occur.

This is the foundational reason d-DNNF or SDD compilation is necessary in
practice: it lets WMC scale to formulas with shared variables without
exponential blow-up. The naïve enumeration in the first implementation
will visibly show this scaling concern, motivating the upgrade.

## Negation in the Probabilistic Setting

Negation-as-failure (`Neg(Term)` in the body literal) interacts with
probability in non-obvious ways.

The semantics LP19 adopts is the **distribution semantics** of Sato (1995),
specialized to a stratified well-founded reading of negation. Concretely:

- `Neg(t)` in a rule body is treated as the proposition `¬provable(t)`.
- For each probabilistic world, `provable(t)` has a Boolean truth value
  determined by the deterministic SLD resolution restricted to that world.
- The WMC backend sums probability mass over worlds where the overall
  formula (with negations) evaluates true.

Programs whose negation introduces cycles or non-stratified dependencies
are rejected at compile time. A static check on the dependency graph
flags such programs; this check is part of `LP19a`.

## The All-Certain Short-Circuit

The engine guarantees the following:

**Theorem (informal):** If every Fact and every Rule in the knowledge
base has `probability = Certain`, then for any query `q`, the engine's
output is structurally identical to the output of the classical
deterministic LP00..LP18 engine on the same knowledge base.

Concretely:

- `AutoDetect` selects `FindFirst`.
- `FindFirst` returns the first proof and stops.
- No proof DAG is materialised beyond the first success path.
- No formula `φ` is constructed.
- No WMC backend is invoked.

This means: programs that don't use probability pay zero extra cost. The
new engine is a strict generalization of the old one along every observable
dimension: API, runtime, memory.

## Worked Example: Probabilistic Graph Reachability

The graph reachability example from the user-facing tutorial:

```text
0.9 :: edge(a, b).
0.8 :: edge(b, c).
0.5 :: edge(a, c).

path(X, Y) :- edge(X, Y).
path(X, Y) :- edge(X, Z), path(Z, Y).

?- path(a, c).
```

The proof DAG for `path(a, c)` has two success paths:

1. `path(a, c)` via the first `path/2` clause, using `edge(a, c)`.
2. `path(a, c)` via the second `path/2` clause, with `Z = b`, using
   `edge(a, b)` and `edge(b, c)` (and the first clause for `path(b, c)`).

The Boolean formula:

```text
φ = e_ac ∨ (e_ab ∧ e_bc)
```

Naïve enumeration over the three probabilistic facts produces eight
worlds; the probability mass of the worlds where `φ` is true is:

```text
P(path(a, c)) = 1 − (1 − 0.5)(1 − 0.9 · 0.8)
             = 1 − 0.5 · 0.28
             = 0.86
```

The engine returns `0.86` as the answer along with the proof DAG.

For the same knowledge base where every probability is replaced by `Certain`,
the engine's `AutoDetect` mode picks `FindFirst`, the search returns
proof 1 (the direct edge), and the engine answers `Certain` after
visiting four nodes. No DAG is materialised, no WMC is invoked.

## Connection to ADJ

The ADJ framework's Rule subtypes (see `ADJ01`):

| ADJ Rule subtype | LP19 representation |
|---|---|
| `definitional(head, body)` | `Rule { head, body, probability: Certain }` |
| `constraint(body)` | Lowered to definitional with synthetic head `_constraint(i)` |
| `default(head, body, exceptions)` | Lowered to a priority-ordered set of rules; exceptions emit `Neg` body literals |
| `probabilistic(p, head, body)` | `Rule { head, body, probability: Value(p) }` |

ADJ11 (the connector spec) is reduced to:

1. The lowering rules in the table above.
2. The JSON encoding the ADJ pipeline produces for the LP19 engine.
3. The interface for surfacing the engine's proof DAG back through the
   audit trail.

That is all. The substantive probabilistic-inference engine lives here in
LP19, not in ADJ.

## Open Questions

1. **Probability arithmetic precision.** Floats vs. arbitrary-precision
   rationals. The engine's correctness should not depend on float
   precision; `LP19b` will specify the rational fallback for cases where
   exact probability is required (drug-interaction probabilities are
   often published as exact fractions, for example).
2. **Continuous distributions.** LP19 covers discrete probabilities only.
   Continuous variables (e.g., "patient's age is normally distributed
   given...") require Gaussian-process-style extensions and are out of
   scope here.
3. **Conditional probabilities and the `evidence/2` predicate.** ProbLog
   supports asserting evidence and querying conditional probability
   `P(query | evidence)`. This is mechanical given the WMC backend (it's
   a ratio of two WMC calls), but its surface syntax and engine API
   warrant explicit specification — `LP19c`.
4. **Streaming and incremental updates.** When a new fact arrives, the
   proof DAG and formula may be incrementally updatable rather than
   re-computed from scratch. Out of scope here.
5. **Approximate inference.** For knowledge bases too large for exact
   WMC, Monte Carlo sampling over worlds is correct but slow. Variational
   methods are an option. `LP19d`.

## Limitations

1. **WMC is #P-hard in general.** The engine cannot guarantee fast
   inference on arbitrary knowledge bases. The d-DNNF compilation in
   `LP19a` mitigates this for many real-world structures but cannot beat
   complexity-theoretic lower bounds.
2. **The independence assumption is a modeling choice, not an engine
   feature.** Two probabilistic facts in the knowledge base are
   independent unless a higher-level model (e.g., a Bayesian network
   structure encoded as additional rules) says otherwise. Modeling
   dependence is the user's responsibility.
3. **Probability calibration is not the engine's concern.** Whether the
   probabilities in the knowledge base are well-calibrated to reality is
   a question for whoever authored the facts. The engine computes faithfully
   from its inputs; garbage in, garbage out.

## Status

Draft. Sufficient to implement the naïve enumeration backend immediately.
Sub-specs `LP19a` (d-DNNF compilation), `LP19b` (rational arithmetic),
`LP19c` (conditional probability / evidence), `LP19d` (approximate
inference) will follow as their respective implementation work begins.
