# logic-engine (Rust)

Probability-aware facts, rules, and deterministic find-first search.

## What This Is

`logic-engine` is the first slice of [`LP19`](../../../specs/LP19-probabilistic-logic-core.md):
the layer above `logic-core` that adds clauses (Facts and Rules), the
knowledge base, and a search engine. Everything in this crate is
*probability-aware from the start* — probability is a uniform property of
clauses, with `Certain` (semantic 1.0) as the default so deterministic
Prolog falls out as a special case.

This first slice limits itself to **deterministic find-first search**. The
proof-DAG return type, full search modes, and the weighted-model-counting
backend land in subsequent PRs. The data shapes are in place from day
one so that adding them is purely additive.

### Differential decisions (v0.7)

`lr_aggregate(query, kb)` scores **one** hypothesis. `differential(hypotheses,
kb)` scores a set of **competing** hypotheses, ranks them by posterior, picks
the argmax, and reports the between-hypothesis margin — the operation MYCIN
performs. The decision is `Determinate` when the leader beats the runner-up
even under the worst-case resolution of every open uncertainty, and `Kickback`
(with ranked markers to resolve) when an unresolved finding could flip the
ranking. Deterministic, CPU-only, and each ranked hypothesis keeps its proof
DAG.

## How It Fits in the Stack

```
   LP00 Logic Core            ← logic-core: terms, variables, unification
        │
        ▼
   LP19 Probabilistic Logic Core   ← this crate (first slice)
        │
        └── Prolog frontends (PR00..PR90) — surface syntax over deterministic clauses
```

## API at a Glance

```rust
use logic_core::{atom, compound, var, Term};
use logic_engine::{Fact, Rule, BodyLiteral, KnowledgeBase, Probability, find_first};

let mut kb = KnowledgeBase::new();

// father(homer, bart).
kb.add_fact(Fact::certain(
    compound("father", vec![atom("homer"), atom("bart")]),
));

// father(homer, lisa).
kb.add_fact(Fact::certain(
    compound("father", vec![atom("homer"), atom("lisa")]),
));

// Query: father(homer, X).
let x = var("X");
let query = compound("father", vec![atom("homer"), Term::Var(x.clone())]);

let answer = find_first(&query, &kb).expect("there is at least one answer");
assert_eq!(answer.walk_var(&x), atom("bart"));  // first matching clause
```

### Defeasible precedence (v0.17, ADJ73)

Most real rulebooks are **defaults with exceptions**: two rules derive conclusions that
cannot both hold, and a priority decides which one *governs*. Declare the predicate
**functional** (at most one value per key) and give the rules a `priority`; then
`enumerate_governing` resolves the conflict as a post-pass over `enumerate_all`:

```rust
use logic_engine::{enumerate_governing, GovernStatus, KnowledgeBase, Rule};

let mut kb = KnowledgeBase::new();
kb.declare_functional("timing", 1);                       // one timing decision may hold
kb.add_fact(/* stable_routine_pending */);
kb.add_rule(Rule::certain(/* timing(await) when stable_routine_pending */).with_priority(10));
kb.add_rule(Rule::certain(/* timing(treat_now) — default */));            // priority 0

let res = enumerate_governing(&/* timing($D) */, &kb);
// timing(await) governs; timing(treat_now) is Defeated { by: timing(await) }.
// A tie at the top priority yields ConflictPeer (never silently resolved) — abstain/ask.
```

A predicate that is **not** declared functional never conflicts, so every answer governs and
`enumerate_all` semantics are unchanged — precedence is opt-in per predicate. The general
partial-order form (`context_order`, for jurisdiction/specialist precedence) and the adj-lang
surface syntax are staged in `code/specs/ADJ73-defeasible-rule-precedence.md`.

## Why Probability From Day One

Real ProbLog engines (KU Leuven's ProbLog 2) are built on top of
deterministic Prolog engines (YAP). The deterministic engine is the
substrate; probability is an annotation that triggers proof enumeration.

This crate inverts the convention slightly: probability is a property of
every clause from the start, with `Certain` as the default. Programs that
never use `Probability::Value(p)` are pure Prolog. The Boolean check
`KnowledgeBase::is_all_certain()` is the runtime gate that, in subsequent
PRs, will enable the find-first short-circuit so deterministic programs
never pay the proof-enumeration tax.

This design choice is documented in [`LP19`](../../../specs/LP19-probabilistic-logic-core.md).
The Rust API mirrors that spec exactly.

## Status

Experimental. Implements the deterministic subset of LP19.
