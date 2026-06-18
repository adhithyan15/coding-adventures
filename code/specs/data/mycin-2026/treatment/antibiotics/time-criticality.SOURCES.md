# Sources — disease time-criticality (CC-5b)

The `_TIME_CRITICALITY` table in [`chart_to_cop.py`](chart_to_cop.py) classifies a disease's
acuity (`time_critical` vs `routine`), which feeds the wait-vs-treat-now precedence ladder
(`timing.adj`). This ledger grounds that classification — replacing the prior `[FLAG: authored]`
paraphrase with verbatim guideline text.

## meningitis → `time_critical`

| Field | Value |
|-------|-------|
| Verbatim quote (start promptly) | "When a patient presents with suspected acute bacterial meningitis, the physician should begin antimicrobial therapy as soon as possible." |
| Verbatim quote (emergency) | "Bacterial meningitis is a neurologic emergency; progression to more severe disease reduces the patient's likelihood of a full recovery." |
| Charter | IDSA *Practice Guidelines for the Management of Bacterial Meningitis* (Tunkel et al., *Clin Infect Dis* 2004;39:1267) |
| Locator | Tunkel 2004, CID 39:1267; AAFP summary (*Am Fam Physician* 2005;71(10):2003) |
| Trust | authoritative |
| Retrieved | 2026-06-17 |

### Honesty correction (why no ≤60-min number)

The earlier entry asserted a `treat_within_min: 60` ("target ≤1 hour") door-to-antibiotic
threshold attributed to IDSA at `consensus` trust. **That was an overclaim.** The IDSA
meningitis guideline does not set a hard numeric door-to-antibiotic threshold for meningitis —
it says to start antimicrobials "as soon as possible" and that meningitis is "a neurologic
emergency." A specific ≤60-minute figure is a **sepsis / quality-bundle** operationalization,
not IDSA meningitis guidance. Grounding surfaced the mismatch, so the table now represents the
urgency qualitatively (`treat_target: as_soon_as_possible`) — faithful to what the cited source
actually states. Catching a mis-asserted pivot value is exactly what byte-provenance grounding
is for.

## Retrieval URLs

- AAFP summary of the IDSA guidelines (verbatim quotes above):
  <https://www.aafp.org/pubs/afp/issues/2005/0515/p2003.html>
- IDSA guideline of record (Tunkel et al. 2004), *Clinical Infectious Diseases*:
  <https://academic.oup.com/cid/article/39/9/1267/402080>
- Supporting empirical evidence — time-to-antibiotic and outcome (Danish population cohort):
  <https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4977612/>

## Still routine (no time-critical edge)

A disease absent from the table defaults to `routine` acuity — there is room to await a culture
when the patient is stable. Adding a new `time_critical` disease must come with its own grounded
quote here, same as meningitis.
