# Paper 1 · E1 — confirmatory omission ablation + abstention gate

> **Work item W4** (spec). Turns the ADJ73 *pilot* into the paper-grade confirmatory experiment for
> the mechanism (C2/C3): **forced justification of discards — not mere coverage — attacks
> omission-class hallucination**, and the **abstention gate** is the companion that stops the
> fabrication trade-off on uncovered items. Pilot: [`adj73-omission-ablation/`](../data/adj73-omission-ablation/FINDINGS.md).
> Plan: [`PAPER1-WORKPLAN.md`](PAPER1-WORKPLAN.md).

## 1. What ADJ73 established, and what it left open

**Established (the clean isolating result, at qwen2.5:3b where omission actually occurs):**
forced justification of discards cut the skim-trap rate 0.33 → **0.17** and lifted override accuracy
0.58 → **0.83**, while **coverage-only did not help** (skim 0.42) — you can tag a span `[DISCARD]`
without engaging it. The *justification requirement*, not the bookkeeping, is the lever.

**Left open (ADJ73's own "Next" list) — E1 must close all three:**
1. **Frontier difficulty.** The synthetic rule-override items top out at the 3b scale; capable models
   (gemma4, qwen2.5:14b) don't skim them (bare PS ≈ 1.00), so the mechanism is untested where it
   matters most. Need items that induce omission in *capable* models (the Palmyrene regime).
2. **Power.** n = 12 PS / 8 AB per cell, no CIs. Need powered n with bootstrap CIs.
3. **Abstention gate.** Justified-discards **increased fabrication on ABSENT items** for weak models
   (qwen2.5:1.5b 0.12 → 0.62; llama3.1:8b 0.25 → 0.62). The discrimination/abstention gate (ADJ67)
   must be added as a condition and the trade-off re-measured.

Two methodology lessons from ADJ73 are now **design requirements**, not findings:
- **Item difficulty must be calibrated to induce the failure** (bury the override after the general
  rule; verify bare-accuracy < ceiling via a smoke test, else there is no omission to fix).
- **Scoring must be style-invariant** (the justified condition emits NL answers like "No sales tax
  applies"; a token matcher biases *against* the framework). Lock the style-robust matcher.

## 2. Design

### 2.1 Conditions (4 — adds the gate)
| condition | what the model must do |
|---|---|
| **bare** | answer; no structure |
| **coverage-only** | tile every span (account-for / `[DISCARD]`) but no justification required |
| **justified-discards** | every discard carries a justification (the contract) |
| **justified + abstention gate** | as justified, **plus** a discrimination gate: if the question is not covered by the grounded material, **abstain** rather than answer |

The (bare → coverage → justified) ladder isolates *justification* from *bookkeeping* (the ADJ73
result); the 4th condition isolates whether the gate **recovers AB abstention without sacrificing the
PS gain**.

### 2.2 Item strata (×2 difficulty tiers — the key upgrade)
- **PS (present-but-skimmed):** the decisive override is present but buried after the general rule.
  - **PS-mid:** calibrated to induce skimming at ~3–8B (ADJ73 v2 style).
  - **PS-adversarial:** calibrated to induce omission in **capable/frontier** models (Palmyrene
    regime — the override is present but easy to skim even for a strong reader). *This tier is what
    makes E1 frontier-relevant; build and smoke-test it explicitly.*
- **AB (absent/uncovered):** the question's answer is **not** in the grounded material → correct
  behavior is **abstain**. Measures the fabrication trade-off and whether the gate fixes it.

Each item ships with a smoke-test bare-accuracy target (must be well below ceiling on PS, or it
doesn't induce omission and is rejected).

### 2.3 Models
Open-weights ladder spanning the capability floor and ceiling (e.g. qwen2.5 {1.5b, 3b, 7b/14b},
gemma, llama3.1:8b) **plus at least one frontier model** run on the PS-adversarial tier (the regime
ADJ73 couldn't reach). Temp 0.

### 2.4 Power
n ≥ 30 per cell (condition × stratum × tier × model), bootstrap 95% CIs on every reported rate.
Pre-register the three planned comparisons (H1–H3 below).

## 3. Hypotheses (pre-registered)
- **H1 (the lever).** Where omission occurs, **justified-discards** lowers the PS skim-trap rate and
  raises override accuracy vs **bare**. *Directional, primary.*
- **H2 (bookkeeping ≠ engagement).** **Coverage-only ≈ bare** on the skim-trap rate (tagging without
  justification does not attack omission). *The isolating control.*
- **H3 (the gate).** On **AB** items, **justified + abstention gate** restores abstention (cuts the
  fabrication rate back toward bare/below) **without** erasing the PS gain from H1. *The companion
  result.*
- **H4 (frontier relevance).** On the **PS-adversarial** tier, the H1 effect reproduces at
  capability scales where bare omission is non-trivial (skim-trap rate at the frontier > 0 under
  bare, and reduced under justified). *Closes ADJ73's biggest gap.*

Honest nulls we must be able to report: if coverage-only helps as much as justified (H2 fails), the
"justification is the lever" claim collapses; if the gate can't recover AB without costing PS (H3
fails), the contract needs the gate but the two conflict; if no items induce frontier omission (H4
fails), the mechanism is a small-model phenomenon — say so.

## 4. Metrics
- `skim_trap_rate` (PS, lower better) — **primary lever metric**.
- `override_accuracy` (PS) — secondary, style-invariant scored.
- `fabrication_rate` / `abstention_rate` (AB) — the gate's target.
- All with bootstrap CIs; report per model and pooled, broken out by tier.
- **Capability-floor reporting:** flag models below the floor (can't follow the clause+justify
  instruction) explicitly; the contract *hurts* below the floor (ADJ73: 1.5b PS 0.50 → 0.25) and
  that is an on-thesis boundary, not a failure to hide.

## 5. Validity guards (locked from ADJ73)
- Style-invariant scoring from saved raw outputs (re-scorable; never a brittle token match).
- Smoke-test each item induces the failure under bare before including it.
- Save all raw generations for deterministic re-scoring (as ADJ73 did — it is what caught Lesson 2).
- Cross-model arm built in (the model ladder), addressing the skeleton's single-family threat.

## 6. What counts as the headline E1 result
A figure: PS skim-trap rate across {bare, coverage, justified} showing **coverage ≈ bare ≫ justified**
(justification is the lever, bookkeeping isn't), with the **PS-adversarial tier reproducing it at
frontier scale**, plus an AB panel showing the **abstention gate** pulling fabrication back down while
PS accuracy stays up. That is C2/C3 made a slam-dunk rather than a pilot.

## 7. Build order (the run that follows this spec)
1. Author PS-mid, PS-adversarial, AB item sets; smoke-test induction; lock the style-invariant matcher.
2. Implement the 4 conditions (the abstention gate reuses the ADJ67 discrimination gate).
3. Run condition × stratum × tier × model at powered n, temp 0, save all raw.
4. Re-score style-invariantly; bootstrap CIs; per-model + pooled tables.
5. FINDINGS with the honest nulls and the capability-floor boundary.
Output: `code/specs/data/e1-ablation-confirmatory/` mirroring the ADJ run layout.

## 8. Closure status (paper-1 finalization)

For the manuscript, E1 is reported as **pilot + at-scale corroboration**, with one component deferred
— honestly scoped, not overclaimed:

- **The lever (H1/H2) — established at pilot, not yet frontier-confirmed.** ADJ73 cleanly isolates
  *justification of discards* (not coverage bookkeeping) as the mechanism that attacks omission, at
  the qwen2.5:3b scale where omission occurs. This is the C2/C3 evidence the paper carries. It is a
  **pilot**: n is small and the PS-adversarial/frontier tier (H4) has **not** been run.
- **The abstention gate (H3) — corroborated at scale by E3's benchmark.** The companion claim (abstain
  rather than fabricate on uncovered items) is independently supported, out of sample, by the 200-item
  cross-domain benchmark (run100 + run100b): underdetermined items resolve to INDETERMINATE
  **24/30** and **26/30**, with **zero confident fabrications** surviving the gate. The structural
  engine-INDETERMINATE there is a different *mechanism* from the discard-justification prompt ablation,
  but it is direct large-n evidence for the same *behavior* the gate targets.
- **Capability-floor boundary — corroborated by E2.** ADJ73's "the contract hurts below the floor"
  boundary (1.5b) is reinforced by E2's recurring-cost result: qwen2.5:1.5b misses a buried override
  on 7/7 cases raw, but is 7/7 *through the framework* — capability paid once into the pipeline lifts a
  sub-threshold model above threshold. This is the on-thesis capability-floor story, now measured twice.
- **The one deferred run: H4 (frontier omission ablation).** Inducing omission in a *capable* model
  (the PS-adversarial tier) and re-running the bare/coverage/justified ladder there remains unrun.
  **Recommendation for the paper:** report E1 as pilot (H1/H2) + at-scale abstention corroboration
  (H3 via E3) + capability-floor (E2), and place H4 in **Future Work / Limitations** — explicitly
  stating the lever is demonstrated where omission occurs (≤3B) and its frontier generality is open.
  This keeps E1 honest and unblocks the manuscript without a marginal-value frontier run.
