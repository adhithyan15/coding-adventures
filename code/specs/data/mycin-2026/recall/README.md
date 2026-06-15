# recall/ — relational recall (REL-1)

Fact recall as a **binding query** over a grounded knowledge graph. "Which enzyme is
deficient in Tay-Sachs?" becomes `? deficient_in(tay_sachs, $Enzyme)` and resolves to
`hexosaminidase_a` **with a proof** — the byte-provenanced edge that justifies it and its
citation — on the CPU, with **0 answer-time model calls**.

This is the first slice of the board-exam substrate (see
[`../REL-1-RELATIONAL-RECALL.md`](../REL-1-RELATIONAL-RECALL.md) for the full design and the
thesis that recall is the single-hop, zero-uncertainty special case of the LR differential —
one engine, not two).

## Files

| file | what it is |
|---|---|
| `iem-edges.adj` | the inborn-errors-of-metabolism knowledge-graph **seed** — 6 diseases × {deficient enzyme, accumulated substrate, inheritance}. Surface preview of the native `relate` clause + binding query (lowered in REL-2/REL-3). **Authored-debt**: illustrative, NOT yet spider-grounded — REL-4 re-grounds each edge against a primary source (OMIM / biochemistry reference) through the cold path. |
| `recall.py` | executable prototype: ground-edge store + single-hop binding-query resolver + proof DAG + **honest abstention** (UNKNOWN when no grounded edge supports an answer). |
| `test_recall.py` | proves the forward + reverse vignettes resolve with citations, reverse lookup is free, and the store abstains on an ungrounded disease. |

## Run

```sh
python3 recall.py        # the two worked vignettes from the spec
python3 test_recall.py   # 7 tests, all deterministic
```

## Status & what's next

REL-1 proves the **semantics** in Python before touching the Rust grammar/engine.
Staged next: REL-2 (grammar: `entity`/`relation` dictionary kinds, the `relate` clause, the
`$`-variable token, binding queries) → REL-3 (engine: SLD resolver + native CLI) → REL-4
(spider-ground the IEM edges, retiring the authored-debt) → REL-5 (board-eval harness scoring
recall + differential with an abstention metric). Every edge enters the CAS only through the
grounding pipeline — nothing is human-authored in the end state.
