# MYCIN-2026 prototype — specification (full pipeline, no shortcuts)

> Resurrects MYCIN on the byte-provenance substrate, on its own domain (bacterial vs viral
> meningitis), to prove **five claims end-to-end**. Plan of record: the approved build plan.
> Companion paper specs: [`PAPER2-MYCIN-build-spec.md`](../../papers/PAPER2-MYCIN-build-spec.md),
> [`PAPER2-MYCIN-derive-once-proof.md`](../../papers/PAPER2-MYCIN-derive-once-proof.md).

## 1. The five claims this prototype proves

1. **Golden rulebook** — a rulebook is derived **once** (with the model + an adversarial gate) and
   reused on many held-out cases at **zero answer-time model calls**.
2. **Cost-to-correct is small** — a wrong rule **localizes to one clause** in the audit trail, and a
   **single CAS edit** fixes it and **propagates** to every dependent case (0 model calls).
3. **The audit trail is easy to follow** — every decision is a proof DAG: prior → each contribution
   (with its cited source bytes) → posterior.
4. **Errors are localizable** — a wrong verdict traces to exactly one clause/fact.
5. **Inference is CPU-bound** — the decision is computed by the deterministic adj-lang/logic engine,
   not the model.

## 2. The hard constraint — the model only decomposes

The LLM is used at exactly **two** points, and **never reasons toward the answer**:
- **Cold (once):** translate byte-grounded literature into candidate adj-lang clauses, then submit
  each to the adversarial CAS-write gate.
- **Warm (per case):** **decompose** a messy clinical vignette into typed IR (findings as dictionary
  terms, with byte spans, type, polarity, a discard list, and inference justifications). It does **not**
  diagnose.

Everything downstream — IR→adj-lang emission, compilation, the differential, the proof DAG — is
**deterministic / CPU**. The diagnosis is the engine's, not the model's.

## 3. The standard dictionary (the linchpin) — [`dictionary.json`](dictionary.json)

A controlled vocabulary **shared** by the decomposer and the rulebook/case programs. Each canonical
term is `functor(value)` (findings) or a bare atom (hypotheses), with a definition, the surface forms
the decomposer maps prose from, and a value domain. It (a) **constrains the decomposer** (its prompt
lists the legal terms), (b) is **enforced** by [`dict_lint.py`](dict_lint.py) — which rejects any
rulebook/case `.adj` that uses an unregistered finding or hypothesis — and (c) makes "finding absent"
(a value, observed) distinguishable from "finding not yet observed" (term legal, no `observe` line),
because the finding set is **closed**. Without it, a case's `observe csf_glucose(low)` could silently
miss a rulebook clause written `csf_glu(low)` — the golden-rulebook problem in miniature.

## 4. Pipeline (data flow)

```
COLD (once):  grounded LRs (adj52 byte_quotes) → rulebook/meningitis.adj
              → CAS-write gate (N adversarial readers: byte_quote ⊨ LR? × byte-stability
                × blind-judge × completeness/discard) → ACCEPT(trust tier) | KICKBACK
              → cas/objects/<hash>.json  (the content-addressed ADJ LIBRARY)

WARM (per case):  vignette prose
              → decompose (LLM, dictionary-constrained, decompose-only)
              → typed IR: findings(term, span, stated|inferred, polarity)
                          + DISCARD list (unmapped spans + reason)
                          + inference justifications (ENTAILED/LEAP basis)
              → adversarial_read: inference read + DISCARD read × N-reader vote × decision-sensitivity
              → ir_to_adj.py (DETERMINISTIC, dict-validated) → case .adj  (observe… ? bacterial ? viral)
              → decide.py: concat(CAS rulebook, case) → adj-lang CLI → differential + proof DAG
              answer-time model calls = 0
```

## 5. Components (build order; one PR each)

| PR | component | reuses |
|---|---|---|
| 1 | **this SPEC**, `dictionary.json`, `dict_lint.py` (+ tests) | — |
| 2 | **adj-lang CLI** (`code/packages/rust/adj-lang/src/bin/`) → JSON differential + proof DAG | adj-lang `compile`/`decide`, logic-engine `differential`/`proof_dag` |
| 3 | `rulebook/meningitis.adj` (bacterial + grounded viral arm) + **CAS-write gate** | `adj52/corpus` byte_quotes; `run100b/adversarial_entail`, `decision_sensitivity_gate`, `cas_exercise` |
| 4 | **case pipeline**: `decompose` + `adversarial_read` (incl. **discard read**) + `ir_to_adj.py` + `decide.py` | `run100/extract100` IR shape; `FORWARD-DESIGN` discard design |
| 5 | **the five proofs** + `FINDINGS.md` | `adj52/cas/overrides/meningitis-csf-correlation.json` (the cost-to-correct bug+fix) |

## 6. The cost-to-correct demonstration (claim 2, concrete)

Ship a **naive** rulebook that treats the four correlated CSF findings (neutrophilic pleocytosis,
low glucose, elevated protein, elevated lactate) as **independent** `contributes` clauses. On the
pre-culture case this **over-saturates** to P ≈ 0.9999 before any culture — the documented ADJ56
failure. The **proof DAG localizes** the error to those four stacked contributions. A **single CAS
override** — keep one representative `contributes` + add an explaining-away `interacts` (joint LR < 1),
the fix in [`adj52/cas/overrides/meningitis-csf-correlation.json`](../adj52/cas/overrides/meningitis-csf-correlation.json)
— re-derives to a calibrated P ≈ 0.77 and **propagates** to every case citing those clauses, at **0
answer-time model calls**. The metric: **1 edit, 1 localized locus, propagates**.

## 7. Verification

`python3 test_dict_lint.py`; `python3 dict_lint.py rulebook/meningitis.adj cases/*.adj`;
`cargo test -p adj-lang` (CLI golden tests); `proofs/golden_rulebook.py` (0 calls);
`proofs/cost_to_correct.py` (naive→localize→1 edit→calibrated→propagate). Posteriors spot-checked
against [`adj52/corpus/eval.py`](../adj52/corpus/eval.py).

## 8. Non-goals (so "small" stays small)

Treatment arm; population stratification; full-text PDF caching / automated citation retrieval;
native adj-lang term-validation (pipeline-level enforcement only this round); LR confidence-interval
propagation. Each is a clean follow-up.
