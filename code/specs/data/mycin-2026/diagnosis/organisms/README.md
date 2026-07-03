# Organism identification (A1) — *which* bacterium, then cover it

1976 MYCIN did two things in sequence: figure out the **significant organism(s)**,
then recommend therapy that covers them with the fewest drugs. The treatment
vertical already does the second half (set-cover + dose-window). This package is
the **first half** — and the join that makes the diagnosis→therapy path
end-to-end on our substrate.

```
findings ──► organism-id differential ──► significant set ──► set-cover ──► regimen + doses
             (which bacterium?)            (cover these)       (formulary)
```

## How it identifies

Two kinds of evidence, exactly as MYCIN reasoned (`organism-id.adj`):

1. **Gram-stain morphology — near-decisive.** The morphology seen on the CSF
   Gram stain points at the genus: lancet-shaped gram-positive diplococci →
   *S. pneumoniae*; gram-negative diplococci → *N. meningitidis*; gram-positive
   rods → *Listeria*; gram-negative coccobacilli → *H. influenzae*; gram-negative
   rods → enteric GNB; gram-positive cocci in clusters → *S. aureus*. Large,
   definitional likelihood ratios (`trust authoritative`).
2. **Epidemiology — priors + shifters.** Community organism distribution
   (Brouwer/Tunkel/van de Beek, *Clin Microbiol Rev* 2010) as the priors; age
   band, immune status, pregnancy/Listeria exposure, recent neurosurgery,
   crowding, and a petechial rash shift it (`trust consensus`/`empirical`,
   authored from standard references — **not yet spider-grounded**, one CAS edit
   from a grounded value, same honesty boundary as the formulary dose-windows).

## The "still in play" idea (why it produces the right empiric regimen)

The **significant set** is the leader *plus every organism still materially in
play* (normalized share ≥ `IN_PLAY_SHARE`). That is the clinical rule "cover what
could plausibly be there," and it is what makes empiric therapy correct:

- **Older / immunocompromised, pneumococcus on the stain** → the differential
  keeps **Listeria in play** (epidemiology), so the derived regimen is
  **vancomycin + ceftriaxone + ampicillin** — the exact IDSA empiric regimen for
  age > 50, produced with *no therapy rule*, only identify → cover.
- **Gram-negative diplococci + petechiae + dormitory** → *N. meningitidis* leads
  decisively; pneumococcus stays in the set and is covered.
- **Post-neurosurgical, GP cocci in clusters** → *S. aureus* leads → vancomycin.
- **Neonate, no organisms seen** → epidemiology-only broad set; *group B Strep*
  has no empiric token in this meningitis formulary yet, so it is **flagged**
  (honest abstention), not silently dropped.

## Files

- `organism-vocab.adj` — controlled vocabulary (organism hypotheses + findings),
  an importable CAS library; names align with the formulary's organism tokens.
- `organism-id.adj` — the differential rulebook (imports the vocab). **GENERATED**
  by the write gate from grounding — do not hand-edit values.
- `identify.py` — runs the differential, computes the significant set, maps it to
  the formulary, and derives the regimen + dose windows (`python3 identify.py`).
- `test_identify.py` — guards the behavior; 0 answer-time model calls; skips if
  the CLI isn't built.

## Grounding — nothing is hand-authored (G1)

The priors and gram-stain morphology mappings are **not authored** — they are
spider-grounded against primary sources and committed through an adversarial write
gate, the same cold path the meningitis formulary uses:

- `../../grounding/workflows/ground-organism-id.workflow.js` — the spider: one
  agent grounds each claim against a primary source (byte-quote + URL + **discards**
  + ENTAILED/LEAP justification), an independent agent re-extracts and tries to
  refute (byte-stability) → a reconciled verdict.
- `../../grounding/organism-id-grounding.json` — the resulting records.
- `organism_id_ground.py` — the write gate (mirrors `cas_build.py`): `grounded` →
  ACCEPT (source value, trust authoritative); `direction_only`/`refuted`/ungrounded
  → FLAG (kept at trust `inferred`, never silently used) → **regenerates
  `organism-id.adj`** + `organism-id-manifest.json` + the `../../PROVENANCE-LEDGER.md`.
- `test_organism_id_ground.py` — guards the gate + that the rulebook is up to date.

First batch result: 7 priors/morphology mappings GROUNDED (e.g. *S. pneumoniae* 0.51
from van de Beek NEJM 2004), 4 FLAGGED direction-only, **2 REFUTED** (the authored
*S. aureus* community prior — the source figure was nosocomial; and the "GN rods =
enteric GNB" mapping — overlaps H. flu). To correct a wrong fact, edit the grounding
record and re-run the gate — never edit `organism-id.adj` by hand.

## Known limitation (honest)

The grounded priors are still an adult-community distribution, so in the **neonate**
scenario pneumococcus over-leads even though neonatal meningitis is GBS/E. coli/
Listeria. The significant set still pulls in the right organisms to cover, but
population-specific priors (peds/neonatal/immunocompromised) are future work — a
sub-population prior layer, grounded the same way. The **host-factor LRs** (age band,
immune status, exposures) are not yet grounded either — they are carried at trust
`inferred` and tracked as authoring debt in the provenance ledger, queued for the
next grounding batch.

## Known limitation (honest)

The priors are an adult-community distribution, so in the **neonate** scenario
pneumococcus over-leads on the prior even though neonatal meningitis is
GBS/E. coli/Listeria. The significant set still pulls in the right organisms to
cover, but population-specific priors (peds/neonatal/immunocompromised
sub-populations) are future work — the same population-guard item noted in the
broader roadmap. The fix is a sub-population prior layer, not a change to the
mechanism.
