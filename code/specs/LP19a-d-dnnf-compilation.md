# LP19a — d-DNNF Compilation: Linear-time Weighted Model Counting

## Overview

[`LP19`](LP19-probabilistic-logic-core.md) defines the probabilistic
engine and a naïve weighted-model-counting backend that enumerates
`2^n` possible worlds. That backend is correct but exponential in the
number of distinct probabilistic clauses, and becomes impractical
beyond roughly twenty.

This sub-spec defines the **d-DNNF compilation** strategy that
replaces naïve enumeration with a structured representation of the
Boolean formula. Once compiled, weighted model counting runs in time
**linear in the size of the d-DNNF**, not exponential in the number
of variables.

The framework's first-paper evaluation can plausibly get away with
the naïve backend on TSA (≤10 prob clauses) and small medical
examples (≤15). Anything richer — full differential diagnosis, real
license-compatibility graphs, regulatory rule sets — needs d-DNNF.

## Background: What d-DNNF Is

**d-DNNF** stands for *deterministic, decomposable negation normal
form*. A Boolean formula in d-DNNF is a DAG whose internal nodes are
**∧** and **∨** gates, leaves are literals (positive or negated
variables), and the gates satisfy two structural properties:

- **Decomposability** (the `d` in d-DNNF): the inputs to every **∧**
  gate share no variables. *"This conjunction's children are
  variable-disjoint."*
- **Determinism** (the second `d`): the inputs to every **∨** gate are
  mutually inconsistent. *"At most one child is satisfied in any
  model."*

These two properties together let weighted model counting run as a
straightforward upward sweep over the DAG: each gate's count is a
function of its children's counts, with no need for inclusion-
exclusion or world enumeration.

References:

- Darwiche, *On the tractable counting of theory models and its
  application to truth maintenance and belief revision* (2001).
- Darwiche & Marquis, *A knowledge compilation map* (2002).
- For probabilistic logic programs specifically: Fierens et al.,
  *Inference and learning in probabilistic logic programs using
  weighted Boolean formulas* (TPLP 2015).

## Layer Position

```
   LP19 probabilistic logic core            ← proof DAG, naïve WMC
        │
        ▼
   LP19a d-DNNF compilation                 ← this spec
        │
        ▼
   logic-engine v3 with d-DNNF backend
```

LP19a is purely additive on top of LP19. The naïve enumeration
backend remains; d-DNNF is enabled by configuration or by an
automatic heuristic when the number of probabilistic clauses crosses
a threshold.

## Compilation Pipeline

```text
ProofDAG (LP19)
   │
   ▼
Boolean formula  (disjunction of per-proof conjunctions)
   │
   ▼
CNF (Conjunctive Normal Form)
   │
   ▼ Knowledge compiler (Tseitin encoding + Sharp-SAT-style branching)
   │
   ▼
d-DNNF (decomposable, deterministic)
   │
   ▼
Weighted Model Count — linear pass over the DAG
```

### Stage 1: From Proof DAG to Boolean formula

The proof DAG produced by `enumerate_all` already carries each proof's
`via_facts` and `via_rules`. The corresponding Boolean formula is:

```text
φ = ⋁ over proofs ( ⋀ over via_facts ∪ via_rules in that proof: indicator )
```

This is structurally identical to the input to the naïve WMC backend.

### Stage 2: CNF Encoding

The disjunction-of-conjunctions form is **DNF**, not CNF. For most
knowledge compilers we need CNF. The standard **Tseitin transformation**
converts arbitrary Boolean formulas to CNF with linear blow-up at the
cost of auxiliary variables:

```text
For each top-level disjunct d_i (a conjunction):
    introduce auxiliary variable y_i
    add clauses: y_i ↔ (l_1 ∧ l_2 ∧ ... ∧ l_k)
       expanded as:  ¬y_i ∨ l_1, ¬y_i ∨ l_2, ..., ¬y_i ∨ l_k
                     y_i ∨ ¬l_1 ∨ ¬l_2 ∨ ... ∨ ¬l_k

Top-level: y_1 ∨ y_2 ∨ ... ∨ y_m
```

The auxiliary variables `y_i` are *deterministic* given the original
inputs and do not carry probability. They are tracked separately so the
final WMC ignores them.

### Stage 3: Knowledge Compilation

The framework supports two backends:

1. **Internal compiler.** A pure-Rust implementation following the
   Sharp-SAT / c2d-style branching strategy:
   - Pick a branching variable using a heuristic (most-frequently-
     occurring, or VSIDS-style activity scores).
   - Build the d-DNNF as `∨(x ∧ C[x=T], ¬x ∧ C[x=F])`. The two
     branches share no variables among their non-`x` literals when
     `x` is chosen well; the resulting **∨** is automatically
     deterministic (because the literal `x` vs `¬x` in each branch
     forces inconsistency).
   - Recurse on connected components for **∧** gates whenever the
     formula factors into variable-disjoint pieces.
2. **External compiler.** A wrapper around an existing compiler
   (`c2d`, `dsharp`, `d4`) when the deployment has it available. The
   compiler is configured as a versioned component (per the audit-
   trail discipline of LP19); the choice is recorded in every
   adjudication.

The internal compiler is the default for portability; the external
option is an opt-in for performance-critical deployments.

### Stage 4: Weighted Model Counting

Given a d-DNNF, WMC is a single upward pass:

```text
WMC(node) =
    if node is literal x:        weight(x)
    if node is literal ¬x:       1 - weight(x)
    if node is ∧(c_1, ..., c_k): Π WMC(c_i)
    if node is ∨(c_1, ..., c_k): Σ WMC(c_i)

weight(x) = the Bernoulli parameter of x's probabilistic clause,
            or 1.0 for auxiliary Tseitin variables.
```

Time is `O(|d-DNNF|)`. For real-world structures the d-DNNF is often
*much* smaller than `2^n`; medical KBs with hundreds of probabilistic
clauses commonly produce d-DNNFs of size in the thousands.

## The d-DNNF Data Structure

```text
DDnnfNode :=
    Literal { var: VarId, polarity: bool }
  | And { children: [NodeId] }
  | Or { children: [NodeId] }
  | True
  | False

DDnnf := {
    root:  NodeId,
    nodes: Vec<DDnnfNode>,        // arena-allocated for cache locality
    var_count: usize,
}
```

Arena allocation: every node is stored in a flat vector and
referenced by index. This is the standard layout for decision-
diagram-style data structures and is cache-friendly.

## Probability Caching

A common case: the same query is asked twice with different evidence
sets. Recomputing the d-DNNF from scratch each time wastes work.
LP19a defines a **cache key** for d-DNNF reuse:

```text
DDnnfCacheKey := {
    kb_version:        ContentHash,
    query_term_hash:   ContentHash,
    relevant_clauses:  SortedSetOf<ClauseId>,
}
```

Two queries with the same key produce the same d-DNNF; the cache
stores the compiled DAG. Evidence is applied *after* compilation by
fixing the corresponding literals' weights to 0 or 1 (for hard
evidence) before the WMC pass. So the cache is effective even when
evidence changes between queries.

## Numerical Considerations

WMC over a d-DNNF can produce very small probabilities (rare-disease
priors, deep conjunction chains). Float underflow is a real risk for
`n > 100` probabilistic clauses. Two mitigations:

1. **Log-space evaluation.** Instead of products of probabilities,
   sum logarithms; convert back at the end. Numerically stable for the
   probabilistic intermediate values, but **∨** in log-space requires
   the log-sum-exp trick.
2. **Rational arithmetic** (`LP19b`). When exact answers matter,
   evaluate the d-DNNF over `Rational<i128>` or arbitrary-precision
   rationals. Slower but exact.

The engine selects log-space by default for `n > 50`, and exposes a
configuration knob for rational mode.

## Compilation Time vs. Query Time

d-DNNF compilation is expensive — it can dominate inference time for a
single query. The crossover point at which d-DNNF is faster than naïve
enumeration depends on:

- The number of probabilistic clauses (`n`).
- The number of proofs.
- The structure of the proof DAG (shared sub-proofs compress well in
  d-DNNF; truly independent proofs do not).

Rough rule of thumb:

| `n` | Naïve WMC | d-DNNF compile + count |
|---|---|---|
| ≤ 15 | milliseconds | seconds (compile dominates) |
| 15–25 | seconds | sub-second (compile pays for itself once) |
| 25–50 | minutes to intractable | seconds |
| 50+ | intractable | seconds to minutes |

The engine's `SearchMode` gains an `EnumerateAllWithCompiledBackend`
variant (or a more terse name TBD) that triggers d-DNNF; `AutoDetect`
picks d-DNNF when `n` exceeds a configurable threshold.

## Caveats and Open Questions

1. **#P-hardness lower bound.** WMC is #P-hard in general; d-DNNF
   compilation is not always small. For *worst-case* formulas
   compilation produces exponentially large d-DNNFs. In practice the
   structure of probabilistic logic programs (especially those derived
   from Bayesian networks or stratified rule corpora) tends to compile
   well, but no closed-form guarantee exists.
2. **Non-stratified negation.** d-DNNF requires the formula to be in
   the standard well-founded reading. LP19 already requires this for
   correctness of WMC; the same requirement applies here.
3. **Updating the d-DNNF when the KB changes.** A new fact added to
   the KB invalidates the cached d-DNNF for any query whose proofs
   could change. Conservative invalidation (drop the cache) is the
   first implementation; incremental d-DNNF maintenance is research-
   open and out of scope.
4. **External compiler interoperability.** `c2d`, `dsharp`, `d4`
   produce d-DNNF in a DIMACS-like text format. The framework's
   internal compiler will optionally emit this format for
   interoperability with academic tooling.

## Implementation Sketch

The Rust implementation lives in the `logic-engine` crate as a new
`wmc::compiled` submodule. Existing `wmc::weighted_model_count` is
preserved; a new `wmc::compile_and_count(dag, kb)` is added.
Compilation cost is paid the first time a query is run; the compiled
d-DNNF is cached keyed on the structural hash described above.

API sketch:

```rust
pub fn compile_proof_dag(dag: &ProofDAG, kb: &KnowledgeBase)
    -> Result<DDnnf, CompileError>;

pub fn evaluate_d_dnnf(d_dnnf: &DDnnf, weights: &WeightMap) -> f64;

pub fn compile_and_count(dag: &ProofDAG, kb: &KnowledgeBase) -> f64;
```

Tests:

- Small examples: hand-compute d-DNNF and verify structure.
- Round-trip: compile → evaluate against the naïve backend for
  formulas with ≤15 variables; results must match within `1e-9`.
- Stress: synthetic Bayesian-network programs with 50, 100, 200
  variables; verify d-DNNF compilation completes in bounded memory.

## Comparison with Existing Implementations

ProbLog 2 (KU Leuven) uses Sentential Decision Diagrams (SDDs) by
default, calling into an external compiler. The SDD vs d-DNNF tradeoff
is well-studied; SDDs are more constrained (a fixed v-tree of
variables) but compile faster on some structures. The framework
adopts d-DNNF as the simpler-to-implement starting point; an SDD
sub-spec (LP19a-sdd) is a possible follow-up.

## Status

Draft. Implementation depends on the naïve WMC backend already
shipping in `logic-engine` v0.2. Performance comparison against ProbLog
2's SDD backend is a planned evaluation experiment for the paper.
