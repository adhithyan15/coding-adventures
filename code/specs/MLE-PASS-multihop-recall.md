# MLE-PASS — multi-hop recall (the two-hop reasoning step)

**Status:** First slice shipped. Harness + worked artifact + 7-item bank, run-verified.
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

## 6. Next
More chains as id joins are grounded (histo/cardio → genetics; disease → *treatment* and
disease → *mechanism* hops); a third hop (clue → disease → drug → contraindication); a `decision`-style
multi-hop where the engine ranks among several reachable answers.
