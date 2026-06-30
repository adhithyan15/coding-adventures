# Changelog — mle-pass

## [0.2.0] — 2026-06-30

### Added — slice 2: more chains, a second hop-2 relation, and an abstention sub-bank

- Bank grows **7 → 15** items, in three groups:
  - **More clue→disease→gene chains** (hop 2 = `gene_defect`): derm `ash_leaf_spots →
    tuberous_sclerosis → TSC1/TSC2`, and two more enzyme deficiencies (`sphingomyelinase →
    Niemann-Pick → SMPD1`, `acid α-glucosidase → Pompe → GAA`). Now 10 gene chains over 5 hop-1
    libraries (ophtho, neuro, collagen, enzyme-deficiency, derm).
  - **A different second hop — `inheritance`** (clue→disease→inheritance pattern): caudate→Huntington
    →autosomal_dominant; KF-rings→Wilson→autosomal_recessive; lens-dislocation→Marfan→autosomal_dominant.
    Proves the harness and rule-body join are **generic over the second relation**, not gene-specific
    (the query builder already parameterizes `hop2_relation`).
  - **An abstention sub-bank**: items whose clue has **no grounded hop-1 edge**. The engine binds
    nothing, so the only correct, non-fabricating answer is to **abstain** — the scorer counts a
    binding here as *wrong* (a fabrication).
- `mle_pass_eval.score()` now handles `expect_abstain` items (abstain = correct; any binding = wrong)
  and reports `abstained_correctly`; `multihop_coverage` is measured over the correct **answerable**
  items only. The worked artifact `../recall/multihop-recall.query.adj` gains the new gene + inheritance
  rules/queries (13 in-place queries, all bound, both hops cited).
- Tests: `test_bank_exercises_multiple_hop2_relations_and_abstention`; the engine gate now asserts
  every abstention item binds nothing and every answerable correct item cites both hops. Run-verified:
  **15/15 correct** (13 answerable, coverage 1.0; 2 abstained correctly), zero model calls. 5 pytest
  pass; ruff clean.

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
