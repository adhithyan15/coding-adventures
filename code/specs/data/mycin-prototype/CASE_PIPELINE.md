# Warm-path case pipeline — findings

The warm path runs **decompose → adversarial read → ir_to_adj → decide**, with the model used at
**exactly one point** (decompose) and **zero answer-time model calls** at decision time.

```
vignette (prose)
  → decompose.workflow.js   (LLM, dictionary-constrained, decompose-only) → ir/<id>.json
  → adversarial_read.workflow.js (3 readers: inference read + discard read, majority) → advread/<id>.json
  → ir_to_adj.py            (DETERMINISTIC: gated IR → observe lines, dict-validated)
  → decide.py              (concat CAS rulebook + case → adj-lang-cli → differential + proof DAG)
```

## Result: 4/4 defensible decisions, `answer_time_model_calls = 0`

| case | observed findings | engine decision | gold | ✓ |
|---|---|---|---|---|
| MEN-1 | Gram+ , neutrophilic, low glucose, ↑protein, ↑lactate, seizure, ↑PCT (7) | **determinate → bacterial** (P=1.0) | bacterial | ✓ |
| MEN-2 | neutrophilic, low glucose, ↑protein, ↑lactate, seizure; Gram−, culture pending (5 evid.) | **determinate → bacterial** (P=0.9999) | bacterial | ✓ |
| MEN-3 | lymphocytic, normal glucose, normal lactate, enteroviral PCR+, seizure absent (4) | **determinate → viral** (P=1.0) | viral | ✓ |
| MEN-4 | only culture-pending + seizure-absent (0 evidence) | **insufficient_evidence → abstain** | indeterminate | ✓ |

## What this exercises (the named mechanisms)

- **Decompose-only / model never reasons.** The decomposer mapped prose → canonical dictionary
  findings + byte spans + a discard list + inference justifications, and **did not diagnose**. The
  diagnosis is the CPU engine's. MEN-2's four CSF findings were emitted `inferred` with ENTAILED
  bases; the decomposer also correctly distinguished "procalcitonin not sent" (a legal term, *not
  observed*) from a finding — the closed-vocabulary "absent vs not-yet-observed" distinction.
- **Standard dictionary enforced.** Every emitted term validated against `dictionary.json`
  (`ir_to_adj` raises on a non-dictionary term); IR and rulebook share one vocabulary by construction.
- **Adversarial reading, both links.** 3 model-diverse readers ran the **inference read** (no
  over-reads found — the decompose was faithful) and the **discard read** (no wrongly-dropped
  findings) — the safety net confirming, here, that nothing was dropped or over-asserted.
- **Evidence-sufficiency (abstain, don't fabricate).** MEN-4 has no dispositive finding, so the
  engine would otherwise commit to viral on the **prior alone** (0.963 vs 0.037). A deterministic
  guard on the proof DAG — *the leader fired zero contributions* — overrides this to
  `insufficient_evidence`. This is the honest failure mode surfaced and fixed: don't decide on base
  rates when no evidence was observed.
- **CPU-bound inference.** `decide.py` invokes only the `adj-lang-cli` binary — no agent, no model.
  `answer_time_model_calls_total = 0` across all four cases.

## The over-saturation is live (for the cost-to-correct proof)

MEN-2 returns **P=0.9999** — the correct leader (bacterial) but an indefensible certainty *before*
the Gram stain / culture, because the four correlated CSF findings are multiplied as independent.
`proofs/cost_to_correct` (PR5) localizes this from the proof DAG and fixes it with one explaining-away
edit. The linked programs (`cases/<id>.linked.adj`) and full results (`decide_results.json`) are the
reproducible artifacts.
