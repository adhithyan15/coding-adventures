# Bacteremia / sepsis (A2) — reason from the source, then cover the blood

Bacteremia (organisms in the bloodstream) was **MYCIN's primary domain**. Its
signature diagnostic move was to infer the likely organism from the **portal of
entry** and host factors, then recommend empiric coverage. This package does that
on our substrate — and it is the *same machinery* as the meningitis vertical (A1)
pointed at a different site, with **no new engine**: that generalization is the
point of A2.

```
source + host factors ──► source-id differential ──► significant set ──► set-cover ──► regimen
        (portal of entry)   (which bloodstream bug?)   (cover these)       (bsi-formulary)
```

## Reasoning from the portal of entry (`source-id.adj`)

| source | organisms it implies |
|---|---|
| urinary | enteric gram-negative bacilli (lead), Enterococcus, Pseudomonas |
| intravascular line | CoNS (lead), S. aureus, Candida (skin flora) |
| intra-abdominal | enteric GNB + anaerobes + Enterococcus (polymicrobial) |
| skin / soft tissue | S. aureus, group A strep |
| respiratory | pneumococcus, Klebsiella |
| host: neutropenia | + Pseudomonas, Candida (anti-pseudomonal coverage matters) |
| host: injection drug use | + S. aureus (right-sided endocarditis) |
| host: prosthetic material | + CoNS (indolent device infection) |

## What the deriver produces (0 model calls)

| scenario | significant set | derived empiric regimen |
|---|---|---|
| Urosepsis | GNB, Enterococcus, S. aureus | piperacillin-tazobactam + vancomycin |
| Central-line BSI | S. aureus, CoNS, GNB | piperacillin-tazobactam + vancomycin |
| **Intra-abdominal** | GNB, anaerobes, Enterococcus | **piperacillin-tazobactam alone** (covers all three) |
| SSTI + injection drug use | S. aureus, GNB | piperacillin-tazobactam + vancomycin |
| Febrile neutropenia, source unknown | GNB, Pseudomonas, S. aureus | piperacillin-tazobactam (anti-pseudomonal) + vancomycin |
| Intra-abdominal + **severe β-lactam allergy** | GNB, anaerobes, Enterococcus | **NO REGIMEN → escalate / specialist** (honest abstention) |

The intra-abdominal case shows the set-cover picking a single broad agent over
multiple narrow ones; the β-lactam-allergy case shows it **abstaining** when this
formulary has no non-β-lactam that covers enteric gram-negatives (a real gap —
aztreonam / fluoroquinolone / aminoglycoside would be the specialist call), rather
than fabricating a regimen.

## Files
- `source-vocab.adj` — controlled vocabulary (bloodstream organisms + source/host
  findings), importable CAS library; names align with the formulary tokens.
- `source-id.adj` — the source→organism differential rulebook.
- `bsi-formulary.json` — systemic empiric formulary (no CSF-penetration filter —
  bloodstream, not CNS). Coverage **authored-illustrative**, clearly marked **not
  yet spider-grounded**; a future formulary-spider pass grounds + gates it exactly
  as it did the meningitis formulary (which corrected three hand-authored errors).
- `identify_bsi.py` — runs the differential, the significant set, and a **generic**
  minimum-cost set-cover (B1 will lift this into the engine as a native, proof-DAG-
  producing `select`). `python3 identify_bsi.py`.
- `test_identify_bsi.py` — guards the behavior; 0 answer-time model calls.

## Honesty boundary
The likelihood ratios and formulary coverage are authored from standard guidance
(IDSA bloodstream / sepsis / SSTI / cIAI, Surviving Sepsis) and are **not yet
spider-grounded** — the same authored→grounded path the meningitis side already
walked. Decision-support only; every premise is one CAS edit overridable.
