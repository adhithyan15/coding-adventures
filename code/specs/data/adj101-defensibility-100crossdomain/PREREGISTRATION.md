# ADJ101 — 100-item cross-domain defensibility benchmark: pre-registration

**Status: DRAFT — design locked before data, per ADJ73/84/86 discipline.** This is the full-scale
execution of the ADJ86 design (which only ran as a 12-item pilot), upgraded with the **corrected
defensibility metric** validated by the ADJ99 rescore (PR #5261) and its W5 Sonnet cross-judge
replication. It is the paper-1 **E3 breadth proof** and the on-ramp to paper-2 MYCIN (auto-derived
rulebooks). Specs: [`PAPER1-E3-domain-expansion.md`](../../papers/PAPER1-E3-domain-expansion.md),
[`MEASUREMENT-VALIDITY-AUDIT.md`](../../papers/MEASUREMENT-VALIDITY-AUDIT.md),
[`PAPER1-methods-protocol.md`](../../papers/PAPER1-methods-protocol.md).

## Claim under test

> *Byte-provenance makes adjudicative work **defensible** — and defensibility is a property of the
> verification discipline, not of the domain or of getting the answer right.* Across **100 items in
> 10 domains**, the framework arm exposes the **locus of contingency** (every load-bearing premise
> surfaced and flagged fallible; the verdict owned by a deterministic engine over byte-anchored
> facts), while the bare arm produces confident-but-un-auditable assertions. We do **not** claim
> correctness parity (FrugalGPT's lane); we claim a **defensibility** improvement that holds **across
> domains and across model scale**, with the honest boundary that any accuracy gap is absorbed by
> **abstention, not fabrication**.

## What changed vs the ADJ86 prereg (and why)

The ADJ86 pilot's blind judge was **format-confounded** (the same failure ADJ99 proved): the framework
arm carried structural tells (`GROUNDED/ENTAILED/ASSUMED`, `INDETERMINATE` verdict) the judge could
read the arm off, and the old rubric scored citation/traceability, not locus-exposure. Worse, the
ADJ86 judge **penalized the framework's `ASSUMED` flags as "manufactured doubt"** — which under the
corrected rubric is exactly the fallibility-flagging we reward. So this run replaces ADJ86's scoring
with the **corrected, format-normalized, dual-judge** metric. Everything else (the real pipeline, the
strata, the machine-checked byte-accounting) is retained.

## Design

Each item is an **adjudication problem**: `scenario` (facts) + `policy` (rules) + a determination
question — the shape the engine adjudicates, generalised across domains.

- **N = 100**, stratified two ways: **10 domains × 10 items**, AND across **4 difficulty strata**
  (2.5 items/stratum/domain): `clean-determinate`, `underdetermined-baited` (dispositive fact
  withheld; bare is tempted to fabricate it), `override-precedence` (an exception rule must dominate),
  `exception-encoding` (an "except…" suppressing an override).
- **Domains (10):** medicine (clinical-coverage), statutory liability, tax/duty, benefits eligibility,
  insurance claims, building-code compliance, employment/labor rules, immigration eligibility,
  contract/SLA terms, academic/grant eligibility. (Spans rule-dense↔precedent-dense and
  quantitative↔qualitative per the [E3 axes](../../papers/PAPER1-E3-domain-expansion.md).)
- **Model scale (RESOLVED — both):** run **{Haiku, Opus}** for both arms (the ADJ84/ADJ86
  defensibility-parity axis). 2× cost, but it tests the central "lifts the *weak* model to the strong
  model's defensibility" claim, which a single model cannot.

### Arms (within-item, paired)
- **BARE** — one-shot: model reads scenario+policy, states the determination in prose.
- **FRAMEWORK = the real pipeline** — the model does ONLY two extraction stages:
  **Stage A** policy → **auto-derived** rulebook-IR (`rules[]` with `when/then/source_span`),
  **Stage B** scenario → input-IR (typed `slots`, each `stated|inferred` with a verbatim byte span +
  a `basis_span` + entailment label, + uncertainties). Then the **deterministic `engine.py`** verifies
  every stated span is verbatim (byte-accounting), evaluates rules, and **owns the verdict**
  (`DETERMINATE` / `INDETERMINATE`-structural / `CONFLICT`). Stage A is the **automatic rulebook
  derivation** — the same mechanism MYCIN-2026 needs for its CAS rules.

## Metrics (per item, per arm, per model)

1. **PRIMARY — corrected defensibility (0–5 locus-exposure), format-normalized, dual-judge.**
   Both arms rendered into one style-neutral `REASONING/CONCLUSION` envelope (strip
   GROUNDED/ENTAILED/ASSUMED labels, verdict scaffolding, citation chrome) so the judge **cannot read
   the arm off format**. Scored by the ADJ99 rubric (is the load-bearing premise surfaced + flagged
   fallible + would-flip stated?). **Two judges (Opus + Sonnet)**; report inter-judge agreement; a
   second judge is decisive on close calls (W5 policy).
2. **Machine-checked byte-accounting** (framework only): citation-fabrication count. *Pre-registered
   prediction: 0 by construction* (deterministic byte-anchor); > 0 expected in BARE.
3. **Accuracy** vs held-aside gold verdict/answer (deterministic / style-invariant match; LLM accuracy
   judge only as flagged approximation, per the ADJ95 lesson).
4. **Abstention / underdetermination rate** (framework emits structural INDETERMINATE with the named
   missing slot).

## Pre-registered hypotheses

- **H1 (primary):** mean corrected-defensibility FRAMEWORK > BARE (paired), under **both** judges and
  at **both** model scales. The fw−bare gap survives format normalization (the ADJ99/W5 result
  predicts it *grows*, because the rubric now rewards the framework's fallibility flags).
- **H2:** citation-fabrication = 0 in FRAMEWORK; > 0 in BARE.
- **H3 (defensibility-parity):** FRAMEWORK closes the **Haiku↔Opus defensibility gap** toward ~0
  (the weak model, run through the engine, is as defensible as the strong one), while the
  **accuracy/coverage gap persists** and is absorbed by honest abstention — not fabrication.
- **H4 (domain-independence):** the defensibility improvement holds in **all 10** domains, not 1–2.
- **Honest nulls we must be able to report:** if format normalization erases the fw gap (it didn't for
  ADJ99), H1 fails; if FRAMEWORK fabricates a slot, H2 fails; if the weak-model lift doesn't
  materialize, H3 fails. The headline is the **conjunction** (defensibility ↑, fabrication = 0,
  accuracy honest), never defensibility alone.

## Analysis (pre-registered)
- Mean corrected-defensibility per arm × model, overall + **per domain**, with **bootstrap 95% CIs**.
- Paired Wilcoxon signed-rank on the fw−bare defensibility delta.
- Inter-judge agreement (exact / within-1 / r), per the W5 template.
- Per-domain table; H4 holds iff FRAMEWORK > BARE in every domain.
- Accuracy + abstention reported alongside (H3); the framework−bare defensibility **distribution
  across domains**, broken out by the E3 axes, is the headline figure.

## Corpus construction (contamination + leakage control)
- Items authored to have a **determinable or explicitly-contested** answer with a citable basis; the
  underdetermined-baited stratum **withholds the dispositive fact** (quality-gated: verify bare is
  actually tempted to fabricate it — the ADJ73 calibration lesson).
- Gold verdict/answer + sources held in a file the arms never see; the judge is blind to arm and gold.
- **Auditor/judge validation:** dual-judge agreement *is* the validation (W5); plus a ≥10% human/
  third-model spot-check, reporting agreement (LLM-judge caveat, not ground truth).

## Staging + cost control (pre-registered)
1. **Pilot slice: N = 20 (all 10 domains × 2, spanning strata)** — validate harness, the corrected
   dual-judge, format normalization, and **measure actual per-item token cost**.
2. **Report pilot + extrapolated full-100 cost**; proceed.
3. **Full run: N = 100**, one orchestrated pre-registered workflow, batched (≤10 concurrent per the
   rate-limit lesson), results + CIs + per-domain table.

## Addendum A — the IR is TYPED (extraction schema; non-negotiable)

Both arms' extraction emits a **typed** IR. Nothing untyped reaches the reasoner (engine or emitted
program) — an untyped fact cannot be computed over correctly, and an unpolarized fact silently flips
verdicts. Every fact/slot carries:

```jsonc
{
  "value": "...",                       // the typed value
  "type": "stated" | "inferred",
  "span": "verbatim source bytes",      // stated: provenance
  "basis_span": "...", "entailment": "ENTAILED" | "LEAP",   // inferred: justification (ADJ61 gate)
  "polarity": "affirmed" | "denied" | "inherit",            // ADJ03 — "not liable", "no allergy", "excluded"
  "quantity": { "magnitude": 1200, "unit": "USD" },          // ADJ21/22 — typed unit, NOT a bare number
  "modality": "categorical" | "conditional" | "uncertain",   // ADJ01
  "uncertainty": null | "..."
}
```

- **Polarity (ADJ03)** is mandatory: a denied condition (`not liable`, `no known drug allergy`, `coverage
  excluded`) is represented as `polarity:denied`, never dropped or affirmed. The polarity/modality
  consistency check runs over the IR before the engine fires.
- **Units (ADJ21/22)** are mandatory on every quantity: `{magnitude, unit}`. The program/engine does the
  unit algebra (km→m, h→s, °→rad, %→fraction); UNIT1 and PHYS1 are wrong without it. A coverage check
  ensures **every quantity in the source is typed-and-represented or explicitly discarded-with-reason**
  (ADJ02/ADJ22) — no number silently dropped.
- **Type + provenance**: `stated(span)` or `inferred(basis_span + entailment)` — no fact inferred
  without a justification (the ADJ61 entailment gate).

## Addendum B — program-emission track (the LLM translates; a PROGRAM reasons)

For **computational** items (`stratum: program-required` — math, physics, chemistry, units, finance;
`items_pilot_compute.json`), the model must **not** compute in its forward pass (LLMs are weak at
math/multi-step derivation — the ADJ99 "self-contained derivation drifted" failure). Instead:

1. **Translate** the messy source → the typed IR above (quantities with units + polarity + provenance).
2. **Emit a program** in a tool that does the actual work — **SymPy** (algebra/calculus/ODEs),
   **RDKit** (chemistry), NumPy/SciPy, or the repo's own `arithmetic`/`symbol-core`/`cas-matrix`/`stats`
   packages — that computes the answer **from the typed IR facts** (not from re-stated numbers).
3. **Execute** the program in a sandbox; the captured output is the answer.

**Byte-provenance is enforced on the program's inputs**, the same discipline as everywhere else:
- **Coverage** — every source quantity is either consumed by the program (traced to its `span`) or
  carries an explicit `discarded(reason)` (the PHYS1 "2 m wide" / FIN1 "branch 4021" distractors test
  this — a silently-dropped distractor is a coverage failure, *and* a distractor used as input is a
  fabrication-class error).
- **Justification** — every program input is `stated(span)` or `inferred(basis+ENTAILED)`; no input
  invented. The program literally cannot reference a value that isn't a typed, provenanced IR fact.

This is the **general form of the adjudication engine**: rules are the fixed program; SymPy/RDKit are
emitted programs; both consume the same typed, provenanced IR and own the answer. It is also the direct
mechanism behind paper-2 MYCIN (CPU-bound reasoning over derived rules).

### Metrics added for the program track — correctness is INFORMATIONAL, not the target

Where ADJ86 went wrong was scoring these on **getting the right answer**. In the rescored paradigm a
wrong program is *fine* if its audit trail leads to **exactly** where it went wrong and the error is
**correctable in one move**. So the program track is scored on the same axis as everything else —
auditability + localizability + correctability — and accuracy is reported only as context.

- **PRIMARY — auditability**: every program input is a typed, provenanced IR fact
  (`stated(span)` / `inferred(basis+ENTAILED)`); every source quantity is used or `discarded(reason)`;
  no magic numbers in the program. (`provenance_program.py` → `auditable`.)
- **PRIMARY — localizability**: when the answer is wrong, the trail names the **exact** culprit. The
  workhorse is the **value-vs-span faithfulness** check — a fact whose magnitude contradicts its own
  cited bytes (e.g. `25` claimed from span `"20 m/s"`) is flagged at that fact. `error_locus` orders
  the places to look: unfaithful facts → un-entailed assumptions → fabrications → exec errors.
- **PRIMARY — correctability**: a single **override** of the located fact re-derives the answer with
  **zero model calls** (`override_facts`; the override is itself audited). This is the program-track
  instance of E2 (localize→fix→persist) and MYCIN's *fix-the-fact-not-the-weight*.
- **SECONDARY / informational — accuracy**: executed output within `tolerance` of the tool-computed
  `gold_answer` (`compute_gold.py`). Reported, never the headline. A **defensible-but-wrong** program
  (auditable, error localized, one-move correctable) is a SUCCESS for the thesis; a
  **confidently-right black box** is not the goal.

### Hypotheses added (on the correctability axis)
- **H5 (auditable + correctable ≫ in-head):** on `program-required` items, FRAMEWORK (emit-program)
  beats BARE (reason-in-head) on **defensibility, localizability, and correctability** — largest exactly
  where ADJ99 showed in-head derivation drifts. When FRAMEWORK is *wrong*, the error localizes to a
  named fact/assumption and a single override fixes it; when BARE is wrong, it is a confident,
  un-localizable prose error.
- **H6 (program provenance clean):** FRAMEWORK program-input fabrication = 0, coverage complete
  (distractors `discarded(reason)`, never silently dropped or used), and **every wrong answer traces to
  a specific, overridable fact/assumption** — never "the model just miscalculated."

## Resolved decisions (ADJ86's open list)
- ✅ **Both models** {Haiku, Opus} (the defensibility-parity axis is core, not optional).
- ✅ **Corrected metric** (locus-exposure + format-normalized + dual-judge), not the format-confounded
  blind judge.
- ✅ **Pilot N=20 first**, then full 100.
- ✅ **Auto-derived rulebook** (Stage A) retained and foregrounded — it is the MYCIN bridge.

## Artifact layout
`code/specs/data/adj101-defensibility-100crossdomain/`: `items_100.json` (corpus, gold held separate
in the same file but never shown to arms), `pipeline.workflow.js` (the run), `engine.py` /
`provenance_engine.py` / `render_judge.py` (reused from ADJ86), `judge_*.workflow.js` (corrected
dual-judge), raw per-cell outputs, `aggregate.py`, `FINDINGS.md`. Every quoted number traces to an
artifact (byte-provenance applied to the paper itself).
