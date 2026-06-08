# ADJ72 artifacts — the inward spider + byte-stability

Spec: [`code/specs/ADJ72-inward-spider-byte-stability.md`](../../ADJ72-inward-spider-byte-stability.md).

All runs were blind closed-book by default (subagents forbidden web/tools except the
open-book grounding layers, which are explicitly marked). Ground truth was withheld from
every phase except the blind judge.

## Layout

```
adj72/
├── 00-preregistration.md        n=4 probe pre-registration (written before any run)
├── 01-raw-outputs.json          n=4 probe raw resamples
├── 02-analyze.py                n=4 stability analysis
├── 03-findings.md               n=4 probe findings (stability = detector, not selector)
├── 50sample/                    50-claim blind run (3 resamples each, strict 3-of-3 bar)
│   ├── claims.json              50 claims + pre-registered category + ground truth
│   ├── blind_batch_{1..4}.json  id+question only (category/gt stripped) — the blind input
│   ├── out/round{1..3}_batch{1..4}.json   12 raw closed-book subagent outputs
│   ├── assemble_and_analyze.py  stability buckets + category cross-tab
│   ├── assembled.json           per-claim assembled resamples
│   └── FINDINGS.md              headline: 0 fabrications in 29 stable claims; 16/17 traps caught
├── hle-twolayer/                Palmyrene RIB 1065 — the STABLE-ERROR HLE case
│   ├── question.txt / GROUND_TRUTH_judge_only.md
│   ├── phase1_ir.json           byte-accounted IR decomposition
│   ├── phase2_rules_round{1..3}.json   closed-book rule derivation (3 resamples)
│   ├── phase2_gate.md           gate result: all glosses stable but WRONG
│   ├── phase2_5_grounding.json  open-book justification: CONTRADICTED
│   ├── phase3_answer.json       composed: "Regina, the freedwoman of Barates, alas"
│   └── RESULTS.md               one layer fails, two layers succeed
├── hle-hummingbird/             Hummingbird sesamoid — the UNSTABLE-GAP + contamination case
│   ├── question.txt
│   ├── grounding.json           open-book: primary source (Zusi & Bentz 1984) → 4
│   └── RESULTS.md               framework beats bare models AND naive RAG (rejects "2")
└── haiku-test/                  Haiku-as-worker test (Agent model=haiku)
    ├── humingbird_spider.json   Haiku spider: right source, PDF-extract ceiling, UNDETERMINED
    ├── palmyrene_spider.json    Haiku spider: CONTRADICTED → correct, cited
    └── RESULTS.md               zero confident-wrong answers; wrong→right where web-groundable
```

## Reproducing the analyses

```bash
# n=4 probe stability
python3 code/specs/data/adj72/02-analyze.py code/specs/data/adj72/01-raw-outputs.json

# 50-sample assembly + stability buckets
python3 code/specs/data/adj72/50sample/assemble_and_analyze.py
```

The subagent runs themselves are LLM calls (recorded as artifacts here, not re-runnable
deterministically). The prompts used are documented inline in the spec and were identical
and blind across resamples.

## One-line result

Byte-stability is a real closed-book confabulation **detector** (0 fabrications in 29
stable claims) but **not** a truth selector; recursive byte-provenance layers — a
justification/entailment check above the gate, plus an open-book spider for flagged atoms —
catch the stable-error and unstable-gap failures a single layer cannot, demonstrated
end-to-end on two real HLE questions and shown working with Haiku as the worker model.
