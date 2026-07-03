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
| `iem-edges.adj` | the inborn-errors-of-metabolism knowledge-graph — 12 diseases × {deficient enzyme, accumulated substrate, inheritance} (36 edges). **GENERATED** by `iem_edge_ground.py` (do not hand-edit). Each ungrounded edge is `trust consensus` + `% [FLAG: …]` (authored-debt, visible); a spider-grounded edge lifts to `trust authoritative` with its byte-quote + URL. |
| `iem_edge_ground.py` | the REL-4 **write gate** — consumes `iem-edge-grounding.json` (the spider's output) and regenerates `iem-edges.adj` + `iem-edge-manifest.json`. `--check` verifies the committed file is up to date. Reuses the shared `organism_id_ground` gate helpers. |
| `ground-iem-edges.workflow.js` | the REL-4 **spider** (opt-in, costs tokens + network): one agent per edge grounds it against a primary source (OMIM / biochemistry reference) with a verbatim byte-quote; an independent agent re-fetches + tries to refute. Produces `iem-edge-grounding.json`. |
| `test_iem_edge_ground.py` | gate tests — ungrounded→consensus+FLAG, grounded→authoritative+quote+locator, refuted stays flagged, untrusted-quote escaping, committed-file `--check`. |
| `recall.py` | executable prototype: ground-edge store + single-hop binding-query resolver + proof DAG + **honest abstention** (UNKNOWN when no grounded edge supports an answer). |
| `test_recall.py` | proves the forward + reverse vignettes resolve with citations, reverse lookup is free, and the store abstains on an ungrounded disease. |

## Additional recall domains

The IEM graph is the template; each later domain is a **drop-in** following the exact
same shape — a `<domain>-edges.adj` (gate-generated), a `<domain>_edge_ground.py` gate
that REUSES `iem_edge_ground._edge_block`, a `ground-<domain>-edges.workflow.js` spider,
and a `test_<domain>_edge_ground.py`. Adding a domain to the board is then a one-line
`EDGE_FILES` append in `../board/board_eval.py` plus its items in `../board/items.json`.

| domain | file | relations |
|---|---|---|
| vitamins | `vitamin-edges.adj` | `deficiency_causes`, `classic_finding` |
| anemia | `anemia-edges.adj` | `has_mcv`, `classic_finding` |
| endocrine | `endocrine-edges.adj` | `secreted_by`, `deficiency_syndrome` |
| **coagulation** (REL-13) | `coag-edges.adj` | `factor_deficiency`, `coag_inheritance`, `prolonged_test` |

The coagulation graph covers 5 board-classic bleeding disorders (hemophilia A/B/C, von
Willebrand disease, factor VII deficiency), querying each three ways — the deficient
clotting factor, the inheritance pattern, and the screening test it prolongs (intrinsic
factors VIII/IX/XI → aPTT; extrinsic VII → PT; vWD → bleeding time). It enters as
`trust consensus` authored-debt; `ground-coag-edges.workflow.js` retires it to
`authoritative` with byte-quotes, exactly like the other domains.

## Run

```sh
python3 recall.py        # the two worked vignettes from the spec
python3 test_recall.py   # 7 tests, all deterministic
python3 test_coag_edge_ground.py   # the coagulation gate + engine-resolution tests
```

## Status & what's next

REL-1 proves the **semantics** in Python. **REL-2 + REL-3 make it native:** the adj-lang
grammar now has the `relate` clause, `$`-variable binding queries, and `entity`/`relation`
dictionary kinds; the `adj-lang-cli` resolves a binding query to a `"recall"` JSON section
(bindings + the citing edge's provenance, or honest abstention). Run the native end-to-end with:

```sh
# from code/packages/rust:  the IEM graph + binding queries, answered with citations
target/debug/adj-lang-cli ../../specs/data/mycin-2026/recall/iem-recall-case.adj
```

`iem-recall-case.adj` imports `iem-edges.adj` and asks `? deficient_in(tay_sachs, $Enzyme)` —
the engine binds `hexosaminidase_a` with its citation, 0 answer-time model calls.

**REL-4 builds the grounding harness** (the gate + spider + tests above). To retire the
authored-debt, trigger the spider and regenerate:

```sh
# opt-in: spawns web agents (OMIM/biochem), costs tokens — run ground-iem-edges.workflow.js,
# then feed its iem-edge-grounding.json output through the gate:
python3 iem_edge_ground.py            # regenerate iem-edges.adj with grounded trust + citations
python3 ../grounding/ground_sources.py  # rebuild the system provenance ledger
```

Until the spider runs, every edge is `trust consensus` + `[FLAG: pending]` — visible authored-debt.

Staged next: **REL-5** (board-eval harness scoring recall + differential with an abstention
metric). Every edge enters the CAS only through the grounding pipeline — nothing is
human-authored in the end state.
