# ADJ101 — bare vs framework head-to-head (pilot): FINDINGS

**Status: COMPLETE (pilot).** The same 20 pilot items (10 computational + 10 adjudication) solved by
the **BARE arm** (model answers in prose, no framework) at **two model scales {Opus, Haiku}**, compared
against the **FRAMEWORK arm** (program-emission / rule-engine over provenanced IR). The point is the
*defensibility* delta, not accuracy.

## The numbers

| arm | computational correct | underdetermined fabricated (of 4) | **auditable** |
|---|---|---|---|
| bare-Opus | 10/10 | 0/4 | **0/20** |
| bare-Haiku | 10/10 | **1/4** | **0/20** |
| **framework** (Opus extraction) | 9/10 | abstains 4/4 (structural) | **20/20** |

## The honest result — and it is on-thesis

1. **Capable bare models are already good on outcomes.** Both bare-Opus and bare-Haiku scored 10/10 on
   the computational items and correctly abstained on 3–4 of the 4 underdetermined items. The framework
   does **not** beat a capable bare model on accuracy or abstention — and it shouldn't claim to (that is
   FrugalGPT's axis). This reproduces ADJ86's "the strong bare model is already good," now extended:
   *modern Haiku is also good*, far beyond the qwen-0.5B-class models where ADJ73/ADJ86 saw fabrication.

2. **The framework's clean, model-independent win is AUDITABILITY.** Every one of the 20 framework
   answers is a **machine-checkable artifact** — a program over typed, provenanced facts, or an engine
   verdict over byte-anchored rules. **All 20 bare answers are un-auditable prose (0/20).** A reviewer
   can re-run and re-check the framework's derivation; the bare answer must be taken on trust. Plus
   **correctability**: when the framework was wrong (MATH1 program bug), the error localized to the exact
   line and a one-edit fix re-derived; bare's wrong answers (none here, but in general) are confident
   prose with no locus.

3. **The weak-model fabrication crack appears — on CON2.** bare-Haiku committed to *"No, not owed a
   credit"* by **assuming** the dispositive fact (that the maintenance window was non-creditable),
   without the announcement-timing fact the policy turns on. The framework **abstained** and named the
   missing slot (`maintenance_announced_hours_in_advance`). This is the leading edge of the ADJ73/ADJ86
   weak-model failure (1/4), and exactly the case the framework's structural INDETERMINATE rescues.

## What this means for the corpus (a design finding for the full 100)

The 4 underdetermined items make the missing fact **salient**, so even Haiku catches 3/4. To measure
fabrication-prevention robustly, the full 100 needs **subtler underdetermination** — items where a
capable model *thinks* it has enough information but doesn't (the Palmyrene regime) — and/or a weaker
extraction model. This is the ADJ73 lesson restated: *the framework's outcome-benefit is conditional on
the failure being present*; with capable models on salient items the benefit is **auditability, not
accuracy**.

## Bottom line
Bare ≈ framework on outcomes for capable models; the framework's durable, model-independent win is that
**every answer is an auditable, correctable artifact** (20/20 vs 0/20), and on the weak model the
fabrication crack the framework prevents is already visible (CON2). The pilot's job — show the arms
run head-to-head and where the defensibility delta lives — is done; the full 100 should add
subtler-underdetermination items to surface the fabrication delta at scale.

## Reproduce
`python3 compare_arms.py` (Opus head-to-head); bare arms at both scales in `bare_results.json` /
`bare_results_haiku.json` (workflow: `bare_solve.workflow.js`, `model` via args).
