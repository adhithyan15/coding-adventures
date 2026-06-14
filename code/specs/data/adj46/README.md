# ADJ46 — ACS rulebook + Jane Doe case run through the existing `logic-engine`

This directory is the toolchain shakedown for the Adj-Lang work
(ADJ47). It encodes the ADJ36 ACS chest-pain rulebook and the same
patient vignette directly against the production `logic-engine` Rust
crate, and runs the query end-to-end.

The point is not elegance. The point is to document, with running
code as the artifact, every place where the existing engine forces an
awkward encoding. Those awkwardnesses are the design inputs to
Adj-Lang.

## Files

- `src/main.rs` — the encoded rulebook + case, query, LR aggregator,
  and audit-printer.
- `AWKWARDNESS.md` — running log of the 10+ awkwardness items found
  during encoding. **This is the primary deliverable** — Adj-Lang's
  language primitives are reverse-engineered from this list.
- `output.txt` — captured output of `cargo run`.
- `Cargo.toml` — depends only on `logic-core` and `logic-engine`.

## Running

```sh
cd code/specs/data/adj46
cargo run
```

## Result

ADJ36's reference posterior on this patient is `P(acs) = 0.281`. This
encoding produces `0.2806` — a 0.04% absolute delta, within rounding.
The math is right; the *encoding* is what's wrong, and that's the point.

## Awkwardness highlights

The full list is in `AWKWARDNESS.md`. Headline items:

1. **LR magnitudes have no home in the engine** — `Probability::Value`
   only accepts values in [0, 1], but LRs range over (0, ∞). Forced to
   stash log-LRs in a side-table and join against `Proof::via_rules` by
   hand.
2. **Provenance is not a clause field** — every citation lives in
   the same side-table.
3. **Prior is just a bare `const`** — engine's `Probability` is
   world-state probability, not Bayesian prior log-odds.
4. **WMC posterior is discarded** — the engine computes the wrong
   number for LR aggregation. We use the proof DAG and aggregate
   ourselves.
5. **"No clear precipitator" has no encoding** — the patient case's
   uncertainty marker is lossy.
6. **No kickback / counterfactual / source-disagreement primitives** —
   harness has to invent all three.
7. **Surface syntax is hand-written Rust** — domain experts would not
   write this rulebook.

## What this changes

- **ADJ46 ships:** the toolchain shakedown is complete. The Rust
  pipeline (rulebook → KB → search → DAG → aggregate → audit) runs in
  ~10 ms and reproduces ADJ36's posterior.
- **ADJ47 unblocked:** the AWKWARDNESS.md list now has concrete,
  numbered items 1–10 that the Adj-Lang frontend has to address. Each
  item maps to one of the five new components estimated in the
  inventory (probabilistic frontend, provenance compiler, VOI engine,
  counterfactual evaluator, source-disagreement aggregator).
- **The Python `adj36-execute.py` is no longer the canonical executor.**
  This Rust binary supersedes it: same posterior, but routed through
  the production engine, and the awkwardness is exhaustively documented
  rather than papered over.

## See also

- [`code/specs/ADJ36-end-to-end-clinical-demo.md`](../../ADJ36-end-to-end-clinical-demo.md)
  — the source rulebook and case this binary reproduces.
- [`code/specs/LP19e-likelihood-ratio-aggregation.md`](../../LP19e-likelihood-ratio-aggregation.md)
  — the spec for the `SearchMode::LRAggregate` mode that would dissolve
  awkwardness A1, A3, and A6 at the engine layer.
- [`code/specs/ADJ45-three-way-blind-judge-experiment.md`](../../ADJ45-three-way-blind-judge-experiment.md)
  — the previous milestone (blind-judge evidence that the framework's
  resolution loop earns its keep on open-ended factual lookup).
