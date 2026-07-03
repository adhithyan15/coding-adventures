# Treatment as a solved constraint problem — findings

MYCIN (1976) recommended therapy with ~hundreds of hand-written therapy rules:
the guideline's pre-solved answers, frozen into the rulebook. We do the opposite.
We store only **drug facts** — a grounded *formulary* — and the engine **derives**
the regimen as a constraint solve. Same clinical behavior; it generalizes to new
organisms and drugs with no new rules, and it can prove when *no* safe regimen or
dose exists.

## The pipeline

```
formulary-spider (live web)                     warm path (per patient, 0 model calls)
   30 drug-coverage + CSF facts                    organisms (from the differential)
   each: ground + independent re-extract                 │
        │                                                ▼
   ADVERSARIAL GATE (formulary_build.py)           min-cost SET-COVER over CSF-penetrant,
   ACCEPT (grounded+stable+entailed)               non-contraindicated drugs + combination
   FLAG  (unclear/unstable → trust inferred)       rules  → DERIVED REGIMEN
   DROP  (source refuted the fact → removed)             │
        │                                                ▼
   cas/objects/<hash>.adj  (importable lib)        DOSE WINDOW solve: floor ≤ dose ≤ ceiling
        └──────────── import ───────────────►      (ceiling shrinks with renal/interaction
                                                    risk) → SAT range, or UNSAT + IIS
```

## What the gate caught (the point of the adversarial read)

The spider grounded the formulary against primary sources (IDSA Tunkel 2004 +
re-extraction), and the gate **dropped three facts I had hand-authored wrong**:

- `vancomycin covers s_pneumoniae_resistant` **alone** — REFUTED. IDSA: vancomycin
  "should not be used alone (A-III)"; it must be combined with a 3rd-gen
  cephalosporin. This refutation is what forced combination modeling (below).
- `ampicillin covers E. coli` — REFUTED (high empiric resistance).
- `moxifloxacin covers s_pneumoniae_resistant` — REFUTED as a standalone claim.

FLAG (kept at trust `inferred`, never deleted — the M5 rule): vancomycin CSF
penetration (penetrates only when meninges are inflamed → "unclear", not "false")
plus five re-extraction-unstable facts. **15 facts ACCEPTED.** The formulary got
*more correct and more honest in the same step.*

## What the deriver produces (0 model calls, from the CAS object)

1. **Adult community** → `vancomycin + ceftriaxone`, citing the grounded
   *combination* rule (single-agent set-cover can't express resistant
   pneumococcus — the gate's refutation is exactly why).
2. **Post-neurosurgical** (adds Pseudomonas/MRSA) → `vancomycin + cefepime` — a
   different regimen from the *same facts*, no new rule. Set-cover generalizes.
3. **Severe β-lactam allergy** → **no grounded β-lactam-free combination covers
   resistant pneumococcus → honest abstention** ("escalate / specialist"). It
   refuses rather than fabricate a regimen.
4. **Vancomycin in severe renal failure + a nephrotoxin interaction** → **DOSE
   UNSAT**: the efficacy floor (15 mg/kg) exceeds the safe ceiling (8 mg/kg); the
   solver returns the IIS. "There is no safe effective dose" — the call a human
   misses under load — is surfaced, not silently rounded.

## Honesty boundaries (what is and isn't grounded)

- **Grounded by the spider:** drug→organism coverage, CSF penetration, the
  combination rules (grounded by the spider's own refutation quotes).
- **Authored-illustrative (clearly marked):** the dose-window numbers and the
  preference tiers are a *mechanism demo*, not validated PK/PD. They show the
  UNSAT machinery; they are not a dosing tool. Every one is one CAS edit away
  from a grounded value.

## Files

- `treatment/antibiotics/formulary.json` — authored drug facts + combination rules.
- `treatment/antibiotics/formulary_build.py` — the adversarial gate → CAS library.
- `treatment/antibiotics/cas/` — the content-addressed, importable formulary.
- `treatment/antibiotics/derive_regimen.py` — set-cover + dose-window deriver.
- `treatment/antibiotics/meningitis-abx.adj` + `select_abx.py` — the grounded
  component-selection rulebook + renal vanc dose + door-to-antibiotic timing.
- `consult.py` — the composed flow: decompose → differential → VOI → therapy.
- `grounding/formulary-grounding.json` — the 30 spider records (21 grounded, 3
  refuted, the rest flagged).
