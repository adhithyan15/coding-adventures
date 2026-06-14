# MYCIN-2026 — findings (the five proofs)

MYCIN, rebuilt on the byte-provenance + constraint substrate, run end-to-end on
the bacterial-vs-viral meningitis differential. The rulebook is **derived once**
(the live spider, grounded in primary sources), checked into a **content-addressed
store of importable `adj-lang` libraries**, and reused on many cases at **zero
answer-time model calls** — the model only decomposes prose into typed findings.
Reproduce everything:

```sh
python3 warm/dict_export.py && python3 warm/run_warm.py     # warm path (uses committed IRs)
python3 proofs/golden_and_cpu.py                            # proofs 1 + 5
python3 proofs/cost_to_correct.py                           # proof 2 (headline)
python3 proofs/audit_trail.py case_bacterial_culture        # proofs 3 + 4
python3 warm/test_m7.py                                     # VOI + IIS
```

## The five claims, and what proves each

### 1. Golden rulebook — derive once, reuse at 0 answer-time model calls
`golden_and_cpu.py`: the 4 vignettes are decided **twice** from their committed
decompositions; results are **identical**, **4/4** match gold, and
`answer_time_model_calls == 0` across both runs. The diagnosis is a pure,
reproducible function of *(decomposition, grounded rulebook)* — no model in the
answer loop. The live model ran exactly once per case, in `decompose.py`, and
wrote only typed findings (never a diagnosis).

### 2. Cost-to-correct — one CAS edit fixes a bug and propagates (the headline)
`cost_to_correct.py`. The bacterial arm deliberately encodes the four
CSF-chemistry findings as **independent** contributions. They are not — they are
joint effects of one inflammatory process — so multiplying their LRs
over-saturates: the pre-culture case (all four present, no Gram stain/culture)
is called bacterial at **P = 0.9995**, an indefensible certainty before
microbiology. The proof DAG **localizes** this to the four stacked CSF
contributions. The fix is **one clause** — an explaining-away `interacts` term
whose joint LR pulls the four-way product back to a single combined signal —
applied once to the library. Re-deciding: **P = 0.7752** (calibrated), and it
**propagates to every citing case at 0 model calls**. The three
microbiologically-confirmed cases stay correctly diagnosed.

**Honest, important sub-finding:** after calibration the pre-culture bacterial
posterior (0.775) drops *below* the aseptic base rate (0.963), so the
argmax-posterior "leader" flips to viral. This is correct, not a regression:
pre-culture you genuinely *cannot* claim bacterial is more **probable** than the
base rate — you treat empirically because of asymmetric **costs**, not
probability. Calibration's job is to remove false certainty, and it does. (A
production system would surface the bacterial *lift* and the cost asymmetry, not
just the argmax; noted as future work.)

### 3. Auditable — every verdict traces to the bytes
`audit_trail.py` renders the proof DAG: each contribution carries its rulebook
clause's **source + trust tier**, and each clause links (via
`grounding/grounding-results.json`) to the **verbatim primary-source byte-quote**
and URL the spider extracted. A reviewer follows the diagnosis from the verdict
down to, e.g., *"CSF Gram stain likely has … sensitivity (85% …) and very high
specificity (99% …)"* (WHO NBK614844). The reasoning is inspectable, not a number.

### 4. Error-localizable — a wrong verdict is one clause
Two mechanisms. (a) The proof DAG (proof 3): a wrong diagnosis is exactly one
wrong contribution line; edit that clause in the CAS and re-derive (proof 2). (b)
Rulebook self-consistency (`consistency/*.adj`, M7): an invariant over the
rulebook's own structure (the two priors must partition to 1.0) is a
`constrain`/`check`; a mis-authored prior yields **UNSAT with an IIS `core`**
naming the exact conflicting clauses — machine-checked "these two rules
contradict."

### 5. CPU-bound — the reasoning is not a model
`golden_and_cpu.py` times each decide at **~26 ms/case**: a CLI call (parse + LR
aggregation over the imported rulebook), no network, no model. Intelligence lives
in the grounded rulebook + the engine, not in an answer-time forward pass.

## Value-of-information — "what should we order next?"
`warm/voi.py` (M7): for a case, rank the **unobserved** findings by how much
observing each would move the differential. On the knife's-edge pre-culture case
it flags that `csf_lactate(normal)` / `enteroviral_pcr(positive)` would **flip**
the leading diagnosis; on a confident case it ranks CSF culture / Gram stain as
the top discriminating tests. Pure CPU; each order-next cites the clause that
would fire.

## Honest limits (surfaced, not hidden)
- **"Byte-stability" = two-reader re-extraction agreement**, not a literal
  byte-diff against fetched HTML (`WebFetch` returns model-summarized markdown).
  A true byte-diff needs a raw-fetch tool — a clean follow-up.
- **Two flagged clauses** (`csf_culture` 271 = definitional not study-anchored;
  `csf_neutrophilic` 15 = conservative, source supports a higher LR at extreme
  thresholds) are downgraded to `inferred` by the M5 gate and carry imperfectly-
  aligned quotes — visible in the audit trail, exactly as the gate intends.
- **The independence over-count is symmetric.** The viral arm's CSF-chemistry
  complements are also correlated; a single 4-way `interacts` corrects the
  bacterial full-house but not every co-occurring subset. Per-subset joint terms
  (or a covariance-aware aggregator) are the principled fix.
- **Asymmetric priors + argmax** (see proof 2): the "leader" by raw posterior is
  not the clinical action under asymmetric costs.
- **Pediatric-cohort priors** (Nigrovic), **single-study specificities**, and
  small-model decompose noise (2 hallucinated terms dropped at the vocabulary
  gate on the bacterial case) are recorded in the grounding + IR artifacts.
- **Scope:** no treatment arm, no population stratification, four vignettes — a
  demonstration of the mechanism, not a validated clinical tool.

## What the rebuild demonstrates
The framework is **open-book and CPU-bound**: it targets *auditable, correctable,
defensible* structured reasoning, with the human as auditor of a byte-cited proof
DAG, not author of weights. A bug is a clause you can find, fix once in the CAS,
and propagate everywhere — at zero answer-time model calls.
