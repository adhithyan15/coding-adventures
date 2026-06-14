# MYCIN-2026 — parity with the 1976 MYCIN, end to end

`mycin_consult.py` is the program that runs MYCIN-2026 the way the 1976 MYCIN
actually ran: an **interactive consultation** that gathers findings by asking the
most decision-relevant question, reaches a diagnosis it can explain, recommends
**culture-directed** therapy, and does it all at **0 answer-time model calls** (the
only model touch is the upstream decompose; here the inputs are clinician-entered
data, exactly as MYCIN took them).

## The flow (a real session)

```
[1] PRESENT ILLNESS: fever, neck stiffness            (sparse intake)
[2] CONSULTATION  (value-of-information drives the questions)
  ? csf_culture      Δmargin −0.875  → unavailable; noted
  ? csf_gram_stain   Δmargin −0.729  → positive
  ? csf_protein      Δmargin −0.192  → high
  ? csf_lactate      Δmargin +0.024  → high
[3] DIAGNOSIS: bacterial_meningitis P=0.9986  (determinate; 3 corroborating findings)
[4] THERAPY
    EMPIRIC: ampicillin + ceftriaxone + vancomycin (cost 3)
    CULTURE BACK — ampicillin-resistant Listeria:
    CULTURE-DIRECTED: ceftriaxone + meropenem + vancomycin (cost 5)   # ampicillin dropped, re-derived
answer-time model calls across the whole consultation: 0
```

## What this closes — the last two 1976 behaviors

1. **The interactive consultation loop.** MYCIN's signature UX was a dialogue: it
   asked for the datum it most needed, took the answer, and re-derived. We already
   had the value-of-information *ranking* (`warm/voi.py`); this drives the **Q&A loop**
   on top of it — ask the highest-VOI unobserved finding, record the answer (or note
   it unavailable and move on), re-derive, and stop once the diagnosis is determinate
   with enough corroborating evidence or no remaining question would change it. Each
   question is justified by its VOI (the **WHY**), and the diagnosis cites its
   contributing findings (the **HOW**).
2. **Live sensitivity ingestion.** MYCIN refined empiric therapy with culture
   results. `native_setcover.py` now takes `defeated` (drug, organism) edges — an
   in-vitro **resistant** result voids that coverage edge (and any combination whose
   member is resistant), so the regimen **re-derives** around it. This wires the B2
   defeasance construct into the live therapy step.

## Full 1976 capability map (now)

| MYCIN (1976) | MYCIN-2026 | status |
|---|---|---|
| production rules + backward chaining | grounded adj-lang rulebooks (CAS) | ✅ |
| certainty factors for uncertainty | calibrated probabilistic LR engine | ✅ beyond |
| identify the significant organism(s) | organism-id differential (A1) + source→organism (A2) | ✅ |
| interactive consultation (Q&A) | **VOI-driven dialogue loop (this)** | ✅ |
| explanation: WHY / HOW | VOI justification + proof DAG / audit trail | ✅ |
| therapy: fewest drugs, by preference, dosed | minimum-cost set-cover + dose-window UNSAT | ✅ beyond |
| combination therapy | n-ary combination coverage (B2) | ✅ |
| **culture sensitivity directs therapy** | **defeasance in the live therapy (this)** | ✅ |
| domain = bacteremia + meningitis | both | ✅ |

**Beyond 1976:** byte-grounded provenance + an adversarial write gate (it caught
real hand-authored errors), a content-addressed *correctable* knowledge base,
local small-model decomposition of messy/spoken input, FHIR chart ingestion, dose
infeasibility (no safe dose), and 0-answer-time-model-call CPU inference.

**Remaining is breadth, not capability:** more organisms / sites / sub-populations
(e.g. peds/neonatal priors — the organism-id uses adult-community priors today).
That is an expansion track; the *mechanisms* are all present and demonstrated.

## Run it

```sh
python3 mycin_consult.py                 # scripted (answers from the case oracle)
python3 mycin_consult.py --ask           # interactive (prompts you per question)
```

Decision-support only — every question, finding, and drug is grounded and
overridable; the physician makes the call.
