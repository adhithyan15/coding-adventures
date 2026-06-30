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
    head: clue_to_gene($X, $G)
    when: eye_finding_indicates($X, $D), gene_defect($D, $G)   % join on $D
}
? clue_to_gene(leukocoria, $G)   % → rb1, with both hops cited
```

The engine returns the gene binding **and** the citing clause of each hop. Nothing is
authored here: every edge reuses an already-grounded, spider+adversarially-gated fact
(`../recall/`). Because adj-lang imports may not escape their directory, the harness assembles
each run in a temp dir (the two needed `*-edges.adj` copied beside the query), leaving the
shipped `recall/` library untouched.

Slice 2: **15/15 correct, zero model calls** — 10 clue→disease→gene chains + 3 clue→disease→**inheritance**
chains (proving the harness is generic over the second hop) + 2 **abstention** items (ungrounded clue ⇒
must abstain). `multihop_coverage` 1.0 over the 13 answerable items; the scorer reports
`abstained_correctly` and counts any binding on an abstention item as a fabrication (wrong).
