# FHIR chart ingestion (D1) — run off a real chart export

The "open format like EPIC" is **HL7 FHIR** — the API EPIC, Cerner, and every
modern EHR expose. This package ingests a FHIR `Bundle` (a self-contained JSON
chart export — no network call) into the MYCIN-2026 warm path.

```
FHIR Bundle ──► extract ──► coded findings  (0 model calls) ─┐
                        └► narrative text → decompose_text   ─┴► ir_to_adj → decide
                                              (1 on-device call, only if needed)
```

## The headline: a coded chart needs **zero** model calls

An EHR's labs, vitals, and problem list are usually **coded** — LOINC on
`Observation`s, SNOMED on `Condition`s — with an HL7 **interpretation** flag
(`H`/`L`/`N`/`POS`/`NEG`). So they map to typed dictionary findings
**deterministically**, by a pure lookup (`fhir_code_map.json`). When the chart is
coded, the *entire* pipeline runs at **0 model calls** — not even the decompose:
the structured data goes straight to the CPU engine. Free-text narrative (an HPI)
still falls back to the on-device decomposer. Either way, nothing leaves the
machine.

Verified on the synthetic coded bundle:

```
$ python3 run_fhir.py samples/meningitis_bundle.json

[1] CODED FINDINGS (deterministic from LOINC/SNOMED, 0 model calls):
    csf_glucose(low), csf_protein(high), csf_neutrophilic_pleocytosis(high),
    csf_gram_stain(positive), fever(present), meningismus(present)
    ALLERGIES (carry to therapy): ['Penicillin']
[2] bacterial_meningitis P = 0.9999  <- leading   (determinate)
total model calls: 0   |   chart data left the machine: none
```

The penicillin allergy is carried out of the chart for therapy (a severe
β-lactam allergy is exactly the exclusion the set-cover deriver re-derives around).

## Files

- `fhir_code_map.json` — the LOINC/SNOMED + interpretation → finding map. Real code
  systems; the value-derivation rule (interpretation flag, or a temperature
  threshold for fever) is named per entry. Authored from standard code systems,
  not spider-grounded; one edit overridable.
- `fhir_ingest.py` — parse a Bundle into `{demographics, findings (typed, 0 model
  calls), unmapped (surfaced, never guessed), narrative (for the decomposer),
  allergies, medications}`. Robust to missing fields and unknown codes; pure JSON,
  no network call.
- `samples/meningitis_bundle.json` — a synthetic coded bacterial-meningitis chart.
- `run_fhir.py` — Bundle → diagnosis (`python3 run_fhir.py <bundle.json>`).
- `test_fhir.py` — mapping rules (interpretation + Fahrenheit normalization),
  robustness to empty/messy resources, unknown-code-not-guessed, and the full
  coded-chart → bacterial_meningitis. No model required; CI runs it.

## Honesty / limits

- The code map is intentionally focused on the meningitis findings the dictionary
  defines; it is a worked subset, not a complete LOINC/SNOMED terminology service.
  Adding a finding is one map entry.
- Exact patient **age** is not derived: a FHIR `birthDate` needs the encounter date
  (a "now") to compute age, and we keep ingestion deterministic (no wall-clock).
  Age-banded priors are future work (see `../diagnosis/organisms/` A1); the
  differential is driven by the clinical findings, not age math.
- An `Observation` with only a free-text value (no interpretation flag) is treated
  as narrative for the decomposer rather than guessed into a value.
