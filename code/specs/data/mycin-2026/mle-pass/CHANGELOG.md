# Changelog — mle-pass

## [0.3.0] — 2026-06-30

### Added — slice 3: the microbiology organism-ID chain, run in reverse

- Bank grows **15 → 30** items, adding the original **MYCIN** reasoning chain: from a *disease*,
  bind the **causative organism**, then read that organism's **Gram stain** or **microscopic
  morphology** (`disease → organism → trait`). 14 answerable micro chains (8 Gram stain, 6
  morphology) + a micro abstention item whose disease has no grounded causative organism.
- New generic harness capability **`hop1_reverse`**: the grounded edge is `causes(organism,
  disease)`, so the clue (disease) is its *second* argument. When `hop1_reverse` is set, hop 1's
  subgoal is emitted as `rel1($D, $X)` (join var first, clue second) instead of `rel1($X, $D)`;
  the engine's SLD resolver joins on `$D` regardless of argument order — **relations are
  bidirectional**, so a two-hop chain need not run left-to-right through the edges.
- `build_query` now **de-duplicates imports** (micro's `causes` and `gram_stain`/`morphology`
  share `micro-edges.adj`, imported once) and the answer variable is generalised `$G` → `$A`
  (it may be a gene, inheritance pattern, Gram stain, or morphology depending on hop 2);
  `run_item` reads the `A` binding. No new model calls, no new sink.
- Worked artifact gains `disease_to_gram` / `disease_to_morphology` reverse-join rules (now 19
  in-place queries, all bound, both hops cited). Tests: `test_reverse_hop1_and_import_dedup`;
  `test_bank_exercises_multiple_hop2_relations_and_abstention` now also requires `gram_stain`,
  `morphology`, and a reverse-hop1 item. Run-verified: **30/30 correct** (27 answerable, coverage
  1.0; 3 abstained correctly), zero model calls. 6 pytest pass; ruff clean.
- Nothing authored: every edge reuses an already-grounded, spider+adversarially-gated micro fact.

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
