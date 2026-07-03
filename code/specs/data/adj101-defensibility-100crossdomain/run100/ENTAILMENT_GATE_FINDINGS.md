# ADJ101 — adversarial entailment gate: FINDINGS (it works at the slot level; blunt application over-abstains)

**Status: COMPLETE.** An **independent adversary** (separate agent, blind to the extractor's self-label)
read all **240 dispositive slots** of the 100-item run, trying to **refute** that the scenario
establishes each value. A LEAP slot (not actually established) was nulled → the engine re-run.

## The adversarial read itself is sharp (39/240 LEAP)

The adversary discriminates well at the slot level — it correctly catches genuine over-reads while
confirming clearly-stated facts:
- **caught** (LEAP): `iodinated_contrast` ("CT scan but never says iodinated"), `paid_or_incurred_current_year`
  ("invoice issued ≠ paid/incurred"), `is_manufacturer` ("a defect exists but the defendant's role is
  unstated"), `threat_imminent` ("within the week ≠ imminent"), `is_interior_bathroom` ("never stated
  interior").
- **confirmed** (ENTAILED): stated numbers, explicit classifications, "no signed consent form exists",
  "she refuses IV antibiotics", etc.

So the **entailment signal is good**: the model's self-reported `ENTAILED` should not be trusted, and an
independent refutation-adversary recovers the real grounding status.

## But applying it bluntly (null any LEAP slot) is net-negative

| metric | baseline | gated (null all LEAP) |
|---|---|---|
| **underdetermined → INDETERMINATE** (the FIX) | 24/30 | **27/30** ✅ |
| **clean-determinate → DETERMINATE** (the COST) | 26/30 | **20/30** ❌ |
| verdict-family match (overall) | 88/100 | **74/100** ❌ |

The gate **fixed the 3 genuine over-reads** (TAX-4 `paid_or_incurred`, TAX-6 `full_time_student`,
STA-5 `is_manufacturer` → now abstain). But it caused **~14 false abstentions** on determinate items,
dropping overall match 88→74.

## Why it over-abstains — and the precise fix

The blunt rule abstains whenever a dispositive slot is LEAP, **even when the slot's imprecision does not
change the verdict.** The clearest example:

> **MED-2** (acetaminophen dose). The adversary ruled `hours_since_last_dose=12` a LEAP — correctly:
> "not received in the last 12 hours" means **≥12**, not exactly 12. But the rule only needs the
> interval **≥4 h**, which *any* value ≥12 satisfies. The exact value is **outcome-invariant**, yet
> nulling it forced an abstention. False abstention.

Contrast with a *real* over-read:

> **TAX-4**. `paid_or_incurred_current_year` is LEAP **and outcome-pivotal** — if the expense wasn't
> paid/incurred this year, the deduction flips. Here abstention is *correct*.

So the missing ingredient is **decision sensitivity** (the project already has it: **ADJ65**): a LEAP
slot should trigger abstention **only if it is outcome-pivotal** — i.e., its plausible alternative
values would actually flip the verdict. A LEAP slot whose uncertainty leaves the determination
invariant (MED-2's `≥12` vs the `≥4` threshold) should be kept, not nulled.

## The corrected design (for the next iteration)

The gate is two stages, not one:
1. **Adversarial entailment read** (this run): is the value genuinely established by the bytes? → ENTAILED / LEAP.
2. **Decision-sensitivity test** (ADJ65, to integrate): for a LEAP dispositive slot, does the verdict
   *change* across the slot's plausible alternative values? **Abstain iff it flips.**
This converts the blunt 88→74 into a targeted gate: TAX-4/TAX-6/STA-5 abstain (pivotal LEAP),
MED-2/CON-2/IMM-2/… stay determinate (non-pivotal LEAP). Expected: underdetermined → ~27-30, clean
→ ~30, overall match **up**, fabrications gone.

## Honest bottom line
The adversarial entailment read **is the right mechanism** and works at the slot level (it stops the
extractor from laundering an over-read as `ENTAILED`). But entailment alone is **not** the gate —
applied indiscriminately it over-abstains. The gate is **entailment × decision-sensitivity**: abstain
only on a LEAP that is *outcome-pivotal*. That is the next-iteration fix, and it reuses ADJ65 directly.

## Update — decision-sensitivity gate implemented (`decision_sensitivity_gate.py`)

The entailment × decision-sensitivity gate (boolean LEAP → negation-pivotality test; numeric LEAP →
kept unless removal yields a *different determinate* answer) was built and run on the existing data:

| | baseline | blunt gate | **sensitivity gate** |
|---|---|---|---|
| underdetermined → INDETERMINATE | 24/30 | 27/30 | **26/30** |
| clean-determinate → DETERMINATE | 26/30 | 20/30 | **23/30** |
| overall verdict match | 88/100 | 74/100 | **82/100** |

- ✅ **Catches the boolean fabrications:** TAX-4, TAX-6 now abstain (were confident DETERMINATE).
- ✅ **Fixes the numeric over-abstention:** MED-2, IMM-2, CON-2 are **kept determinate** (the blunt
  gate's main failure — over-precise LEAP values whose imprecision doesn't change the verdict).
- **Residual (82 vs 88):** 3 clean items (INS-2, CON-3, ACA-3) + a few override/exception items still
  false-abstain — a **single** adversary occasionally LEAP-flags a *pivotal boolean* that is actually
  adequately established. The fix is the **N-reader majority vote** (the CAS-write gate design): N
  independent adversaries reduce false LEAPs. This is the empirical motivation for that design.

Framing: verdict-match undersells this — the gate moves failures from **confident fabrication** (bad)
to **abstention-with-a-named-locus** (safe), which is the defensible direction. The remaining gap is
single-adversary calibration → N readers.

## Reproduce
`build_entailment_checks.py` → `adversarial_entail.workflow.js` (240 checks) → `apply_entail_rerun.py`
(blunt) → `decision_sensitivity_gate.py` (entailment × sensitivity). Verdicts: `entail_verdicts.json`.
