# ADJ56 — Cross-Domain Stress Test + Two Honest Failure Modes

> **Status (2026-06-04):** Done. Extends [ADJ55](ADJ55-provenance-first-corpus.md)
> (which proved byte-provenance grounding on one PE case) by (a) stress-testing PE at
> n>1, and (b) grounding two more sub-domains and diagnosing a case in each. The
> headline is *not* "the framework wins everywhere" — it is a precise map of where it
> helps, where it merely matches, and two named failure modes. Artifacts under
> [`code/specs/data/adj52/corpus/`](data/adj52/corpus/) and
> [`.../provenance/`](data/adj52/provenance/).

## 1. The method generalizes (construction)

The forward provenance spider + the generic `corpus/build.py` / `corpus/eval.py`
tooling built three grounded corpora, case-blind, with no per-domain code:

| domain | links grounded | sample grounded LRs |
|---|---|---|
| pulmonary_embolism | **12/12** | D-dimer+ 1.64, CTPA+ 20.75 |
| streptococcal_pharyngitis | **9/9** | tonsillar exudate 3.4 (Ebell 2000), RADT+ 18.6, culture 92.5 |
| bacterial_meningitis | **9/9** | CSF Gram+ 85 (WHO/Straus), lactate 22.9, culture 271 |

Across cardiopulmonary, ENT-infectious, and CNS-infectious medicine, the construction
holds: every magnitude traces to primary data with a byte-anchored chain.

## 2. PE stress test (n = 4) — the edge is real but narrow

| case | truth | grounded | plain Claude | ungrounded |
|---|---|---|---|---|
| PMC11999957 (Wells-0 trap) | present | 0.28→**0.89** ✅ | 3–5%, won't image ❌ | 1% ❌ |
| rule-out (fat necrosis) | excluded | **0.065** ✅ | 10% ✅ | 10% ✅ |
| confirm (post-surgery + tachy) | present | 0.48→**0.95** ✅ | 80% ✅ | 95% ✅ |
| trap (D-dimer+ → CTPA+) | present | 0.28→**0.89** ✅ | 96% ✅ | 98% ✅ |

**The framework's *correctness* edge appeared in 1 of 4 cases — the Wells-0 trap.** On
the three clear-cut cases all three arms agreed and were correct. So PMC11999957 was
robust but **not representative**: it is the case where a prediction-rule gestalt
(Wells 0) actively misleads the unconstrained reasoner, and the grounded base rate
keeps the real PE alive. On clear cases the framework's value reverts to
**auditability** (same answer, every number traceable) and more calibrated
intermediates (grounded pretest 48% vs plain's 80% on the confirm case; the ungrounded
arm runs hot at 0.95–0.98). Honest framing: **the framework does not out-diagnose
frontier Claude across the board — it catches the discordant/trap cases and makes the
agreed answer defensible.**

## 3. Two honest failure modes (the most useful results)

### 3.1 Population extrapolation — strep in a 62-day-old infant

Real case: a 62-day-old infant with exudative pharyngitis whose throat culture grew
*S. pyogenes* (GAS present — but a *rare* host; GAS pharyngitis is a disease of
children > 3 years).

| arm | P(GAS) | note |
|---|---|---|
| grounded corpus | 0.88 → **0.999** | "correct" — but by extrapolation |
| plain Claude | 0.20 (suspected GBS; full sepsis workup + empiric abx) | clinically **best** reasoning |
| ungrounded | **0.002** | wrong — over-penalized age |

The grounded corpus scored "correct" only because this rare infant *happened* to have
GAS. Mechanically it applied a **child prior (0.37) and an `age(under_15)` LR that
*raises* GAS** to a 2-month-old — outside the population those numbers were grounded
in. On a typical infant with viral pharyngitis it would confidently over-call.
**A grounded corpus is only valid inside its grounding population**; the corpus had no
"infant" node and silently extrapolated. Plain Claude's clinical nuance (infant → GBS
more likely, treat empirically, Lancefield-group) was the safer read.

### 3.2 Correlated over-saturation — pneumococcal meningitis

Real case: pneumococcal meningitis with every CSF parameter abnormal. The grounded
sequential update:

```
prior 0.037 → gram+ (85) → 0.77 → neutrophils (15) → 0.98 → glucose (18) → 0.999
            → protein (9.33) → 1.000 → lactate (22.9) → 1.000 → ... → P = 1.0000
```

Every LR is individually grounded and correct, but the CSF parameters are
**manifestations of one process** (bacterial infection). Multiplying them as if
independent **saturates to 1.0000 before the culture even fires** — the exact
Naive-Bayes over-counting the ADJ53/ADJ54 `mechanism` construct exists to fix (group
the correlated findings, fire once). Right answer, indefensible confidence.
**Grounding the magnitudes (ADJ55) is necessary but not sufficient — you still need
correlation-aware combination (ADJ54).**

## 4. The unifying result

Byte-provenance (ADJ55) grounds the numbers; calibration + correlation handling
(ADJ54) combines them. The stress test proves you need **both**:

- ADJ55 alone over-saturates on correlated evidence (meningitis) and over-extrapolates
  past its grounding population (infant strep).
- ADJ54 alone (calibration on invented numbers) was the case-5 disaster.

The framework's clean wins are concentrated on **discordant/trap cases**; on clear
cases it matches frontier Claude with added auditability; and it has **real, nameable
failure modes**. That is a more credible — and more useful — story than universal
dominance.

## 5. Next

- **Mechanism-group the meningitis CSF parameters** (ADJ53 construct) and re-run §3.2;
  it should temper 1.0000 to a calibrated-but-still-high posterior.
- **Population-stratified priors** — an `infant`/age-band node so the strep corpus
  stops extrapolating; more generally, every corpus should record its grounding
  population and refuse (or flag) out-of-population cases.
- **Ground LR-for-absence** (carried over from ADJ55 §6).
- A clean three-arm comparison at larger n per domain, now that three corpora exist.
