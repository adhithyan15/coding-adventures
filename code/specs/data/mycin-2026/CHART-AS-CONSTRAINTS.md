# Chart-as-Constraints — the whole patient chart becomes a constraint program

**Directive (2026-06-14):** *"Everything in the patient chart should become a constraint
in one way or the other"* — comorbidities, dosing, cost, the wait-for-results-vs-treat-now
decision, side effects, and insurance/formulary rules (e.g. step therapy: a payer won't
approve drug Y until regimen X has been tried). Solve it with the **constraint solver we
already built** (`adj-constraint-solver`). This document specs that architecture.

It does **not** replace the diagnostic side (organism-id LRs, the differential). It
replaces the *treatment-selection* side: today a min-cost **set-cover** picks drugs to
cover the likely organisms. That is a special case. The general form is a **constrained
optimization problem (COP)** whose feasible region and objective are *read off the chart*.

> Invariant, unchanged: decision **support**, never replacement. The optimizer proposes a
> regimen with a full audit trail (every constraint cites the chart fact and the
> byte-provenanced clinical rule that produced it); the physician decides and can edit any
> constraint. The optimizer must **abstain** (return INDETERMINATE / INFEASIBLE with the
> conflicting constraints named) rather than fabricate a regimen.

---

## 1. The shape: chart → constraints → COP → solver

```
patient chart (FHIR / prose / voice)
   │  decompose (local model)  +  PHI de-identification   [CH task]
   ▼
typed IR: a set of CHART FACTS (conditions, labs, meds, allergies, vitals, coverage)
   │  constraint-compiler (deterministic): each chart fact × grounded clinical rule
   ▼
a CONSTRAINT PROGRAM over decision variables:
     variables : x_d ∈ {0,1}  (give drug d),  dose_d ∈ ℤ (mg/kg),  t ∈ {treat_now, await_culture}
     hard      : coverage, exclusions, dose-feasibility, step-therapy precedence, renal/hepatic caps
     objective : minimize  w_cost·cost + w_tox·side_effect_risk + w_delay·delay_risk + w_mon·monitoring
   │  adj-constraint-solver  (B1 int-opt · B1b SAT/PB · B2c LIA feasibility · C2 simplex min/max)
   ▼
DifferentialRegimen: chosen drugs + doses + timing, OR INFEASIBLE(conflict set)
   + proof: every constraint → the chart fact + the byte-provenanced rule that justifies it
```

Everything below already exists in the substrate and is **reused, not rebuilt**:
`adj-constraint-solver` (integer optimization, Sinz at-most-k SAT/PB, Cooper LIA
feasibility/`check`, simplex `minimize`/`maximize`, observed-value substitution),
`native_setcover` (coverage + exclusions + defeasance), the dose-window solve in
`derive_regimen.py`, the logic engine, and adj-lang `constrain`/`solve`/`check`/`minimize`.

---

## 2. The constraint taxonomy (each chart fact → a constraint family)

| Chart fact | Constraint family | Form | Example |
|---|---|---|---|
| **Likely organisms** (from the differential) | coverage | hard set-cover: ⋁ drugs covering each organism | every organism in the differential must be covered |
| **Allergy** (penicillin) | exclusion | hard: `x_d = 0` | β-lactams excluded |
| **Pregnancy** | exclusion | hard: `x_d = 0` | moxifloxacin, TMP-SMX excluded |
| **Renal impairment** (eGFR/CrCl lab) | dose cap | `dose_d ≤ ceiling_d(renal)` | vancomycin ceiling shrinks; renally-cleared drugs capped |
| **Hepatic impairment** | dose cap (conjunctive) | `dose_d ≤ ceiling_d(hepatic ∧ renal)` | ceftriaxone capped only when hepatic **and** renal impairment co-occur (FDA label); hepatic alone needs no adjustment (§3a) |
| **Concurrent meds** | drug–drug interaction | exclusion or dose cap | other nephrotoxin → vancomycin ceiling ↓ (additive toxicity) |
| **Comorbidity** (QT, G6PD, seizure hx, myasthenia) | exclusion / penalty | hard or soft | avoid QT-prolonging agents; avoid seizure-threshold-lowering |
| **Dose-window** (efficacy ↔ toxicity) | feasibility band | `floor_d ≤ dose_d ≤ ceiling_d` | UNSAT when no safe-and-effective dose exists |
| **Severity / time-criticality** | timing | hard or soft on `t` | septic/comatose ⇒ `t = treat_now` (empiric) |
| **Culture pending** | timing tradeoff | soft: narrow after result | await vs treat-now is a modeled decision (§4) |
| **Insurance / formulary** | step-therapy precedence + cost | hard precedence + objective | payer requires X tried before Y; tier → cost weight |
| **Side-effect profile** | objective penalty | soft | each drug carries a toxicity weight scaled by patient factors |
| **Cost** (drug + monitoring + LOS) | objective | minimize | the primary objective term |

**Hard** constraints define feasibility (a regimen that violates one is not offered).
**Soft** constraints become weighted terms in the objective (tradeoffs the solver
optimizes). The split is itself a grounded, editable choice in the CAS.

---

## 3. Nothing authored — the constraint *rules* are spider-grounded too

The governing project rule applies recursively: the clinical rules that turn a chart fact
into a constraint must themselves enter the CAS via the cold path (spider → byte-quote →
adversarial gate → citation verified), exactly like the organism-id LRs and the doses.
Examples that must be grounded, not authored:

- "moxifloxacin / fluoroquinolones contraindicated in pregnancy" → byte-quote from the
  FDA label / ACOG.
- "vancomycin nephrotoxicity is additive with other nephrotoxins" → primary source.
- "ceftriaxone contraindicated in neonates receiving IV calcium" → FDA label (already
  surfaced verbatim in the G3 source decomposition).
- step-therapy / prior-authorization rules → the payer's published policy document.

So Chart-as-Constraints reuses the **same grounding harness** (`grounding/harness.py`,
`ground_sources.py`): a new grounding file per constraint family
(`interaction-grounding.json`, `contraindication-grounding.json`,
`step-therapy-grounding.json`), a gate that emits the constraint rules into the CAS, and a
new ledger artifact ("treatment constraints") with its own grounded/flagged/debt counts.

---

## 4. The wait-vs-treat-now decision, modeled

Empiric-now vs await-culture is a real tradeoff the directive calls out. Model it as a
binary `t` with two costed branches:

- **treat_now (empiric):** broad coverage (more drugs) → higher cost + higher cumulative
  side-effect risk, but `delay_risk = 0`.
- **await_culture (targeted):** narrower (cheaper, fewer side effects) but
  `delay_risk = severity × P(progression in the culture window)`.

A hard guard forces `t = treat_now` when severity/time-criticality crosses a grounded
threshold (e.g. suspected bacterial meningitis — every hour of delay raises mortality, a
byte-provenanced fact). Otherwise the objective decides. The output names the tradeoff
explicitly ("empiric now costs $X and N side-effect-units; awaiting culture saves that but
risks Y") — decision support, not a hidden choice.

---

## 5. Insurance / step-therapy as constraints

- **Step therapy** ("won't approve Y until X tried/failed"): a precedence constraint
  `x_Y ≤ tried_X` where `tried_X` is a chart fact (prior failed regimen). If `X` hasn't
  been tried, `Y` is infeasible *for reimbursement* — surfaced distinctly from clinical
  infeasibility, because the physician may override on medical necessity.
- **Formulary tier / prior-auth**: tier → the `cost` weight; prior-auth-required → a soft
  penalty (delay/admin burden) or a hard gate the physician can override.
- The optimizer returns **two** regimens when they differ: the *clinically optimal* and
  the *insurance-feasible* — so the tradeoff (and any appeal) is explicit.

---

## 6. Output: a faithful, conflict-aware regimen

- **Feasible:** the chosen `{drug, dose, timing}` + the objective breakdown (cost / tox /
  delay / monitoring) + per-constraint provenance (chart fact → grounded rule → byte-quote).
- **Infeasible:** the **minimal conflict set** (reuse the IIS / conflicting-constraint core
  from B2c) — e.g. "covering Pseudomonas requires cefepime, but cefepime is excluded by the
  documented allergy" — so the physician sees exactly which two facts collide.
- Every number is editable in the CAS; editing a constraint re-derives at **0 model calls**
  (warm path), and the change propagates with a new audit trail.

---

## 7. Build plan (incremental PRs, specs-first, each grounded + babysat)

- **CC-1 — constraint IR + compiler skeleton.** Define the chart-fact → constraint mapping
  as data (`constraints/` schema) and a deterministic compiler `chart_to_cop.py` that emits
  an adj-lang constraint program from a chart IR + the grounded rule tables. Reuse
  `native_setcover` for coverage; emit `constrain`/`exclude`/dose-band clauses. Tests on the
  existing meningitis profiles (young/elderly/allergic) reproducing today's set-cover output
  as a special case — proving generalization didn't regress.
- **CC-2 — dose feasibility + renal/interaction caps as constraints.** Fold the dose-window
  solve into the COP (`floor ≤ dose ≤ ceiling(renal, interactions)`); UNSAT path returns the
  conflict. Ground the renal-adjustment + additive-nephrotoxicity rules (spider).
  - **CC-2c — the vancomycin renal + nephrotoxin penalty DIRECTIONS are grounded. ✅ DONE.**
    The renal and additive-nephrotoxicity ceiling penalties previously had no grounding record
    (only the trough target `dose_vancomycin` did). The FDA vancomycin label now backs their
    DIRECTION: *"Vancomycin should be used with caution in patients with renal insufficiency
    because the risk of toxicity is appreciably increased by high, prolonged blood
    concentrations. Dosage of vancomycin hydrochloride for injection must be adjusted for
    patients with renal dysfunction."* (WARNINGS) and *"In order to minimize the risk of
    nephrotoxicity when treating patients with underlying renal dysfunction or patients
    receiving concomitant therapy with an aminoglycoside, … particular care should be taken in
    following appropriate dosing schedules."* (PRECAUTIONS) — DailyMed setid
    `75a6a873-21b9-4f48-89a9-c1476294c0ce`, records `dose_penalty_vancomycin_renal` /
    `dose_penalty_vancomycin_nephrotoxin` in `dose-window-grounding.json` (`spider_status:
    direction_only`). The label grounds that renal dysfunction and concurrent nephrotoxins
    REQUIRE a downward dose adjustment (the safe ceiling shrinks), which is exactly the
    direction the `renal_severe`/`renal_moderate`/`nephrotoxin_interaction` ceiling penalties
    encode. As with every dose number, only the DIRECTION is grounded; the per-kg penalty
    magnitudes in `formulary.json` stay the standing **ILLUSTRATIVE** feasibility model (the
    label gives no closed-form per-kg reduction). Pure ledger/provenance increment — no engine
    or compiler logic changed, so the regimen output is byte-identical.
- **REFACTOR (ADJ-first): every exclusion is now an EXPLICIT engine constraint.** Dose-infeasible
  (CC-2), contraindicated (CC-3), and step-therapy (CC-6) drugs are unified into one
  `forced_zero: {drug → reason}` and emitted as `constrain x_d <= 0   % excluded (reason)` in the
  adj-lang program — not pre-removed from the candidate list in Python. So the emitted program is
  self-documenting about *why* each drug is out, the infeasibility verdict is the engine's, and
  the exclusion reasoning lives in ADJ, not the compiler. (Reasons are sanitized before reaching
  the `%` comment.)
- **CC-2b — hepatic impairment is a CONJUNCTIVE dose cap, not a standalone one. ✅ DONE.** The
  ceftriaxone FDA label conditions its 2 g/day ceiling on the *joint* presence of two organ
  impairments: *"Patients with hepatic impairment and significant renal impairment should not
  receive more than 2 grams per day of ceftriaxone."* (DailyMed setid
  `5cd2d96f-83e5-4326-ae87-d0ede4ba493a`, §5.7 / USE IN SPECIFIC POPULATIONS; grounded as record
  `dose_cap_ceftriaxone_hepatorenal` in `dose-window-grounding.json`). Faithfully modeling this
  means **not** treating hepatic impairment as its own dose cap: a `hepatic_status` chart fact
  alone adds an audit-only risk marker but **no** dose penalty (the same label says hepatic
  impairment alone needs no adjustment). Only when a `hepatic_*` risk and a `renal_*` risk
  co-occur is a derived **`hepatorenal`** risk synthesized, which then applies ceftriaxone's
  `ceiling_penalty_mg_per_kg.hepatorenal` reduction (50 → 38 mg/kg in the feasibility model —
  still ≥ the 25 mg/kg floor, so the regimen stays FEASIBLE but dose-adjusted, mirroring "cap,
  don't forbid"). This is the first constraint family whose *trigger* is a conjunction of two
  chart facts rather than a single fact. As with every dose number in `formulary.json`, the
  **mechanism** (conjunction → shrink the ceiling, on ceftriaxone) is grounded; the precise mg/kg
  shrink is the standing **ILLUSTRATIVE** feasibility model (the label's cap is an absolute
  2 g/day), not validated PK/PD.
  - **ADJ-NATIVE (the conjunction is the ENGINE'S, not Python's). ✅ DONE.** The conjunction
    first landed as a Python post-loop block in `chart_to_cop.py`
    (`if has_hepatic and has_renal: cop.risks.add("hepatorenal")`) — the same "Python rule layer
    in the middle" CC-3 removed. It is now an ADJ rulebook, `treatment/antibiotics/dose_caps.adj`,
    **generated** by `dose_caps_build.py` from `dose-window-grounding.json`: definitional
    `risk_in_category` facts (`hepatic_severe`/`hepatic_moderate` ∈ hepatic; `renal_severe`/
    `renal_moderate` ∈ renal), the compound's two component categories
    (`compound_first(hepatorenal, hepatic)`, `compound_second(hepatorenal, renal)`), the
    **grounded** `dose_capped_under(ceftriaxone, hepatorenal)` fact (carrying the FDA byte-quote
    + DailyMed locator at `trust authoritative`), and **two generic conjunction rules**
    (`derived_risk($C) when compound_first($C,$A), risk_in_category($Ra,$A), active_risk($Ra),
    compound_second($C,$B), risk_in_category($Rb,$B), active_risk($Rb)` + the analogous
    `dose_capped($D,$C)`). `chart_to_cop` now only asserts the patient's raw `active_risk` tokens;
    `derive()` asks the engine `? derived_risk` / `? dose_capped`
    (`dose_caps.derive_dose_caps`, 0 model calls) and folds the derived compound risk into the
    COP before the dose-window solve. **The conjunction reasoning moved out of Python and into the
    language.** Because the rule keys on *categories*, ANY two-factor compound risk (and any drug
    capped under it) is added as a grounded row, with no new Python branch — the same domain-neutral
    substrate as `contraindications.adj`. A single active risk derives nothing (engine-verified),
    so hepatic-alone still changes nothing. Verified by `test_dose_caps.py` (generator `--check`,
    injection guard, both-categories-required, grounded-quote-flows-through) and
    `test_hepatic_renal_conjunction_caps_ceftriaxone` (now asserting `compile_cop` is pure and the
    `hepatorenal` risk + `derived_risk(hepatorenal)` constraint are engine-derived in `derive()`);
    the board's `mgmt_hepatorenal` item exercises the full engine path end-to-end (0 wrong).
- **CC-3 — contraindication / interaction grounding.** `contraindication-grounding.json` +
  `interaction-grounding.json` via the harness (pregnancy, QT, G6PD, allergy classes,
  drug–drug); gate → CAS; new "treatment constraints" ledger artifact.
- **CC-3 REFACTOR (ADJ-native): contraindications are now DERIVED BY THE ENGINE, not a
  Python set. ✅ DONE.** The exclusion knowledge no longer lives in a `chart_to_cop.py` dict
  (`_PREGNANCY_CONTRAINDICATED = {"moxifloxacin", "tmp_smx"}`) — that was exactly the
  "Python rule layer in the middle" we are removing. It is now an ADJ rulebook,
  `treatment/antibiotics/contraindications.adj`, **generated** by `contraindications_build.py`
  from `grounding/treatment-constraints-grounding.json`: grounded `relate` facts
  (`class_contraindicated_in(fluoroquinolone, pregnancy)` etc., each carrying its FDA
  byte-quote + DailyMed locator at `trust authoritative`) plus **two generic, context-scoped
  `rule { head: contraindicated($D,$C) when: active_context($C), … }` clauses** (the rulebook
  keystone). `chart_to_cop` now only translates a chart fact into the patient's active
  *context* (`pregnancy=present → active_context "pregnancy"`); `derive()` asks the engine
  `? contraindicated($D,$C)` (`contraindications.derive_contraindications`, 0 model calls)
  and folds the derived drugs into `forced_zero`. **The reasoning moved out of Python and into
  the language.** A contraindication is McCarthy's `ist(c, φ)` — "drug excluded IN context C"
  — so the *same* generic shape encodes any context-scoped rule corpus (e.g. "a term MEANS m
  in jurisdiction J"); we deliberately use the domain-neutral word **context**. Demonstrated
  by the QT-prolongation fact: the identical class rule excludes the fluoroquinolone in a
  different context, with no new Python branch. Adding a new contraindication is now a
  grounded fact in the rulebook + (if a new context) one row in `_CONTEXT_FROM_FACT` — no
  drug-name logic in the compiler. Remaining CC-3 follow-ups (allergy drug-class exclusion,
  drug–drug interactions) move into the same rulebook next.
- **CC-3b — β-lactam allergy is SIDE-CHAIN-scoped, not class-wide (grounded; ⚠️ CHANGES
  BEHAVIOR — review).** The current `_ALLERGY_EXCLUSION` drops the *entire* β-lactam class on a
  penicillin allergy. The literature (spider-grounded 2026-06-16, record
  `ci_betalactam_sidechain_mechanism` in `treatment-constraints-grounding.json`) says that is
  wrong: *"The similarity in structure of the R1-side-chains of penicillins and cephalosporins
  determines the likelihood of cross-sensitivity between the drug classes — not the presence of
  the beta-lactam ring."* Cross-reaction risk to cephalosporins in reported (untested)
  penicillin allergy is **<1%** (~2% with a positive skin test); 3rd-gen cephalosporins
  (ceftriaxone, cefepime), carbapenems, and aztreonam are given to penicillin-allergic patients
  **without testing** under 2024 drug-allergy practice parameters.
  - **Corrected model (the right thing from the literature):** a penicillin allergy excludes
    penicillins **+ only the cephalosporins/agents that SHARE the culprit's R1 side chain** — NOT
    structurally-dissimilar β-lactams. The cross-reactivity figures are **typed quantities**
    (`percentage`, via the typed-value pipeline), grounded, and used directly (e.g. "<1%" is a
    `percentage` literal in the ADJ program, not prose).
  - **✅ DONE (implemented; ⚠️ this CHANGED clinical behaviour — physician-auditor please verify).**
    The Python `_ALLERGY_EXCLUSION` map + the blanket `betalactam_allergy_severe` token are
    **retired**. An allergy now activates a CONTEXT and the engine derives the exclusions from
    the grounded contraindication rulebook, scoped by pharmacologic **subclass**:
    - `contraindications.adj` gains `has_class` facts for the β-lactam subclasses (ampicillin
      ∈ penicillin; ceftriaxone, cefepime ∈ cephalosporin; meropenem ∈ carbapenem; aztreonam ∈
      monobactam) + DEFINITIONAL `class_contraindicated_in` edges: `penicillin_allergy` →
      penicillins; `cephalosporin_allergy` → cephalosporins; `betalactam_allergy` (unspecified
      whole-class) → penicillins + cephalosporins + carbapenems (**not** monobactams). The
      grounded literature justifies the **absences** (no rule emitted for cephalosporins under
      `penicillin_allergy`, so they stay available).
    - `chart_to_cop._CONTEXT_FROM_FACT` maps `allergy=penicillin → penicillin_allergy`, etc.;
      `derive()` folds the engine-derived exclusions into `forced_zero`.
    - **Resulting behaviour change (verified by tests):** a penicillin allergy (even
      anaphylactic) is now **FEASIBLE** — `vancomycin + ceftriaxone` (3rd-gen, <1% cross-
      reactivity) instead of the old blanket abstention. An **unspecified** whole-class
      β-lactam allergy still abstains (only aztreonam survives, which can't cover *S.
      pneumoniae*) — honest INFEASIBLE preserved.
    - **Still authored-debt / follow-up:** true R1-side-chain *identity* per drug (so a
      specific aminopenicillin allergy can flag the few same-side-chain cephalosporins, and a
      severity dimension can gate anaphylaxis caution). The current cut is subclass-level,
      which is correct for the formulary's drugs (no formulary cephalosporin shares a penicillin
      side chain) but should refine to side-chain identity as the formulary grows.
- **CC-4 — cost + side-effect objective. ✅ DONE.** The set-cover objective is now the
  weighted blend `minimize Σ (w_cost·tier + w_tox·side_effects)·x_d`, emitted to the engine's
  integer optimizer (coefficients stay integer). A chart `objective_priority` fact selects the
  `(w_cost, w_tox)` weights (`cost`=(1,0), `balanced`=(1,1), `low_toxicity`=(1,3)); the default
  (1,0) reproduces the historical tier-only set-cover exactly, so every prior consumer is
  unchanged. `derive()` surfaces the per-component objective breakdown (cost / side_effects /
  total), and the engine agrees with the Python weighted set-cover under every blend (verified
  in `test_native_setcover`: the regimen flips cefepime→aztreonam for pseudomonas as w_tox
  rises). Drug **cost** uses the grounded preference `tier`; the **side-effect** weights are an
  authored-debt layer (`formulary.json` `side_effects` map, flagged) pending **CC-4b** spider
  grounding (FDA-label adverse-event / monitoring burden) — the standard domain→ground flywheel.
- **CC-5 — wait-vs-treat-now decision (§4). ✅ DONE.** `decide_timing(disease, culture_status,
  clinical_status)` models the empiric-now vs await-culture choice as a function of the disease's
  TIME-CRITICALITY (grounded threshold) and the patient's culture/clinical status: a time-critical
  disease (meningitis, door-to-antibiotic ≤60 min) or an unstable patient → `treat_now_empiric`
  (delay_risk high); stable + routine-acuity + culture pending → `await_culture` (delay_risk low,
  cheaper/narrower); culture resulted → `targeted_culture_directed`. New chart facts
  `culture_status` (pending/resulted) and `clinical_status` (critical/unstable/stable) feed it;
  `derive()` surfaces the decision + delay_risk + rationale + threshold. The decision is reusable
  (not meningitis-specific). The ≤60-min meningitis threshold is authored-debt (IDSA), flagged for
  **CC-5b** spider grounding.
- **CC-5 REFACTOR (ADJ-native): the timing DECISION is now a defeasible-precedence ladder the
  engine resolves, not a Python if/elif. ✅ DONE.** The wait-vs-treat logic moved out of
  `decide_timing` into `timing.adj` — a `functional timing(_)` predicate + four `priority:`-tiered
  rules (ADJ73): resulted-culture → targeted (`mandatory`); time-critical/unstable → treat-now
  (`authoritative`); stable+routine+pending → await (`specific`); else → treat-now (`default`).
  `timing.derive_timing(cli, culture, clinical, acuity)` asserts the per-case facts, runs the
  engine, and reads the **governing** answer (adj-lang-cli's `governing` section); `delay_risk`
  is read off the governing **tier** (treat-now@authoritative = high vs @default = moderate). The
  Python if/elif is retired — the reasoning lives in the language; the engine derives it (0 model
  calls) and the proof shows which rule governed + what it defeated. `decide_timing` is now a thin
  wrapper supplying the disease's acuity (from the flagged `_TIME_CRITICALITY` input table) + the
  threshold/rationale presentation. (The disease→acuity table remains authored-debt — input data,
  not decision logic.)
- **CC-6 — insurance / step-therapy (§5). ✅ DONE.** A payer step-therapy rule ("won't
  approve Y until X tried") enters as a `step_therapy` chart fact (`restricted:prerequisite`)
  + `prior_failed` facts (drugs already tried); the precedence `x_Y ≤ tried_X` is emitted as an
  EXPLICIT engine constraint `constrain x_Y <= 0` in the reimbursement program (the known-untried
  `tried_X = 0` folded in) — enforced by the constraint solver and auditable in the program, NOT
  pre-filtered in Python. `derive()` solves TWICE: `regimen`
  is the clinically optimal one; `reimbursement` carries the payer-covered regimen, the
  `blocked` drugs, whether it `differs_from_clinical`, and a note. Reimbursement infeasibility
  (a rule blocking a clinically-forced drug → covered = INFEASIBLE) is surfaced **distinctly**
  from clinical infeasibility → physician override / appeal on medical necessity. Grounding a
  real published payer policy is the **CC-6b** follow-up.
- **CC-6 REFACTOR (ADJ-native): the step-therapy precedence is now an ENGINE RULE, not a
  Python set-difference. ✅ DONE.** The blocked-drug derivation left `chart_to_cop.py`
  (`reimbursement_blocked()` is deleted) and became `step_therapy.adj` — a durable,
  domain-neutral **negation-as-failure** rule: `reimbursement_blocked($Y) when:
  requires_prerequisite($Y,$X), not already_tried($X)`. The per-case payer facts
  (`requires_prerequisite` / `already_tried`) are asserted from the chart at query time;
  `derive()` calls `step_therapy.derive_blocked(cli, …)` which runs the engine
  (`? reimbursement_blocked($Y)`, 0 model calls) and folds the blocked drugs into the
  reimbursement program's `forced_zero`. NAF is the natural encoding of "blocked unless the
  step is satisfied"; the precedence reasoning lives in the language. The same `requires… ∧
  not done…` shape is reusable across any rule corpus (a filing gated on a precondition, a
  benefit gated on a prior step).
- **CC-7 — full chart drive-through. ✅ DONE.** `fhir/run_chart_to_regimen.py` joins the three
  stages into one call — `chart_to_regimen(cli, bundle, disease, as_of_year)` runs
  *deidentify → to_chartfacts → chart_to_cop.derive* and returns the treatment decision PLUS the
  full audit trail (Safe-Harbor de-id report, mapped ChartFacts, per-resource discards, grounded
  constraints, wait-vs-treat + reimbursement). `answer_time_model_calls == 0` — a whole
  de-identified FHIR chart → a regimen (or honest INFEASIBLE + conflict) entirely on the CPU.
  Two committed fixtures + `fhir/test_run_chart_to_regimen.py`: a straightforward adult →
  vancomycin + ceftriaxone, and the complex PHI chart → de-identified → abstention. This closes
  CC-2..7; the chart-as-constraints arc is end-to-end.

Each CC-n: spec note → grounded rules (no authoring) → deterministic compiler/solver wiring
→ tests (incl. an infeasibility/abstention test) → security review → PR → babysit.

---

## 8. Why this is the right shape

- It is the **generic engine** position (no domain-specific point solution): treatment is
  constrained optimization; the solver already exists; the chart supplies the constraints.
- It makes the chart **fully accounted for** — the same "no unaccounted bytes" discipline as
  diagnosis: every chart fact must land as a constraint or be explicitly discarded with a
  reason, so nothing in the chart is silently ignored.
- It keeps inference **CPU-bound and correctable**: the model only decomposes the chart; the
  solver reasons; editing a grounded rule re-derives deterministically.
- It preserves **abstention**: INFEASIBLE with a named conflict set is a first-class,
  honest answer — the optimizer never invents a regimen to avoid saying "these facts collide."
