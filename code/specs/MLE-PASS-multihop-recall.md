# MLE-PASS — multi-hop recall (the two-hop reasoning step)

**Status:** Slices 1–4 shipped. Harness + worked artifact + 34-item bank, run-verified
(gene / inheritance / microbiology-trait hops + abstention; forward and reverse hop 1; a genuine
three-relation chain — the harness now threads an arbitrary N-hop join).
**Author:** autonomous loop, 2026-06-29.

## 1. Why

The single-hop recall rungs answer "which gene is mutated in Huntington disease?" — one
grounded edge, one binding query. Real board questions usually need **two hops**: you are
given a *clinical clue*, not the disease name, and must chain

```
clue ──(hop 1)──▶ disease ──(hop 2)──▶ gene / treatment / mechanism
```

e.g. *"a child with **leukocoria** — the disease it indicates is caused by a mutation in
which gene?"* → leukocoria → **retinoblastoma** → **RB1**. MLE-PASS is the harness that
answers these **purely on the CPU from grounded edges, with zero model calls**, and proves
each answer is a genuine two-hop derivation (both hops cited), not a one-edge coincidence.

This is the reasoning step past the recall rungs toward board coverage
([[project_board_exam_goal]], [[project_hle_north_star]]): the join is **CPU-bound**
([[project_cpu_bound_reasoning_problog]]) — the engine does it, not a model.

## 2. The join is a rule body (in-engine SLD), not Python

adj-lang has no top-level conjunctive query (`? a, b` is a parse error), but a **rule body**
takes comma-separated subgoals sharing variables, and the engine's SLD resolver evaluates
them. So a two-hop question is one rule whose body joins two grounded `relate` edges on the
shared disease, then a binding query on the rule head:

```adj
import "ophtho-edges.adj"        % hop-1 library (clue → disease)
import "genetics-edges.adj"      % hop-2 library (disease → gene)
rule {
    head: clue_to_gene($X, $G)
    when: eye_finding_indicates($X, $D), gene_defect($D, $G)   % join on $D
}
? clue_to_gene(leukocoria, $G)   % engine binds $G = rb1, cites BOTH spans
```

The engine returns the `$G` binding **and the citing clause of each hop** (the
leukocoria→retinoblastoma span *and* the retinoblastoma→RB1 span). The join is done in the
engine; Python only assembles the program and reads the binding.

## 3. Nothing authored — existing grounded edges only

Every chain reuses edges that already shipped through their own
spider→byte-provenance→adversarial-gate PRs ([[feedback_nothing_human_authored]]). The first
slice draws hop-1 from four organ-system libraries that already target genetics-library
diseases by the same id:

| hop-1 library | relation | example clue → disease | hop-2 gene |
|---|---|---|---|
| `ophtho-edges` | `eye_finding_indicates` | leukocoria → retinoblastoma | RB1 |
| `ophtho-edges` | `eye_finding_indicates` | Kayser-Fleischer rings → Wilson | ATP7B |
| `ophtho-edges` | `eye_finding_indicates` | superior lens dislocation → Marfan | FBN1 |
| `neuro-edges` | `lesion_causes` | caudate degeneration → Huntington | HTT |
| `collagen-defect-edges` | `defect_causes` | type I collagen defect → OI | COL1A1 |
| `enzyme-deficiency-edges` | `enzyme_deficiency_disease` | α-galactosidase A → Fabry | GLA |
| `enzyme-deficiency-edges` | `enzyme_deficiency_disease` | glucocerebrosidase → Gaucher | GBA1 |

The bank grows as more cross-library id joins are grounded — no new harness work per chain.

## 4. Harness & metrics

`code/specs/data/mycin-2026/mle-pass/`:
- `items.json` — the MCQ bank (clue, hop-1 library+relation, gene options, gold).
- `mle_pass_eval.py` — builds each two-hop query, runs `adj-lang-cli`, reads the binding,
  maps to the printed gene option, scores **correct / abstained / wrong**. Because imports
  may not escape their directory, each run is assembled in a temp dir (the two `*-edges.adj`
  copied beside the query), leaving `recall/` pristine.
- `test_mle_pass.py` — gates: engine binds gold for every item; **every correct answer cites
  both hops** (`multihop_coverage == 1.0`); an unknown clue **abstains** (never fabricates).
- `recall/multihop-recall.query.adj` — the shipped worked artifact (runs in place).

**`multihop_coverage`** = fraction of correct answers citing BOTH grounded hops — the
defensibility number a future grounding PR moves, and the proof of a real two-hop derivation.
First slice: **7/7 correct, coverage 1.0, zero model calls.**

## 5. Slice 2 (shipped)
The bank grew **7 → 15**: more `gene_defect` chains (derm `ash_leaf_spots → tuberous_sclerosis →
TSC1/TSC2`; Niemann-Pick → SMPD1; Pompe → GAA — 10 gene chains over 5 hop-1 libraries); a **second
hop-2 relation, `inheritance`** (clue → disease → inheritance pattern), proving the harness is generic
over the second relation, not gene-specific; and an **abstention sub-bank** whose clue has no grounded
hop-1 edge — the engine binds nothing and the scorer counts abstaining as correct, a binding as a
fabrication. `score()` reports `abstained_correctly`; `multihop_coverage` is over correct *answerable*
items. Run-verified: 15/15 correct (13 answerable, coverage 1.0; 2 abstained), zero model calls.

## 6. Slice 3 (shipped) — the microbiology organism-ID chain, run in reverse
The bank grew **15 → 30**, adding the original **MYCIN** reasoning chain: from a *disease*,
find the **causative organism**, then read that organism's **Gram stain** or **microscopic
morphology**. The twist is direction. The grounded edge is `causes(organism, disease)`, so the
clue (the disease) is its **second** argument and the organism is the middle entity. The harness
gained a generic **`hop1_reverse`** flag: hop 1's subgoal is emitted as `rel1($D, $X)` (join var
first, clue second) instead of `rel1($X, $D)`, and the engine's SLD resolver joins on `$D`
regardless of argument order — relations are bidirectional. Both hops live in one library
(`micro-edges.adj`), so imports are de-duplicated.

```adj
import "micro-edges.adj"
rule {
    head: clue_to_answer($X, $A)
    when: causes($D, $X), gram_stain($D, $A)   % $X = disease (clue), $D = organism (join)
}
? clue_to_answer(cholera, $A)   % engine binds $A = gram_negative, cites BOTH hops
```

14 answerable micro chains (8 Gram stain, 6 morphology) plus a micro abstention item whose disease
has no grounded causative organism. The answer variable is now `$A` (it is a gene, an inheritance
pattern, a Gram stain, …, depending on hop 2). Run-verified: **30/30 correct** (27 answerable,
`multihop_coverage` 1.0; 3 abstained correctly), zero model calls. The worked artifact gains
`disease_to_gram` / `disease_to_morphology` rules (19 in-place queries, all bound, both hops cited).

## 6b. Slice 4 (shipped) — a genuine THREE-relation chain
The bank grew **30 → 34**: three more `disease → organism → Gram stain` chains, and — the headline —
the first **three-relation** chain, joined on TWO shared interior entities:

```
pseudomembranes ──biopsy_finding_in──▶ pseudomembranous_colitis ──causes⁻¹──▶ C. difficile ──gram_stain──▶ gram_positive
   (gi-edges)                                       (micro-edges, reverse)              (micro-edges)
```

`build_query` now threads an arbitrary chain over `$X → $D → $E → … → $A` (each hop forward or
reverse), so an N-hop question is one rule with N subgoals:

```adj
when: biopsy_finding_in($X, $D), causes($E, $D), gram_stain($E, $A)
```

The middle `causes` hop is reversed (binds the organism `$E` from the disease `$D`); the answer
carries **all three** hops' byte-provenance (the engine returns 3 citing clauses). This is the
deepest CPU-bound derivation in the bank — a model only names `pseudomembranes`; the engine does the
disease→organism→trait reasoning. Today only this disease bridges a finding library to a micro
`causes` disease, so the bank ships one three-hop chain — but the **harness now supports any 3-hop**,
so more land for free as cross-library id-joins are grounded (§7). Run-verified: **34/34 correct**
(31 answerable, `multihop_coverage` 1.0, the 3-hop citing 3 spans; 3 abstained), zero model calls.

## 7. Next
The **three-hop harness now exists** (slice 4); the limit is grounded id-joins, not the engine.
More 3-hops land for free once: (a) more finding→disease edges reach micro `causes` diseases
(today only `pseudomembranous_colitis` does); (b) a relation *chains off* a hop-2 answer — today no
edge starts from a gene, substrate, or trait; (c) an id-consistency pass aligns near-miss ids (the
enzyme-deficiency disease ids `gaucher_disease` do not yet match the lysosomal `accumulates` ids
`gaucher`), best shipped as its own regrounding PR. Also: disease → *treatment* / *mechanism* hops
(blocked by the agent-vs-poisoning-condition id mismatch in `antidote_for`), and a `decision`-style
multi-hop where the engine ranks among several reachable answers.
