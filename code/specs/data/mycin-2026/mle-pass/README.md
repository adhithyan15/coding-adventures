# mle-pass — multi-hop recall harness

Answers **two-hop** board questions purely on the CPU from grounded edges, with **zero model
calls**. A clinical clue is chained `clue → disease → gene` by joining two grounded `relate`
edges in an adj-lang **rule body** (the engine's SLD resolver does the join on the shared
disease), and the answer carries **both hops' byte-provenance**.

Spec: [`code/specs/MLE-PASS-multihop-recall.md`](../../../MLE-PASS-multihop-recall.md).
The single-hop predecessors live in [`../recall/`](../recall); this is the reasoning step
past them, toward board coverage.

```
clue ──eye_finding_indicates──▶ disease ──gene_defect──▶ gene
        (ophtho-edges.adj)                 (genetics-edges.adj)
   leukocoria ───────────────▶ retinoblastoma ─────────▶ RB1
```

## Files

| file | what |
|------|------|
| `items.json` | the MCQ bank — clue, hop-1 library+relation, 5 gene options, gold |
| `mle_pass_eval.py` | builds each two-hop query, runs `adj-lang-cli`, scores correct/abstained/wrong + `multihop_coverage` |
| `test_mle_pass.py` | gates: gold bound for every item, both hops cited, unknown clue abstains |
| `gen_items.py` | regenerates `items.json` from the verified chain table |
| `../recall/multihop-recall.query.adj` | the shipped worked artifact (runs in place) |

## Run

```
# from this directory (needs adj-lang-cli built: cargo build -p adj-lang-cli)
python3 mle_pass_eval.py
python3 -m pytest test_mle_pass.py -q
```

## How the join works

adj-lang has no top-level conjunctive query, but a rule body chains subgoals sharing a
variable, so a two-hop question is one rule + one binding query:

```adj
import "ophtho-edges.adj"
import "genetics-edges.adj"
rule {
    head: clue_to_answer($X, $A)
    when: eye_finding_indicates($X, $D), gene_defect($D, $A)   % join on $D
}
? clue_to_answer(leukocoria, $A)   % → rb1, with both hops cited
```

The engine returns the binding **and** the citing clause of each hop. Nothing is
authored here: every edge reuses an already-grounded, spider+adversarially-gated fact
(`../recall/`). Because adj-lang imports may not escape their directory, the harness assembles
each run in a temp dir (the needed `*-edges.adj` copied beside the query, de-duplicated),
leaving the shipped `recall/` library untouched.

### Hop 1 can run in reverse

Some chains run "backwards". The grounded edge is `causes(organism, disease)`, so to answer
"disease X — what is its causative organism's Gram stain?" the clue is the *second* argument
and the organism is the middle entity. Setting `"hop1_reverse": true` emits the hop-1 subgoal
as `rel1($D, $X)` (join var first), and the engine's SLD resolver joins on `$D` either way —
relations are bidirectional, so a two-hop chain need not run left-to-right:

```adj
import "micro-edges.adj"          % both hops live here → imported once
rule {
    head: clue_to_answer($X, $A)
    when: causes($D, $X), gram_stain($D, $A)   % $X = disease (clue), $D = organism (join)
}
? clue_to_answer(cholera, $A)     % → gram_negative (vibrio_cholerae), both hops cited
```

Slice 3: **30/30 correct, zero model calls** — slice-2's 13 gene/inheritance chains + 14
microbiology organism-ID chains (8 `disease → organism → Gram stain`, 6 `→ morphology`, run in
reverse — the original MYCIN domain) + 3 **abstention** items (ungrounded clue/disease ⇒ must
abstain). `multihop_coverage` 1.0 over the 27 answerable items; the scorer reports
`abstained_correctly` and counts any binding on an abstention item as a fabrication (wrong).
The answer variable is `$A` (gene / inheritance pattern / Gram stain / morphology, per hop 2).

### Three hops

`build_query` threads an **arbitrary chain** `$X → $D → $E → … → $A` (each hop forward or
reverse), so a three-relation question is one rule with three joined subgoals:

```adj
import "gi-edges.adj"
import "micro-edges.adj"
rule {
    head: clue_to_answer($X, $A)
    when: biopsy_finding_in($X, $D), causes($E, $D), gram_stain($E, $A)   % join on $D and $E
}
? clue_to_answer(pseudomembranes, $A)   % → gram_positive (C. difficile via pseudomembranous_colitis), 3 hops cited
```

Slice 4: **34/34 correct, zero model calls** — slice-3 plus three more Gram-stain chains and the
first genuine **three-relation** chain (`pseudomembranes → pseudomembranous_colitis → C. difficile →
gram_positive`), joined on two shared interior entities, the answer citing all three hops. The bank
ships one 3-hop today (only that disease bridges a finding library to a micro `causes` disease), but
the harness supports any N-hop — more land for free as cross-library id-joins are grounded.

Slice 5: **38/38 correct, zero model calls** — slice-4 plus four more clue→disease→{gene,
inheritance} chains landed PURELY by cross-library id-joins that are already grounded:
`cafe_au_lait_macules → neurofibromatosis_type_1 → nf1 / autosomal_dominant` (hop 1 = `derm-edges`)
and `heinz_bodies → g6pd_deficiency → g6pd / x_linked` (hop 1 = `histo-edges`). Nothing was grounded,
renamed, or authored — exactly the "land for free" the spec anticipated. (`heinz_bodies` also maps to
`methemoglobinemia`, but only `g6pd_deficiency` carries the gene/inheritance edge, so the join binds a
single answer — the second hop disambiguates.) `multihop_coverage` 1.0 over the 35 answerable items.
