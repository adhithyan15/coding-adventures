# Changelog — mle-pass

## [0.6.0] — 2026-06-30

### Added — slice 6: a third hop-2 relation per disease, and three-hop abstention

- Bank grows **38 → 40**, advancing the multi-hop spine in two distinct ways, both purely by
  arranging already-grounded, spider+adversarially-gated facts (no edge grounded, renamed, or
  authored — the no-human-authored rule):
  - **`mh-39` — write-once-use-many at the hop-2 level.** `g6pd_deficiency` already answers
    `gene_defect` (`mh-36`) and `inheritance` (`mh-38`); it now also answers a hematology
    **`classic_finding`**: `heinz_bodies → g6pd_deficiency → bite_cells`, with hop 1 =
    `histo-edges.adj seen_in` and hop 2 = a **third hop-2 library** (`anemia-edges.adj`). One
    grounded disease id now feeds three distinct second relations across three libraries from two
    different hop-1 findings — the strongest single-disease demonstration that the second hop is
    fully generic over the relation/library.
  - **`mh-40` — three-hop abstention.** A 3-hop question whose *ends* are grounded
    (`leukocoria → retinoblastoma` is a real ophtho edge; `gram_stain` is a real relation) but whose
    **interior join is ungrounded**: retinoblastoma is a tumour with no grounded causative organism,
    so `causes(organism, retinoblastoma)` binds nothing and the chain cannot complete. The engine
    MUST abstain rather than fabricate a Gram stain — extending the never-fabricate discipline to the
    deepest chain (a partially-grounded multi-hop is still an abstention, not a guess).
- Join-finder note: a systematic scan confirms `g6pd_deficiency` is today the **only** disease whose
  id appears both as a finding-library object and as a knowledge-library subject, and that
  `pseudomembranous_colitis` (slice 4, `mh-34`) remains the only finding→disease→organism bridge — so
  the answerable cross-library surface is currently saturated; more lands for free as new ids are
  grounded (spec §7).
- Eval: **40/40 correct, 0 wrong, 4 correct abstentions, multihop_coverage 1.0** — every answerable
  item cites all its hops' byte-provenance (`mh-39` cites 2, the 3-hop `mh-34` cites 3), zero model
  calls.

## [0.5.0] — 2026-06-30

### Added — slice 5: two more clue→disease→{gene, inheritance} chains via cross-library id-joins

- Bank grows **34 → 38**: two new diseases now chain end-to-end purely because a **finding-library
  disease id already matches a `genetics-edges` disease id** — no edge was grounded, renamed, or
  authored for this PR; the generator only arranges already-grounded, spider+adversarially-gated
  facts (per the no-human-authored rule).
  - **`cafe_au_lait_macules → neurofibromatosis_type_1 → nf1`** (gene, `mh-35`) and **→
    `autosomal_dominant`** (inheritance, `mh-37`): hop 1 is `derm-edges.adj skin_finding_in`, joined
    on the shared disease to `genetics-edges.adj`.
  - **`heinz_bodies → g6pd_deficiency → g6pd`** (gene, `mh-36`) and **→ `x_linked`** (inheritance,
    `mh-38`): hop 1 is `histo-edges.adj seen_in`. Note `heinz_bodies` also has a grounded edge to
    `methemoglobinemia`, but only `g6pd_deficiency` has a `gene_defect`/`inheritance` edge, so the
    conjunctive join disambiguates to a single answer — a demonstration that a multi-disease hop-1
    finding is resolved by the second hop, not by guessing.
- These reach **two new genes** (`nf1`, `g6pd`, added to `GENE_POOL`) and the **`x_linked`**
  inheritance pattern (added to `INH_POOL`), and exercise **two new hop-1 libraries for the gene/
  inheritance chains** (derm, histopathology) — widening the proof that the harness is generic over
  the source library.
- Eval: **38/38 correct, 0 wrong, 3 correct abstentions, multihop_coverage 1.0** — every answerable
  item cites BOTH hops' byte-provenance, zero model calls.

## [0.4.0] — 2026-06-30

### Added — slice 4: a genuine THREE-relation chain (N-hop harness)

- Bank grows **30 → 34**: three more `disease → organism → Gram stain` chains
  (community-acquired pneumonia, streptococcal pharyngitis, Klebsiella pneumonia), and — the
  headline — the first **three-relation** chain:
  `pseudomembranes →(gi biopsy) pseudomembranous_colitis →(causes⁻¹) C. difficile →(gram_stain) gram_positive`,
  joined on TWO shared interior entities (the disease **and** the organism), every hop
  byte-provenanced (the engine returns **3** citing clauses).
- `build_query` now threads an **arbitrary chain** over `$X → $D → $E → … → $A`, each hop forward
  or reverse, driven by optional `hop3_relation`/`hop3_lib`/`hop3_reverse` fields (and the existing
  per-hop reverse). The 2-hop output is **unchanged** (middle var stays `$D`), so slices 1–3 items
  and tests are unaffected; `run_item` copies the third library too. No new model calls, no new sink.
- The bank ships **one** three-hop chain because today only `pseudomembranous_colitis` bridges a
  finding library to a micro `causes` disease — but the **harness supports any 3-hop**, so more land
  for free as cross-library id-joins are grounded (spec §7 documents the frontier honestly, incl.
  the `antidote_for` agent-vs-poisoning id mismatch that blocks a treatment hop).
- Worked artifact gains a `biopsy_to_gram` three-relation rule (20 in-place queries, all bound; the
  3-hop cites 3 spans). Tests: `test_three_hop_chain_is_a_three_subgoal_join`,
  `test_three_hop_chain_cites_all_three_hops`. Run-verified: **34/34 correct** (31 answerable,
  `multihop_coverage` 1.0; 3 abstained), zero model calls. 8 pytest pass; ruff clean.

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
