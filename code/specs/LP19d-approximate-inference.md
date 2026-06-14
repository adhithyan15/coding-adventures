# LP19d — Approximate Inference: Monte Carlo over Possible Worlds

## Overview

[`LP19`](LP19-probabilistic-logic-core.md) defines exact weighted
model counting. [`LP19a`](LP19a-d-dnnf-compilation.md) scales it to
moderately large knowledge bases via d-DNNF compilation.
[`LP19b`](LP19b-rational-arithmetic.md) makes it exact. This
sub-spec defines what to do when even d-DNNF compilation is
intractable: **approximate inference via Monte Carlo sampling
over the possible-world distribution**.

The framework's existing engine modes (`FindFirst`, `EnumerateAll`,
`AutoDetect`) are augmented with a new variant:

```text
SearchMode :=
    FindFirst
  | EnumerateAll
  | AutoDetect
  | MonteCarlo { samples: usize, seed: Option<u64> }    -- new
```

Approximate inference produces a probability estimate plus a
confidence interval (an interval, not a point estimate, is the right
shape for a sampled result).

## When Approximate Inference Is Needed

Exact WMC scales until one of the following becomes true:

- The number of distinct probabilistic clauses exceeds ~200
  (d-DNNF compilation becomes expensive on dense formulas).
- The proof DAG has many overlapping branches whose d-DNNF
  representation is non-compact.
- Inference must run within a strict latency budget (real-time
  triage, interactive decision support) that exact methods cannot
  meet.

Approximate inference trades a confidence interval for the ability to
run on KBs of arbitrary size and to bound runtime tightly.

## Layer Position

```
   LP19  probabilistic logic core            ← exact WMC over proof DAG
        │
        ├── LP19a  d-DNNF compilation        ← scales formula side
        ├── LP19b  rational arithmetic       ← scales number-precision side
        ├── LP19c  conditional probability   ← P(Q | E)
        │
        └── LP19d  approximate inference     ← this spec; bounded runtime
        │
        ▼
   logic-engine v5 with MonteCarlo mode
```

LP19d composes with LP19c: conditional `P(Q | E)` under MonteCarlo is
the ratio of two sampled WMCs, with the confidence interval
propagated.

## Algorithm: Importance Sampling Over Possible Worlds

The naïve approach is to sample each probabilistic fact's truth
according to its Bernoulli parameter, then check whether the query
is provable in the sampled world. Repeat `N` times; the empirical
fraction of provable worlds is the estimate.

```text
estimate(query, kb, N):
    successes = 0
    for _ in 1..=N:
        world = sample_world(kb)             # one Boolean per prob clause
        if is_provable(query, kb, world):
            successes += 1
    return successes / N
```

This is correct but wasteful when the query's probability is very
small (rare-disease scenarios): most samples don't satisfy the
query, and a billion samples may produce only a handful of
"successes" — the estimate has huge relative error.

**Importance sampling** biases the sampler toward worlds in which
the query is more likely to be provable, then reweights:

```text
estimate_is(query, kb, proposal, N):
    weight_sum = 0.0
    weighted_successes = 0.0
    for _ in 1..=N:
        (world, proposal_prob) = sample(proposal)
        true_prob = world_prob(world, kb)
        importance_weight = true_prob / proposal_prob
        weight_sum += importance_weight
        if is_provable(query, kb, world):
            weighted_successes += importance_weight
    return weighted_successes / weight_sum
```

The proposal distribution comes from the proof DAG: facts that
appear in many proofs are sampled toward TRUE more aggressively
than their base rate. This is the strategy used by ProbLog 2's
sampling backend.

## Confidence Intervals

Each call returns a `MonteCarloResult`:

```text
MonteCarloResult := {
    estimate:        f64,
    confidence:      f64,              -- e.g. 0.95
    interval:        (f64, f64),       -- lower, upper bound
    samples:         usize,
    effective_samples: usize,          -- for importance sampling
    seed:            u64,               -- for reproducibility
}
```

The interval uses the **Clopper-Pearson** exact binomial interval for
naïve sampling and a **bootstrap** confidence interval for importance
sampling. Both can be replaced with normal-approximation intervals
when sample count is large (>1000) for performance.

## Reproducibility

Every Monte Carlo run records its seed. Replay (`ADJ08`) re-uses the
seed and reproduces the exact estimate and interval. This is
non-negotiable: an adjudication that depends on a sampled estimate
must be reproducible byte-for-byte across replay.

## Stopping Criteria

Three options:

1. **Fixed N**: run exactly `N` samples. Predictable runtime.
2. **Width-based**: keep sampling until the confidence interval
   width drops below a threshold `ε`. Predictable accuracy; unbounded
   runtime in pathological cases.
3. **Time-budgeted**: run for at most `T` seconds, return whatever
   estimate has been reached. Predictable latency; accuracy varies
   with KB structure.

The configuration knob is `StoppingCriterion`:

```text
StoppingCriterion :=
    FixedSamples(N)
  | ConfidenceWidth { epsilon: f64, max_samples: usize }
  | TimeBudget { duration: Duration, min_samples: usize }
```

Defaults: `FixedSamples(10_000)` for unattended runs;
`TimeBudget { duration: 1s, min_samples: 1000 }` for interactive use.

## API Sketch

```rust
pub struct MonteCarloOptions {
    pub samples: StoppingCriterion,
    pub seed: Option<u64>,         // None → seed-from-entropy
    pub confidence: f64,           // default 0.95
    pub sampling: SamplingStrategy,
}

pub enum SamplingStrategy {
    /// Sample each probabilistic clause according to its Bernoulli
    /// parameter. Simple and unbiased, but slow for rare queries.
    Naive,
    /// Importance sampling using a proposal derived from the proof
    /// DAG's structure. Better when the query is rare.
    Importance,
}

pub fn weighted_model_count_approx(
    dag: &ProofDAG,
    kb: &KnowledgeBase,
    options: &MonteCarloOptions,
) -> MonteCarloResult;
```

The existing `search` function gains a `MonteCarlo { ... }` mode
that routes to this backend.

## Worked Example

A KB with 500 probabilistic facts (well beyond exact-WMC tractability
without d-DNNF; even d-DNNF compilation may take minutes for dense
KBs). The query `P(diagnosis)` is computed with:

```text
let opts = MonteCarloOptions {
    samples: StoppingCriterion::FixedSamples(100_000),
    seed: Some(42),
    confidence: 0.95,
    sampling: SamplingStrategy::Importance,
};
let result = weighted_model_count_approx(&dag, &kb, &opts);
// result.estimate ≈ 0.013
// result.interval ≈ (0.012, 0.014) at 95% confidence
```

Wall clock: ~100ms on a modern CPU for 100k samples. The interval
narrows as the cube root of samples for importance sampling.

## Composition with LP19c

For `P(Q | E)`, the naïve approach is two separate MonteCarlo runs
(numerator and denominator), but their sampling can be **shared**:

```text
estimate_conditional(query, evidence, kb, options):
    seeded_rng = seed(options.seed)
    numerator = 0.0
    denominator = 0.0
    weight_sum = 0.0
    for _ in 1..=N:
        (world, proposal_prob) = sample(proposal_from_evidence(...))
        true_prob = world_prob(world, kb)
        w = true_prob / proposal_prob
        weight_sum += w
        if all_evidence_satisfied(world, evidence):
            denominator += w
            if is_provable(query, kb, world):
                numerator += w
    return numerator / denominator   # plus a confidence interval
                                       # from a paired bootstrap
```

Paired sampling is more efficient than two independent runs because
worlds that satisfy the evidence contribute to both terms.

## Caveats and Limitations

1. **Estimates have error**. The whole point of LP19d is to trade
   exactness for tractability. Audit-sensitive deployments should
   prefer LP19's exact WMC (or LP19a's d-DNNF) when the KB allows;
   LP19d is the fallback when nothing else fits the latency budget.
2. **Importance sampling can degrade**. A poorly chosen proposal
   distribution produces high variance and slow convergence. The
   framework's default proposal is heuristic; advanced users can
   register custom proposals.
3. **Reproducibility requires seed handling discipline**. Replay
   (`ADJ08`) must use the recorded seed. If the seed is `None`, the
   engine logs the seed-from-entropy value at run time so replay
   can re-use it.
4. **Confidence intervals are themselves random**. A 95% interval
   covers the true probability 95% of the time on average; any given
   interval may not.

## Open Questions

1. **Adaptive proposal learning**. After K initial samples, refine
   the proposal distribution based on observed proof-DAG branch
   selection. Out of scope for the first version.
2. **Antithetic / quasi-random sequences**. Halton or Sobol
   sequences may improve convergence over uniform random; tradeoff
   is implementation complexity.
3. **GPU sampling**. The world-sampling step is embarrassingly
   parallel. A future sub-spec (`LP19d-gpu`) could explore.
4. **Streaming results**. A live UI could show the estimate
   converging as samples arrive. API extension; not in scope.

## Status

Draft. The naïve sampler is mechanical; the importance-sampling
proposal is the substantive design choice. First implementation
should ship the naïve sampler as `SamplingStrategy::Naive`, with
the importance sampler following.
