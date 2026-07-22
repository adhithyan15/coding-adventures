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

### Defeasible precedence (v0.18, ADJ73)

Most real rulebooks are **defaults with exceptions**: two rules derive conclusions that
cannot both hold, and a priority decides which one *governs*. Declare the predicate
**functional** (at most one value per key) and give the rules a `Priority` **tier**; then
`enumerate_governing` resolves the conflict as a post-pass over `enumerate_all`:

```rust
use logic_engine::{enumerate_governing, GovernStatus, KnowledgeBase, Priority, Rule};

let mut kb = KnowledgeBase::new();
kb.declare_functional("timing", 1);                       // one timing decision may hold
kb.add_fact(/* stable_routine_pending */);
kb.add_rule(Rule::certain(/* timing(await) when stable_routine_pending */)
    .with_priority(Priority::Specific));                  // beats the default
kb.add_rule(Rule::certain(/* timing(treat_now) — default */));            // Priority::Default

let res = enumerate_governing(&/* timing($D) */, &kb);
// timing(await) governs; timing(treat_now) is Defeated { by: timing(await) }.
// A tie at the top tier yields ConflictPeer (never silently resolved) — abstain/ask.
```

Priority is a **named enum tier** (`Default < Specific < Authoritative < Mandatory`), not a raw
integer; a ground fact (`Standing::Asserted`) outranks every rule tier. A predicate that is
**not** declared functional never conflicts, so every answer governs and `enumerate_all`
semantics are unchanged — precedence is opt-in per predicate.

**Grounded context precedence (v0.19, lex superior).** A rule can be grounded in a *context*
(`Rule::with_context("ninth_circuit")`), and a partial order over contexts
(`kb.add_context_outranks("ninth_circuit", "district_court")`) makes the rule in the **greater**
context defeat a conflicting one in a lesser context — *before* the tier is consulted (the tier
breaks ties the context order leaves open):

```rust
kb.declare_functional("means", 2);
kb.add_context_outranks("ninth_circuit", "district_court");  // grounded precedence edge
kb.add_rule(Rule::certain(/* means(waters, broad) */).with_context("ninth_circuit"));
kb.add_rule(Rule::certain(/* means(waters, narrow) */).with_context("district_court"));
// → means(waters, broad) governs; the district reading is Defeated { by: … }.
```

`context_outranks` is cycle-safe; a cyclic order (`context_order_has_cycle`) crowns nothing
rather than picking wrong. With no context order declared, resolution is pure-tier (back-compat).

**Grounded precedence edges (v0.20).** `add_context_outranks` edges carry no provenance. The
*grounded* way to declare precedence is a `relate outranks_context(higher, lower)` clause — an
ordinary [`Fact`] carrying `source`/`locator`/`trust`, queryable and one CAS edit from
correctable. Such a fact participates in the context order exactly like an explicit edge, so the
*reason* one context outranks another rides on the edge itself:

```rust
// federal outranks state BECAUSE of the Supremacy Clause — the citation is on the edge.
kb.add_fact(
    Fact::certain(/* outranks_context(federal, state) */)
        .with_provenance(Provenance::new(
            "U.S. Const. art. VI, cl. 2 (Supremacy Clause)", Some("cl. 2".into()),
            TrustTier::Authoritative)),
);
// a federal rule now governs a conflicting state rule even at a lower tier; the edge is auditable.
```

`context_adjacency()` unions the explicit and grounded edges into one directed graph, so the
cycle check (a single Kahn pass) and `context_outranks` (a cycle-safe DFS) span both sources.

**Derived precedence — meta-rules (v0.21).** Most precedence is not a standing hierarchy but is
*derived* from a primitive fact about the two authorities (which came later, which reversed which).
So `context_adjacency` also reads **rule-derived** `outranks_context` edges: when the KB has any
`outranks_context/2` rule, it enumerates every provable edge (subsuming the ground facts). A
grounded meta-rule then turns a primitive into precedence — `outranks_context($H, $L) :-
reverses($H, $L)` (citing the overruling doctrine) makes a `reverses(a, b)` fact an edge `a > b`:

```rust
kb.add_rule(/* outranks_context(H, L) :- reverses(H, L) */);   // the appeal-status canon
kb.add_fact(/* reverses(scotus_2023, ninth_circuit_2019) */);   // a primitive grounded fact
assert!(kb.context_outranks("scotus_2023", "ninth_circuit_2019")); // edge DERIVED, not asserted
```

The recursion bottoms out at the primitive grounded facts, each byte-provenanced — an edge that can
be derived is derived (and cited), not duplicated. The grounded `context-precedence` rulebook + its
meta-rules + worked legal/medical examples live in `code/specs/data/context-precedence/`
(`code/specs/ADJ73-defeasible-rule-precedence.md` §2.3, §7).

## Re-checking a proof (`verify`)

`verify_proof(&Proof, &KnowledgeBase, &dyn SnapshotStore)` re-executes a proof
rather than reading its claims as authority — the checkability invariant of
`ADJ-REASON-MATH.md` §E.5. All seven `DerivationOrigin` variants are re-run: facts
and rules must still unify, a negation's subgoal must still have an **empty** proof
set (a *truncated* search is a failure, not an absence), and every log-odds
contribution must reproduce from its clause.

Each step gets two independent verdicts, because there are two different ways to be
wrong: `LogicStatus` (did the inference go through?) and `QuoteStatus` (do the bytes
say what it claims?). Keeping them apart is what makes the system able to name its
most interesting failure — a *valid derivation from an invented fact*.

Quote checking is **anchored** to the recorded byte offset in the pinned snapshot,
never an unanchored search: on a long document a short phrase occurs somewhere with
near-certainty, so searching would confirm the words exist, not that they support
the clause. `SnapshotStore` is the seam through which a caller supplies snapshot
bytes; a store with nothing yields `Unverified`, never a pass. Nothing in this
module touches the network.

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
