# ADJ48 — MYCIN-2026 in Adj-Lang: End-to-End Clinical Demo

> **Headline.** Five real-ish ED chest-pain vignettes run end-to-end
> through Adj-Lang against a 29-clause ACS rulebook compiled from
> AHA/ACC/ESC guidelines + Panju 1998 + Diamond/Forrester 1979 + the
> HEART Score literature. Every fired contribution in every case
> output names its citation. The framework distinguishes "this
> uncertainty changes the decision" (Jane Doe: kickback at 30%
> threshold) from "this uncertainty exists but is no longer
> load-bearing" (same Jane Doe with troponin rising: no kickback) —
> the practical clinical utility a status-quo LLM cannot deliver
> because it cannot represent the uncertainty explicitly.

## What this milestone is

After ADJ47 closed the awkwardness catalogue across `logic-engine`
0.3.0–0.6.0 and `adj-lang` 0.1.0–0.2.0, the natural test is to
exercise the full stack on a meaningful clinical domain in one
pass:

```
rulebook.adj  +  vignette.adj
        │
        ▼ [adj-lang::compile]
   LoweredProgram { kb, queries }
        │
        ▼ [logic_engine::search, SearchMode::LRAggregate]
   LRAggregateResult { posterior, dag, uncertainties, warnings }
        │
        ▼ [LRAggregateResult::suggest_kickback(threshold)]
   Per-case audit document
```

The 29-clause rulebook + five vignettes + runner binary +
captured output ship as `code/specs/data/adj48/`. Every line of
output is grounded in a citation an ED physician can click and
verify.

## Vignettes and results

| # | Description | P(acs) | Decision-threshold 30% kickback? |
|---|---|---|---|
| 1 | Jane Doe — 62yo M, pressure pain, diaphoresis, vitals normal, ECG no acute STC, **precipitator unknown** | 0.369 | **Yes** — band [0.260, 0.594] straddles 0.30 |
| 2 | Classic STEMI — exertional pressure, bilateral arm radiation, diaphoresis, dyspnea, new ST elevation | 0.997 | No |
| 3 | Pleuritic MSS — sharp pleuritic pain in 28yo F, no PMH, vitals normal | 0.002 | No |
| 4 | NSTEMI equivocal — pressure pain, **three concurrent uncertainties** (precipitator, ECG vintage, troponin pending) | 0.734 | (multiple — see output.txt) |
| 5 | Same Jane Doe but **troponin rises** dynamically on serial measurement | 0.824 | **No** — troponin dominates; precipitator uncertainty no longer load-bearing |

The 1 vs 5 contrast is the headline finding.

## The headline finding: distinguishing decision-relevant uncertainty

Vignette 1 and vignette 5 have **the same precipitator
uncertainty** — the patient said "no clear precipitator" in both.
But the framework treats them differently:

- Vignette 1: the precipitator's VOI band straddles the 30%
  admit/discharge threshold. Kickback recommended. "Get the
  precipitator before committing."
- Vignette 5: the same precipitator's VOI band is the same in
  *log-odds space*, but the running posterior is at 0.824 — well
  above 30%, with a worst-case lower bound that's still well
  above 30%. **No kickback.** The precipitator uncertainty exists
  and is reported, but the user is told "the decision doesn't
  depend on it at this threshold."

A status-quo LLM has no way to make this distinction because it
has no representation of uncertainty at all. It would either
mention the precipitator uncertainty as prose for both cases
(leaving the reader to do the math) or omit it for both (losing
information).

The framework's representation makes the distinction explicit and
mechanical. That's the practical-clinical-utility argument the
framework was built to deliver.

## What dissolves at this point

The framework spec's promises, as of ADJ48:

| Promise | Status at ADJ48 |
|---|---|
| Defensible per-claim audit trail | ✅ proof DAG with provenance per step |
| Probabilistic Bayesian inference | ✅ LP19e LR aggregation in production |
| Uncertainty as a first-class signal | ✅ `uncertain { ... } for ...` + VOI reports |
| Kickback when uncertainty changes the decision | ✅ `suggest_kickback(threshold)` |
| Counterfactual queries | ✅ `counterfactual(query, kb, &[Term])` |
| Source-disagreement detection | ✅ `source_disagreements(kb, conclusion)` |
| Domain-expert-readable rulebooks | ✅ `adj-lang` surface syntax |
| End-to-end clinical demonstration | ✅ ADJ48 (this spec) |

## What's not in this spec

- **Physician comparison.** The Yu et al. 1979 evaluation of MYCIN
  vs. five infectious-disease faculty is the right validation
  experiment. Cheap in compute, expensive in physician recruitment.
  Out of scope for this PR.
- **VOI ranked by test cost.** The kickback report names the
  uncertainties; it doesn't yet rank them by test cost ($25
  troponin in 4 hours vs. 5-minute history). That layer needs an
  EMR adapter and a cost model.
- **Recursive rulebook derivation from PDFs.** ADJ44's pipeline
  can produce this rulebook automatically; here it's hand-written
  for reviewability. Both approaches converge on the same .adj
  file.

## See also

- [ADJ46](ADJ46-acs-rulebook-on-logic-engine-toolchain-shakedown.md)
  — the awkwardness catalogue this work closes.
- [ADJ44](ADJ44-mycin-2026-meningitis.md) — the original MYCIN-2026
  reproduction (meningitis differential).
- [LP19e](LP19e-likelihood-ratio-aggregation.md) — the engine-level
  LR aggregation spec.
- [`code/packages/rust/logic-engine`](../packages/rust/logic-engine/)
  — the inference layer.
- [`code/packages/rust/adj-lang`](../packages/rust/adj-lang/) —
  the surface-syntax frontend.

## Status

- 2026-06-02: ADJ48 runner runs all five vignettes end-to-end;
  rulebook + vignettes + output captured.
- Next: **ADJ49** — M&A deal demo answering the investment-banker
  objection. Same architecture, different domain rulebook.
