# ADJ57 — byte-provenance pipeline

A generic pipeline that holds **every byte, at every layer**, to one invariant:
represented in the reasoning (with a retrievable span) or discarded with a reason.
Spec: [ADJ57](../../ADJ57-byte-provenance-pipeline.md).

## Layers

| layer | file | what it enforces |
|---|---|---|
| **L0 — CAS** | [`pipeline/cas.py`](pipeline/cas.py) | content-addressed source store; `cite()` rejects any quote not literally present in the source |
| **L1 — coverage** | [`pipeline/coverage.py`](pipeline/coverage.py) | the case→IR partition must reconstruct the input byte-for-byte (total coverage) |
| **L1–L3 — ingest/derive/spider** | [`pipeline/slice.workflow.js`](pipeline/slice.workflow.js), [`full.workflow.js`](pipeline/full.workflow.js) | LLM layers: lossless typed partition, fact-driven link derivation (with total fact coverage), recursive-to-root grounding |
| **driver** | [`pipeline/assemble.py`](pipeline/assemble.py) | runs the coverage check, interns sources into the CAS, byte-provenances each citation, writes the rulebook |
| **ADJ58 — universal stage contract** | [`pipeline/stage.py`](pipeline/stage.py), [`run.py`](pipeline/run.py) | the gate at EVERY arrow: each stage proves 100% input coverage (used-cited + discarded-with-reason), composed into one auditable `Trail`. `run.py` drives a full run through every gate + computes the verdict. Tests: [`test_stage.py`](pipeline/test_stage.py) |

## Run

```bash
# (the slice.workflow.js run produces slice-results.json)
python pipeline/assemble.py pipeline/slice-results.json   # coverage + CAS + rulebook
python pipeline/cas.py ls                                  # inspect the content-addressed store
python pipeline/coverage.py <case.txt> <segments.json>     # standalone coverage check
```

## Artifacts from the proof slice (PMC11521393, pheochromocytoma)

- [`pipeline/slice-results.json`](pipeline/slice-results.json) — the L1/L2/L3 workflow output (after the coverage-correction loop).
- [`cas/`](cas/) — the content-addressed source store: 4 interned sources, 4 byte-anchored citations.
- [`rulebook.json`](rulebook.json) — the byte-addressable rulebook: each LR points at a CAS span and records whether it reached a root source.

## The keystone

The CAS makes the corpus **accrete and deduplicate**: decomposition cost is paid once
per source, ever. A future case in any domain that cites an already-interned source
reuses its byte-provenanced span for free — this is the indexed-source corpus that
makes scale reachable.
