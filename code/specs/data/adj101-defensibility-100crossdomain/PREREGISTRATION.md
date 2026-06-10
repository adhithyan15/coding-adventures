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
