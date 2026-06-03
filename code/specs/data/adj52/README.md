# ADJ52 — autonomous blind cross-arm experiment loop

Implementation artifacts for [`code/specs/ADJ52-autonomous-blind-cross-arm-experiment-loop.md`](../../ADJ52-autonomous-blind-cross-arm-experiment-loop.md).

ADJ52 industrialises the ADJ51 byte-recursive-provenance pipeline and
fuses it with the ADJ45 blind-judge design. This directory is built
incrementally; see the spec's "implementation sequence."

## What's here so far

### The counterfactual / VOI / kickback runner (Phase 1)

`src/main.rs` is the ADJ51 runner plus the part of the thesis ADJ51
specified but never shipped. The engine's `lr_aggregate` already
computes, for every active `uncertain { … }` marker whose domain is
unobserved, what each candidate value would contribute *if observed*
(`UncertaintyReport`). The ADJ51 runner threw that away
(`uncertainties: _`). ADJ52 surfaces it as a per-query panel:

- **Counterfactual sensitivity** — for each candidate value in an
  unresolved uncertainty, the posterior the answer would move to if
  that value were observed, flagged when it flips the decision. ("If
  the biopsy comes back malignant → 99.5%; benign → 7.9%, which flips
  the decision.")
- **Kickback** — when the plausible posterior band (best/worst case
  over resolving every open uncertainty) straddles the decision
  threshold, recommend escalating and list the uncertainties to
  resolve, ranked by value-of-information.
- **Source disagreement** — when two cited sources assign different
  LRs to the same evidence, surface that the posterior is sensitive to
  which authority you trust.

It is a strict superset of the ADJ51 runner: on a rulebook with no
`uncertain` markers it prints the same posteriors + coverage and simply
omits the panel.

## Reproducing

```bash
# Bundled demo fixture (has an open biopsy uncertainty → panel fires):
cargo run --manifest-path code/specs/data/adj52/Cargo.toml

# Any ADJ51-shaped case directory via ADJ52_DIR (path relative to this
# crate). E.g. the real ADJ51 experiment 2 (no markers → graceful, no panel):
ADJ52_DIR=../adj51/experiment2 cargo run --manifest-path code/specs/data/adj52/Cargo.toml
```

The runner reads `<dir>/03-derived-rulebook.adj` +
`<dir>/04-vignette.adj`, compiles via `adj-lang`, and runs each query
through `logic-engine`'s LR aggregator.

## Layout

```
adj52/
├── Cargo.toml                          — standalone runner crate (not a workspace member)
├── README.md                           — this file
├── CHANGELOG.md
├── src/main.rs                         — runner with counterfactual/VOI/kickback panel
└── fixtures/
    └── uncertainty-demo/
        ├── 03-derived-rulebook.adj     — tiny rulebook with one open uncertainty
        └── 04-vignette.adj             — two findings observed; biopsy unobserved
```

## Notes

- Standalone Cargo crate with its own `Cargo.lock`, mirroring ADJ51;
  `target/` is gitignored. Build with `--manifest-path` from the repo
  root, or from within this directory.
- Still to come (per spec): the four-arm orchestrator (generic
  sandboxed ingester, recursive rulebook deriver, plain-Claude control,
  blind judge), the three gates (round-trip, semantic verifier,
  regression suite), and the cron-driven case loop.
