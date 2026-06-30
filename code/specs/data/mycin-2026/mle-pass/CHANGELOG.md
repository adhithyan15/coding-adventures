# Changelog — mle-pass

## [0.1.0] — 2026-06-29

### Added — MLE-PASS multi-hop recall harness (first slice)

- New `mle-pass/` harness answering **two-hop** board questions purely on the CPU from
  grounded edges, **zero model calls**: a clinical clue chained `clue → disease → gene` by
  joining two grounded `relate` edges in an adj-lang **rule body** (the engine's SLD resolver
  does the join on the shared disease), with **both hops' byte-provenance** returned.
- `items.json` — 7-item bank over four hop-1 organ-system libraries joined to the genetics
  library (ophtho → leukocoria/KF-rings/lens-dislocation; neuro → caudate; collagen → type-I;
  enzyme-deficiency → α-galactosidase-A/glucocerebrosidase). Nothing authored — every edge
  reuses an already-grounded, spider+adversarially-gated fact in `../recall/`.
- `mle_pass_eval.py` — builds each two-hop query (in a temp dir, since adj-lang imports may
  not escape their directory), runs `adj-lang-cli`, maps the gene binding to the printed
  option, scores **correct / abstained / wrong** + **`multihop_coverage`** (fraction of
  correct answers citing BOTH hops).
- `test_mle_pass.py` — gates: engine binds gold for every item; `multihop_coverage == 1.0`
  (every correct answer cites both hops); an unknown clue **abstains**, never fabricates.
- `../recall/multihop-recall.query.adj` — the shipped worked artifact (runs in place).
- First slice: **7/7 correct, coverage 1.0, zero model calls.** Spec:
  `code/specs/MLE-PASS-multihop-recall.md`.
