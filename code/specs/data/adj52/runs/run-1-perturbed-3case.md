# ADJ52 — first fully hands-off run (3 perturbed published cases)

Run via `pipeline.workflow.js` (Workflow task `w4vuex97b`), 15 agents, no human
in the loop. Three published "masquerade" cases, each diagnosis-invariantly
perturbed to defeat training-data recall. I read the aggregate, not each case.

## Scorecard

| | result |
|---|---|
| cases | 3 |
| **framework won** | **1** |
| **plain Claude won** | **2** |
| tie | 0 |
| framework correct | 2/3 (1 partial) |
| plain correct | 2/3 (1 partial) |
| compile failures | **0** |
| perturbation preserved diagnosis | **3/3** |

## Per case (blind judge)

- **case-1 — McArdle disease mimicking PMR.** Perturbed (age 64→58, CRP 103→88,
  CK 3473→3210, anecdote reworded, PMR label + all confirmatory results
  removed). Framework: McArdle 100%, next step PYGM — **correct**. Plain:
  **correct**. **Plain won** — framework overconfident (P=1.0000), omitted
  treatment/disposition, downranked biopsy; plain was calibrated, named the
  exact mutation, and covered management.
- **case-2 — Zenker (pharyngo-oesophageal) diverticulum mimicking a thyroid
  nodule.** **Framework won.** Framework got the core right (a pharyngo-
  oesophageal diverticulum — specifically Killian-Jamieson, anatomically apt
  for a left-posterior lesion, with Zenker ranked #2). **Plain Claude was
  WRONG** (migrated fish-bone / foreign-body granuloma / fistula). The
  framework won *because the base model erred* — despite its own 100%
  miscalibration.
- **case-3 — Hansen's disease (leprosy) mimicking seronegative RA.** Framework:
  multibacillary leprosy 100%, next step slit-skin smear — **correct**. Plain:
  **correct**. **Plain won** — framework P=1.0000 **and an incoherent posterior
  set** (paucibacillary 50.9% alongside multibacillary 100% — not a coherent
  distribution), plus risky pinpoint citations; plain was calibrated (~90%) and
  added the safety/management actions the framework omitted (stop methotrexate,
  check G6PD before dapsone, reaction monitoring, contact screening).

## What this establishes (now consistent across 5 cases total)

1. **Calibration is THE blocker.** The framework's saturated, internally-
   incoherent posteriors (multiple hypotheses ~100%; case-3's 50.9% + 100% set)
   lose to plain Claude's graded confidence *even when the framework is
   correct*. The judge cites this every time. This is the #1 fix: a **coherent,
   normalized differential** (compete the mutually-exclusive diagnosis queries,
   softmax over their log-odds, temper extremes) — not more citations.
2. **The framework wins exactly where the base model fails** (case-2: plain
   hallucinated a fish-bone; framework got the diverticulum family). That is the
   regime where the framework earns its keep.
3. **Disposition gap.** The framework answered diagnosis + next test but omitted
   treatment/management actions the judge rewarded. It should also query
   disposition/management, not just diagnosis.
4. **Perturbation works** — 3/3 diagnoses preserved under heavy surface change;
   recall defeated; both arms forced to reason.
5. **The automation works** — 15 agents, 0 compile failures (the grammar-valid-
   atom fix held hands-off), perturbation→ingest→derive→run→judge→aggregate with
   no human extraction.

## Next

Highest-leverage change: the **calibration / coherent-differential fix** in the
runner (it would plausibly flip case-1 and case-3). The rulebook/program
separation + accumulating store is built and committed; a same-area sequential
run is what would exercise accumulation.
