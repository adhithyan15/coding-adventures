# ADJ86 pilot v2 — findings (2×2, real pipeline, pipeline-order fixed)

12 adjudication items (2 domains × 4 strata), 2×2 **{Haiku, Opus} × {bare, framework}**.
Framework = the real pipeline (Phase 1 input-IR → Phase 2 rulebook-derived-**from**-the-IR →
deterministic `engine.py`). Bare = one-shot prose; a fixed Opus auditor scores bare
defensibility.

| cell | accuracy | defensibility | underdetermined (n=3) |
|---|---|---|---|
| BARE Haiku | 11/12 | 0.88 (audited frac) | fabricates ≥1/3 (audit flags NOT_DEFENSIBLE) |
| **FRAMEWORK Haiku** | **11/12** | **12/12 byte-clean, 0 halluc** | **INDETERMINATE 3/3 (structural)** |
| BARE Opus | 12/12 | 0.99 | abstains 3/3 |
| FRAMEWORK Opus | 9/12 | 12/12 byte-clean, 0 halluc | INDETERMINATE 3/3 |

## 1. The pipeline-order fix worked
v1 (rulebook-first) gave framework accuracy 6/12 from slot-vocabulary misalignment. v2
(IR-first; rulebook conditions on the IR's *actual* slot names/values) → **Haiku 11/12**. The
two stages can no longer represent the same thing differently.

## 2. Defensibility-parity on the underdetermined subclass — the thesis, demonstrated
On the baited-underdetermined items (dispositive fact withheld): **bare-Haiku fabricates a
determination** ("DENIED" on MED2, "INVALID" on LAW2 — the blind auditor marks them
NOT_DEFENSIBLE), while **framework-Haiku returns INDETERMINATE structurally on all 3,
identical to framework-Opus**, with 0 hallucinated slots. The engine removes the weak model's
overclaim: the verdict lives in `engine.py`, so a faithful Haiku extractor is **as defensible
as Opus**. *This is the lift the thesis predicted.*

## 3. Two honest complications
- **The strong bare model is already good.** Opus-bare is 12/12 and 0.99-defensible and
  abstains 3/3 — the framework's defensibility win is concentrated on the **weak** model.
- **Over-extraction costs accuracy through the engine.** Framework-Opus accuracy *dropped*
  to 9/12 — Opus over-elaborates the IR/rulebook (extra inferred slots, extra rules), which
  the deterministic engine flags as ambiguous → spurious INDETERMINATE/CONFLICT on
  *determinate* items. **Haiku's simpler extraction (11/12) beats Opus's (9/12) through the
  engine** — replicating ADJ84 Finding 2 ("Haiku produced the cleaner rulebook"). The engine
  rewards minimal faithful extraction.

## 4. Measurement caveat (fix before the full run)
The keyword abstain-detector is crude — it credited Haiku-bare with "abstaining" when its
prose contained "does not state" even though it committed to "DENIED". The **blind audit**
(NOT_DEFENSIBLE) and the **engine verdict** are the reliable signals; drop keyword-accuracy
in favour of those.

## 5. Net for the full 100-run
- ✓ Thesis supported on the subclass where a weak model would overclaim: framework lifts
  Haiku to Opus-level defensibility (structural INDETERMINATE; 0 fabrication).
- The headline metric should be **defensibility on the underdetermined/baited strata**
  (where bare-Haiku fails); weight those strata up.
- Decide the over-extraction issue: either **constrain extraction** (minimal slots) or report
  the engine-rewards-minimal-extraction finding (and improve engine precedence/CONFLICT).
- Replace keyword abstain-scoring with the audit + engine verdict.

Cost: v2 = 96 agents, ~2.0M tokens for 12 items × 2 models → ~**8–9M tokens per model-pair**
for the full 100 (the 2×2).

## 6. The blind judge (replacing the keyword scorer) — and the provenance hole it exposed
A blind Opus judge scored bare vs framework (unlabeled A/B). It confirmed the win on the
underdetermined subclass (Haiku bare 0.75 → framework 1.00) but initially rated the framework
LOWER overall — because the rulebook leg of byte-provenance was never enforced:
- **The engine never received the policy**, so rule `source_span`s were never verified (0/50
  were actually fabricated here, but nothing checked).
- **Inferred slots (span=null) were exempt** from byte-accounting by design, so a model could
  launder an assumption ("cardiologist→specialist") through an inferred slot and a rule could
  condition on it to reach a confident verdict with the assumption invisible.

`provenance_engine.py` closes both: (A) verify every rule span verbatim in the policy; (B)
surface any dispositive inferred condition as an explicit `DETERMINATE(assumes:…)`. After the
fix + honest rendering (GROUNDED vs ASSUMED labels), the blind judge flipped: **FW-Haiku 0.81→
0.89 (now > bare-Haiku 0.85), more-defensible 5→8/12; UD subclass 3/3.** Strong-model bare
still edges FW-Opus (0.96 vs 0.86) — the win is for the weak model, as predicted.

## 7. Byte-cited justification gate for inferred facts (the fix for over-flagging)
The naive "all inferred = assumption" over-flagged grounded computations (the judge caught
"four months"→"<1 year" as manufactured uncertainty). Fix (ADJ61 at the slot level): every
inferred fact must cite the **exact scenario bytes** it derives from, then an adversarial check
asks *does the meaning of those bytes alone ENTAIL the inference, or is it a world-knowledge
LEAP?* On the 17 inferred facts: **6 GROUNDED (ENTAILED) + 11 ASSUMPTION (LEAP)**, byte-anchor
0 failures. It correctly grounds "four months"→"<1 year", "emergency room"→"emergency", and
flags "cardiologist"→"specialist", "$"→"USD", "member"→"covered", "sixteen"→"minor". Byte
provenance is now complete on all three legs (stated→scenario, inferred→basis+entailment,
rulebook→policy).

## 8. Net / next
- Integrate the justification gate into the pipeline (Phase 1 emits `basis_span` for inferred
  slots; a justification stage classifies ENTAILED/LEAP; the engine flags only LEAP) and
  re-run the pilot to confirm the over-flag penalty is gone.
- Then scale to the full 100 (10 domains), 2×2 {Haiku,Opus}×{bare,framework}, blind judge +
  accuracy, weighting the underdetermined/baited strata.

## 9. v3 — full provenance end-to-end (honest result: architecture right, score did not improve)
v3 wires the unified contract into the live pipeline: Phase 1 emits `basis_span` for every
inferred slot; a per-slot adversarial entailment gate (model-self-run) classifies ENTAILED vs
LEAP; the provenance-complete engine flags only LEAP dispositive conditions; rulebook spans
verified against the policy. (Also fixed a workflow stall: one `bare` agent looped 15× on a
required `reasoning` field — required fields trimmed to the essential so a schema miss can't
deadlock `parallel()`.)

**Blind judge (un-blinded):** FW-Haiku **0.79** vs bare-Haiku 0.84; FW-Opus **0.81** vs
bare-Opus 0.97. Framework byte-clean 12/12 (no hallucinated slots, no laundered conditions —
**the escape hatch is closed**), INDETERMINATE 3/3 on underdetermined. But the framework does
**NOT** beat bare overall this run. Why (judge-specific, fixable — not a provenance failure):
- **Inline gate noise.** Each model self-gating its own inferences mis-classifies some
  ENTAILED facts as LEAP → spurious assumptions the judge calls "manufactured doubt"
  (Opus flagged `holding_period_is_short`=true a LEAP; the judge: "four months < one year is
  pure arithmetic"). The standalone Opus gate was 17/17, but inline + vague slot names is
  noisy: **`holding_period_less_than_one_year` → ENTAILED, but `is_short` → LEAP** (needs a
  threshold). Slot *naming* now drives the verdict.
- **Engine over-abstention on determinate items.** LAW1-Opus wrongly returned INDETERMINATE
  (a needed slot wasn't extracted / the rule couldn't fire) → judge scored it 0. The
  deterministic engine's rigidity (no forward-chaining of derived slots; flat precedence)
  costs determinate-item defensibility.

**Honest conclusion.** Provenance-completeness is the correct, necessary architecture — it
provably closes the inferred-slot hallucination hole (0 laundered conditions). But completing
it did not raise the blind-judge score, because the **binding constraints moved**: (1) the
entailment gate must be *reliable* (precise slot naming + a verified/stronger gate, not noisy
inline self-gating), and (2) the engine must stop over-abstaining on determinate items
(forward-chain ENTAILED derived slots; show the comparison step). The framework's defensibility
advantage is currently **confined to genuinely-underdetermined inputs** (FW-Haiku 0.95 vs bare
0.75) — where refusing to fabricate is unambiguously right — and is *erased on determinate
items* by gate noise + over-abstention. The thesis ("framework lifts weak Haiku to Opus-level
defensibility") is **not yet demonstrated end-to-end**; it holds on the underdetermined slice
only. Next: precise-slot-naming + a verified gate, forward-chaining in the engine, then re-run
before scaling to 100.
