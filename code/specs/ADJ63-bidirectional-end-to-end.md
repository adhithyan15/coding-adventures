# ADJ63 — Bidirectional Justification, End to End (and what it exposed)

> **Status (2026-06-04):** Built and run. The ADJ61 (output) and ADJ62 (input)
> justification gates, wired into one pipeline and run on a **fresh, non-medical** case
> the agent found on its own — a railway-axle metallurgical failure analysis. All four
> corners of the provenance grid held. The run also surfaced the failure that motivates
> [ADJ64](ADJ64-underdetermination-gate.md): **over-attribution under missing evidence.**
> Implementation: [`code/specs/data/adj57/pipeline/`](data/adj57/pipeline/)
> (`bidirectional.workflow.js`, `run_bidirectional.py`).

## 1. The run

A brand-new domain (materials-failure-analysis, MDPI article PMC12387781), decomposed and
answered under **both** justification gates in sequence:

```
[1] INPUT coverage      100%  — 17 facts = all 1975 bytes (nothing dropped)
[2] INPUT extraction    32/32 — 24 extracted + 8 inferred, 0 rejected (nothing mis-extracted)
[3] OUTPUT grounding    13/13 — 7 evidence + 6 conclusion, 0 rejected (nothing invented)
=> BIDIRECTIONAL PROVENANCE COMPLETE
```

The input gate again did real work — it filed *"the failure mode is bending fatigue,"*
*"material defects ruled out,"* and *"root cause is improper machining"* as **inferred**
(syntheses across several bytes), keeping the directly-stated measurements as
**extracted**. The mechanism generalized to a hard quantitative engineering domain with
no changes. The full bidirectional provenance held on the first pass.

## 2. The finding — faithfulness ≠ completeness

The framework's byte-faithful answer: *"bending-fatigue cracking … driven by
**manufacturing/surface condition rather than material defect**."*

The held-aside ground truth: *"HIGH-STRESS FATIGUE FRACTURE — driven by **operating
stress exceeding the material's fatigue strength** — **not** a manufacturing-flaw
failure."* The decisive datum was an FEA result — operating stress ~273–299 MPa
*exceeding* the measured fatigue limit ~265–273 MPa — and **that comparison was never in
the decomposed `case_text`.**

So the answer agreed on everything the bytes contained (bending fatigue, initiation at
the fillet, surface stress concentrators, clean material) but **diverged on the top-line
root cause** — and the divergence traces entirely to a missing input. Crucially, the
framework **did not fabricate the FEA numbers to match a remembered answer.** It stayed
grounded, hedged ("most plausibly"), and gave the answer the bytes support. The error is
not an invention — it is a *visible gap*. An attending reading it would see every claim
byte-cited and immediately notice the stress-vs-fatigue-limit comparison was never made.

> **Byte provenance guarantees the answer follows from the bytes; it does not guarantee
> the bytes are complete.** When they are not, it shows up as a traceable divergence, not
> a confident fabrication.

## 3. What it exposed — and the fix

The byte-anchor stops *invented evidence*. It does **not** stop a subtler failure the
ADJ63 answer committed: with the deciding datum absent, the conclusion **drifted to the
loudest present lever** (the surface defects) and singled out one cause anyway. Every
claim was grounded; the *selection among causes* was not. Invention is not the only way
to be wrong — a conclusion can be **underdetermined**: several hypotheses fit the present
bytes and the observation that would separate them is missing.

That is the cut [ADJ64](ADJ64-underdetermination-gate.md) makes: when a conclusion cannot
be discriminated from its rivals by the present bytes, refuse to single one out, and emit
the missing discriminating observation as a *named provenance hole* — a query the
spider/CAS can fetch. Re-run on this same axle conclusion, ADJ64 flags it
**underdetermined** and names the exact missing measurement (operating stress vs. fatigue
limit) — the ground-truth root cause.

## 4. Honest note

Both gates passed clean on the first pass (0 rejections), so the live reject/kickback
path was again not exercised here. The reject path is covered by unit tests; a case (or
deriver) engineered to force a live kickback remains the way to exercise it end-to-end.
