# ADJ86 — Defensibility benchmark (100 samples × 10 domains): pre-registration

**Status: DRAFT for sign-off. Nothing run yet.** This fixes the design before any data is
collected, per the ADJ73/ADJ84 pre-registration discipline.

## Claim under test

> *Byte-provenance makes work **defensible** — and defensibility is a property of the
> verification discipline, not of the domain or of getting the answer right.* Across 100
> questions spanning 10 domains, the framework arm produces work whose every claim is
> traceable to a source/rule/computation (auditable), while the bare arm produces correct-
> *or*-incorrect but **un-auditable** assertions. We do **not** claim correctness parity
> (that is FrugalGPT's lane); we claim **defensibility** improvement, with the honest
> boundary that any accuracy gap is absorbed by **abstention, not fabrication**.

## Design — the REAL framework (ADJ84 pipeline), not a single prompt

Each item is an **adjudication problem**: a `scenario` (facts) + a `policy` (rules) +
a determination question. This is the shape the real engine adjudicates, generalised
across domains (clinical-coverage, statutory liability, tax/duty, eligibility, etc.).

- **N = 100**, stratified two ways: **10 domains × 10**, AND across **4 difficulty strata**
  (the ADJ84 strata): `clean-determinate`, `underdetermined-baited` (the dispositive fact is
  withheld; bare is tempted to fabricate it), `override-precedence` (an exception rule must
  dominate), `exception-encoding` (an "except…" suppressing override).
- **Arms (within-item, paired), one fixed model:**
  - **BARE** — one-shot: model reads scenario+policy and states the determination in prose
    (no framework).
  - **FRAMEWORK = the real ADJ84 pipeline** — the model does ONLY two extraction stages:
    **Stage A** policy → rulebook-IR (`rules[]` with `when`/`then`/`source_span`),
    **Stage B** scenario → input-IR (typed `slots`, each `stated|inferred` with a verbatim
    byte span, + uncertainties). Then the **deterministic `engine.py`** (reused unchanged)
    verifies every stated slot's span is verbatim (byte-accounting), evaluates the rules, and
    **owns the verdict** — returning `INDETERMINATE` *structurally* when a dispositive slot is
    missing, `CONFLICT` when satisfied rules disagree, `DETERMINATE(answer)` otherwise. Every
    intermediate artifact (IR, rulebook, proof) is recorded.
- **Defensibility, per arm:**
  - **FRAMEWORK** — defensible iff **byte-accounting is clean** (no hallucinated slot: every
    stated slot's span is verbatim) AND the verdict is the engine's structural output (the
    proof = which rules fired on which spans). Machine-checked; near-1 by construction unless
    the model fabricates a slot (caught deterministically).
  - **BARE** — a **blind adversarial auditor** enumerates every claim in the prose and marks
    it verifiable (cites a scenario/policy span) or unsupported → defensibility fraction.
- The auditor never sees which arm is which or the gold key; correctness is scored separately
  against the held-aside gold verdict/answer.

## Primary + secondary metrics (per item, per arm)

- **PRIMARY — defensibility fraction** = verifiable claims / total claims. (ADJ68 metric.)
- Binary **DEFENSIBLE** verdict (fraction == 1.0).
- **Citation-fabrication count** — cited spans not present in the cited source. *Pre-registered
  prediction: exactly 0 in FRAMEWORK by construction (deterministic byte-anchor).*
- **Accuracy** vs a held-aside ground truth, where the question has a determinable answer.
- **Abstention/underdetermination rate** (FRAMEWORK emits UNDERDETERMINED / named hole).

## Pre-registered hypotheses

- **H1 (primary):** mean defensibility fraction FRAMEWORK ≫ BARE (paired), large effect.
- **H2:** citation-fabrication == 0 in FRAMEWORK; > 0 in BARE.
- **H3 (honest boundary):** accuracy(FRAMEWORK) is **not** uniformly higher than BARE; where
  FRAMEWORK lacks the knowledge it **abstains** (UNDERDETERMINED) rather than fabricating, so
  BARE's errors are *confident-wrong* while FRAMEWORK's are *flagged-uncertain*. We expect a
  residual accuracy/coverage gap; the defensibility gap closes, the accuracy gap does not.
- **H4 (domain-independence):** the defensibility improvement holds in **all 10** domains
  (not driven by 1–2). This is the "property of the discipline, not the domain" claim.

## Analysis (also pre-registered)

- Mean defensibility fraction per arm, overall + per domain, with **bootstrap 95% CIs**.
- Paired test (Wilcoxon signed-rank) on defensibility fraction.
- Per-domain table; H4 holds iff FRAMEWORK > BARE in every domain.
- Report accuracy + abstention alongside (H3); the **headline is the conjunction**
  (defensibility ↑, fabrication = 0, accuracy honest), never defensibility alone.

## Corpus construction (to avoid contamination + leakage)

- Questions authored to have a **determinable or explicitly-contested** answer with a
  citable basis; mix of (a) answerable-from-public-sources and (b) genuinely-contested
  (no-single-answer, like the Jason item) so defensibility ≠ correctness is testable.
- Ground truth + sources held in a separate file the arms never see; the auditor is given
  the answer text only, not which arm or the ground truth (it judges *verifiability*, not
  correctness — correctness is scored separately against the held-aside key).
- **Auditor validation:** a human/second-model spot-check of ≥10% of audit verdicts for
  inter-rater agreement (the auditor is an LLM — Jain&Wallace-style caveat applies; we
  report agreement, not treat the auditor as ground truth).

## Staging + cost control

1. **Pilot: N = 12 (2 domains × 6)** — validate the harness, the auditor, the metric, and
   measure **actual per-sample token cost**. ~36 agent runs.
2. **Report pilot + extrapolated cost for the full 100**; get budget sign-off.
3. **Full run: N = 100**, executed as a deterministic pipeline (one orchestrated workflow,
   pre-registered script), with results + CIs + the per-domain table.

## Open decisions for sign-off

- [ ] Domains list above OK, or adjust?
- [ ] Single fixed model for both arms (recommended — isolates BARE vs FRAMEWORK), or also
      vary model scale {Haiku, Opus} to fold in the ADJ84 defensibility-parity axis (2×)?
- [ ] Pilot N=12 first (recommended) before committing the full-100 spend?
