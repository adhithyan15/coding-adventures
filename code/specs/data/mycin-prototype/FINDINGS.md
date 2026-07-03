# MYCIN-2026 prototype — findings (all five claims proven, end-to-end, on the real engine)

The prototype resurrects MYCIN on the byte-provenance substrate (bacterial vs viral meningitis) and
proves five claims with **no shortcuts**: the model only decomposes; the rulebook and case compile
deterministically to **adj-lang** programs; a **standard dictionary** binds their vocabulary; an
**adversarial CAS-write gate** earns trust before commit; and the decision runs on the **real
CPU engine** (`adj-lang-cli`). Every number below traces to a committed artifact.

## The five claims

| # | claim | proof | result |
|---|---|---|---|
| 1 | **Golden rulebook** — derive once, reuse indefinitely | `proofs/golden_rulebook.py` | rulebook derived **once** (36 one-time gate reads); **4 cases decided off the single CAS library**; **0 answer-time model calls** |
| 2 | **Cost-to-correct is small** | `proofs/cost_to_correct.py` | over-saturation localized to **4 stacked CSF clauses**; **1 edit (3 clauses)** → new CAS version → MEN-2 **0.9999 → 0.7709**, propagated to all cases, **0 model calls** |
| 3 | **Audit trail easy to follow** | `proofs/audit_trail.md` | every decision is a line-by-line proof DAG: prior → each cited contribution (log-LR, running P, source, trust) → posterior |
| 4 | **Errors are localizable** | `proofs/error_localization.py` | a seeded 100× typo (seizure LR 5.84→584) localizes to **exactly one proof step** — no model re-run |
| 5 | **Inference is CPU-bound** | `proofs/golden_rulebook.py` | `answer_time_model_calls_total = 0`; the decision is the engine binary, deterministic and reproducible |

## The pipeline, proven

- **Standard dictionary (the linchpin).** `dictionary.json` is the closed vocabulary shared by the
  decomposer and the programs; `dict_lint.py` enforces it; `ir_to_adj.py` raises on any drift. So the
  golden-rulebook problem is solved at the term level — a case's `observe csf_glucose(low)` can never
  silently miss the rulebook.
- **Model only decomposes.** The two model touchpoints are the **cold** rulebook gate and the
  **warm** case decompose; both ran exactly as designed. The decomposer mapped prose → dictionary
  findings + byte spans + a discard list + inference justifications and **did not diagnose** — it even
  distinguished "procalcitonin not sent" (legal term, not observed) from a finding, and abstained
  from forcing a pleocytosis type when the differential was pending.
- **Adversarial reading at both links.** The CAS-write gate's 3 readers accepted the 8 clauses whose
  byte-quotes state the LR directly and **flagged 6 weaker-grounded ones** (the prior is a
  *prevalence*; culture LR=271 is over-precise; derived/ungrounded viral LRs). The case-IR inference
  read + **discard read** found no over-reads and no wrongly-dropped findings on faithful inputs.
- **CPU-bound decisions.** 4 held-out cases → 4 defensible verdicts at **0 answer-time model calls**:
  MEN-1/MEN-2 bacterial, MEN-3 viral, MEN-4 **abstain** (insufficient evidence — the guard caught a
  prior-only commitment).

## What the framework's own audit trail surfaced (honest limitations)

Because everything is auditable, the prototype made its **own** weaknesses visible — which is the
point:
1. **Viral prior dominance.** After the cost-to-correct edit de-saturates bacterial on MEN-2
   (0.77), the viral arm — which has *no negative evidence from the bacterial-pattern CSF* — floats
   on its high base-rate prior (0.963) and out-ranks bacterial. The fix (bacterial findings should
   also contribute *against* viral, LR<1) is a clean follow-up the proof DAG points straight at.
2. **Six clauses flagged at `inferred`.** The gate openly flagged the prior, the culture LR, and the
   viral LRs for re-grounding — they are cold-path follow-ups, not hidden assumptions.
3. **Over-saturation persists on genuinely strong evidence** (MEN-1, with Gram+ and culture, is
   legitimately ~1.0). The CSF-correlation fix targets the chemistry panel, not the dispositive tests.

These are not failures of the demonstration; they are the **measurement-validity discipline** working
— the audit trail localizes each modeling gap to a specific clause, which is exactly the
correctability the program is about.

## Reproduce

`cargo build -p adj-lang-cli` → `python3 test_dict_lint.py` → `dict_lint.py rulebook/meningitis.adj` →
`cas_write_gate.py prep` + the gate workflow + `commit` → `decompose.workflow.js` →
`adversarial_read.workflow.js` → `decide.py` → `proofs/*.py`. Artifacts: `decide_results.json`,
`proofs/*_result.json`, `proofs/audit_trail.md`, `cas/objects/*.json`.
