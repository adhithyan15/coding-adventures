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
