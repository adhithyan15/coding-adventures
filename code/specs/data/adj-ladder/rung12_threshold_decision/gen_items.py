"""Generate rung-12 (threshold decision) items.json for the ADJ-LADDER.

Rung 12 is a NEW decision SHAPE: the *wait-vs-treat threshold* decision. A single
patient measurement is compared against a published treatment threshold, and the
engine chooses the action — **start treatment now** or **continue observation** —
purely from whether the measured value crosses that threshold. This is distinct
from the earlier decision rungs:

  * rung-6 / rung-11 rank competing *diagnoses* by posterior (`decision_leader`);
  * rung-6b checks whether a dose can fit inside a *window* of two caps at once;
  * rung-12 asks the clean clinical question "given THIS number and THIS cut-off,
    do we act or wait?" — an observed-measurement-crosses-a-threshold decision.

The shape of one item:

  symbol marker : scalar
  constrain marker == <observed value>      # the patient's measurement, pinned
  constrain marker >= <treatment threshold> # (or <= for a low-is-dangerous marker)
  check

The engine runs feasibility over the reals (QF_LRA): the pinned value and the
threshold are *jointly satisfiable* exactly when the value has crossed the
threshold, so

  crossed  -> check.outcome == "sat" / "sat_real"  -> "Start treatment now"
  not yet  -> check.outcome == "unsat"             -> "Continue observation"

Python never compares the numbers — it only maps the engine's verdict to an option
label (`check_outcome` dispatch with custom `labels`). The reasoning the rung tests
is choosing the inequality *direction*: for a high-is-dangerous marker
(potassium, INR, calcium, lactate, bilirubin, ammonia) the danger is `marker >=
threshold`; for a low-is-dangerous marker (platelets, glucose, haemoglobin,
sodium, neutrophils, arterial pH) it is `marker <= threshold`. A naive
"bigger number means treat" heuristic gets exactly half the items wrong, because
half the markers are treated when they fall and half when they rise.

Contamination-safe: the only literals in each program are the observed value and
the threshold, and BOTH are printed verbatim in the stem; the answer is an action
LABEL (never a number), so nothing numeric can leak. Identifiers are digit-free
(the contamination gate reads digit-runs out of identifiers, so a `0`/`3` in a
variable name would leak as a literal). Gold rotates A-E by index, and every
item's expected verdict is asserted against the threshold at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# Five fixed, distinct, non-numeric action labels. The two clinically-relevant ones
# ("Start treatment now", "Continue observation") are the engine's two verdicts; the
# other three are plausible-but-wrong management distractors, identical across items
# so the rung tests the threshold reasoning, not option memorisation.
ACTIONS = [
    "Start treatment now",
    "Continue observation",
    "Discharge the patient home",
    "Refer for elective outpatient follow-up",
    "Repeat the test before deciding",
]
TREAT = "Start treatment now"
OBSERVE = "Continue observation"

# Each scenario is a clinical marker with a published treatment threshold and a
# direction. Fields:
#   (var, human marker name, unit phrase, threshold, direction,
#    treat_value, observe_value)
# direction "high" => danger when marker >= threshold (treat the high value);
# direction "low"  => danger when marker <= threshold (treat the low value).
# Variable names are digit-free. treat_value crosses the threshold, observe_value
# does not. Both values and the threshold are written verbatim into the stem.
SCENARIOS = [
    # --- high-is-dangerous: treat when marker >= threshold ----------------------
    ("potassium", "serum potassium", "mEq/L", "6.5", "high", "7.2", "5.1"),
    ("inr", "INR", "", "9", "high", "11", "4"),
    ("calcium", "serum calcium", "mg/dL", "14", "high", "15.5", "10.2"),
    ("lactate", "serum lactate", "mmol/L", "4", "high", "6.5", "2.1"),
    ("bilirubin", "total bilirubin", "mg/dL", "20", "high", "24", "12"),
    ("ammonia", "serum ammonia", "micromol/L", "100", "high", "180", "45"),
    # --- low-is-dangerous: treat when marker <= threshold -----------------------
    ("platelets", "platelet count", "thousand per microliter", "10", "low", "6", "45"),
    ("glucose", "blood glucose", "mg/dL", "50", "low", "38", "96"),
    ("hemoglobin", "hemoglobin", "g/dL", "7", "low", "6.2", "9.5"),
    ("sodium", "serum sodium", "mEq/L", "120", "low", "114", "132"),
    ("neutrophils", "absolute neutrophil count", "per microliter", "500", "low", "200", "1500"),
    ("ph", "arterial pH", "", "7.1", "low", "7.02", "7.32"),
]

# --- batch 2: therapeutic-range BAND decision ---------------------------------------------------
# A single measured drug level is pinned and tested against a TWO-SIDED therapeutic window
# (`constrain level >= low` AND `constrain level <= high`) with `check`: in-range (sat → keep the
# dose) vs out-of-range (unsat → adjust the dose). This is distinct from batch-1's single threshold
# (here BOTH bounds must hold at once) and from rung-6b's dose-window feasibility (there a FREE dose
# is searched; here a PINNED observed value is tested). The trap: a value can clear the lower bound
# yet still be out of range by exceeding the upper bound — so "is it above the minimum?" is not
# enough; both bounds decide. Half the out-of-range values are below the window and half above, so a
# one-sided heuristic fails.
BAND_ACTIONS = [
    "Continue the current dose",
    "Adjust the dose",
    "Stop the drug entirely",
    "Switch to a different drug",
    "Recheck the level tomorrow",
]
IN_RANGE = "Continue the current dose"
OUT_OF_RANGE = "Adjust the dose"

# (var, drug-level name, unit, low, high, in_range_value, out_of_range_value). low < in_range < high;
# the out-of-range value lies OUTSIDE [low, high] — some below `low`, some above `high`. Every literal
# is printed in the stem; the answer is an action LABEL; the variable name is digit-free.
THERAPEUTIC_RANGES = [
    ("level", "serum digoxin level", "ng/mL", "0.8", "2.0", "1.2", "2.8"),        # above the window
    ("level", "serum lithium level", "mEq/L", "0.6", "1.2", "0.9", "1.8"),        # above
    ("level", "vancomycin trough", "mcg/mL", "10", "20", "15", "6"),             # below the window
    ("level", "serum phenytoin level", "mcg/mL", "10", "20", "14", "27"),        # above
    ("level", "serum theophylline level", "mcg/mL", "10", "20", "16", "4"),      # below
    ("level", "serum valproate level", "mcg/mL", "50", "100", "72", "128"),      # above
    ("level", "serum carbamazepine level", "mcg/mL", "4", "12", "8", "2"),       # below
    ("level", "tacrolimus trough", "ng/mL", "5", "15", "9", "22"),               # above
    ("level", "cyclosporine trough", "ng/mL", "100", "300", "180", "60"),        # below
    ("level", "gentamicin peak", "mcg/mL", "5", "10", "7", "13"),                # above
    ("level", "serum procainamide level", "mcg/mL", "4", "10", "6", "2.5"),      # below
    ("level", "sirolimus trough", "ng/mL", "5", "15", "10", "19"),               # above
]


def _unit_suffix(unit: str) -> str:
    return f" {unit}" if unit else ""


def build():
    items = []
    idx = 0
    for var, marker, unit, threshold, direction, treat_value, observe_value in SCENARIOS:
        op = ">=" if direction == "high" else "<="
        # Two items per scenario: one that crosses (treat) and one that does not (observe).
        for observed, expected in ((treat_value, TREAT), (observe_value, OBSERVE)):
            # Engine truth: equality-pinned value is jointly satisfiable with the
            # threshold inequality iff the value has crossed the threshold.
            crossed = (
                float(observed) >= float(threshold)
                if direction == "high"
                else float(observed) <= float(threshold)
            )
            assert (TREAT if crossed else OBSERVE) == expected, (var, observed, threshold)

            gold_pos = idx % 5
            others = [a for a in ACTIONS if a != expected]
            opts = others[:]
            opts.insert(gold_pos, expected)
            opts = opts[:5]
            assert opts[gold_pos] == expected, opts
            assert len(set(opts)) == 5, opts
            options = {LETTERS[i]: opts[i] for i in range(5)}

            prog = (
                f"symbol {var} : scalar\n"
                f"constrain {var} == {observed}\n"
                f"constrain {var} {op} {threshold}\n"
                "check\n"
            )

            us = _unit_suffix(unit)
            if direction == "high":
                rule_phrase = (
                    f"at or above {threshold}{us} the condition is dangerous and requires "
                    f"immediate treatment; below it, observation is appropriate"
                )
            else:
                rule_phrase = (
                    f"at or below {threshold}{us} the condition is dangerous and requires "
                    f"immediate treatment; above it, observation is appropriate"
                )
            stem = (
                f"A patient's {marker} is {observed}{us}. The treatment threshold for "
                f"{marker} is {threshold}{us}: {rule_phrase}. Based on this measurement, "
                f"what is the correct management step?"
            )

            items.append({
                "id": f"r12td-{idx + 1:02d}",
                "qtype": "threshold_decision",
                "stem": stem,
                "program": prog,
                "answer_from": {
                    "type": "check_outcome",
                    "labels": {"sat": TREAT, "sat_real": TREAT, "unsat": OBSERVE},
                },
                "options": options,
                "gold_letter": LETTERS[gold_pos],
            })
            idx += 1

    # --- batch 2: therapeutic-range band decision (a two-sided window) -------------------------
    for var, marker, unit, low, high, in_value, out_value in THERAPEUTIC_RANGES:
        for observed, expected in ((in_value, IN_RANGE), (out_value, OUT_OF_RANGE)):
            # Engine truth: the pinned value is jointly satisfiable with BOTH bounds iff it lies
            # inside the closed window [low, high].
            in_range = float(low) <= float(observed) <= float(high)
            assert (IN_RANGE if in_range else OUT_OF_RANGE) == expected, (marker, observed, low, high)

            gold_pos = idx % 5
            others = [a for a in BAND_ACTIONS if a != expected]
            opts = others[:]
            opts.insert(gold_pos, expected)
            opts = opts[:5]
            assert opts[gold_pos] == expected, opts
            assert len(set(opts)) == 5, opts
            options = {LETTERS[i]: opts[i] for i in range(5)}

            prog = (
                f"symbol {var} : scalar\n"
                f"constrain {var} == {observed}\n"
                f"constrain {var} >= {low}\n"
                f"constrain {var} <= {high}\n"
                "check\n"
            )

            us = _unit_suffix(unit)
            stem = (
                f"A patient's {marker} is {observed}{us}. The therapeutic range for {marker} is "
                f"{low}{us} to {high}{us}: within that window (at or above {low}{us} and at or below "
                f"{high}{us}) the dose is correct; outside it — whether too low or too high — the dose "
                f"must be adjusted. Based on this level, what is the correct management step?"
            )

            items.append({
                "id": f"r12td-{idx + 1:02d}",
                "qtype": "band_decision",
                "stem": stem,
                "program": prog,
                "answer_from": {
                    "type": "check_outcome",
                    "labels": {"sat": IN_RANGE, "sat_real": IN_RANGE, "unsat": OUT_OF_RANGE},
                },
                "options": options,
                "gold_letter": LETTERS[gold_pos],
            })
            idx += 1

    return {
        "description": (
            "ADJ-LADDER rung 12 — threshold decision: the wait-vs-treat shape. A single patient "
            "measurement is pinned (`constrain marker == value`) and tested against a published "
            "treatment threshold (`constrain marker >= threshold`, or `<=` for a low-is-dangerous "
            "marker) with `check`. The engine's QF_LRA feasibility verdict IS the decision — the "
            "pinned value and the threshold are jointly satisfiable exactly when the value has "
            "crossed the threshold, so sat -> 'Start treatment now' and unsat -> 'Continue "
            "observation' (the harness only maps the verdict to an option label via check_outcome). "
            "The reasoning tested is the inequality DIRECTION: half the markers (potassium, INR, "
            "calcium, lactate, bilirubin, ammonia) are dangerous when HIGH and half (platelets, "
            "glucose, haemoglobin, sodium, neutrophils, arterial pH) when LOW, so a 'bigger number "
            "means treat' heuristic fails exactly half the items. No engine/harness change (reuses "
            "the rung-6b check_outcome extractor). Contamination-safe: the only literals are the "
            "observed value and the threshold, both printed in the stem, and the answer is an action "
            "label, so no result literal leaks; identifiers are digit-free; gold rotates A-E and "
            "every item's verdict is asserted against the threshold at build. Batch 2 adds the "
            "therapeutic-range BAND decision: a pinned drug level is tested against a TWO-SIDED "
            "window (`constrain level >= low` AND `constrain level <= high`), so sat -> 'Continue "
            "the current dose' (in range) and unsat -> 'Adjust the dose' (out of range). Both "
            "bounds must hold at once — a value can clear the lower bound yet still be out of range "
            "by exceeding the upper bound (half the out-of-range values are below the window, half "
            "above, so a one-sided 'is it above the minimum?' heuristic fails). Distinct from "
            "batch-1's single threshold and from rung-6b's free-dose window feasibility (here the "
            "observed value is PINNED, not searched)."
        ),
        "items": items,
    }


if __name__ == "__main__":
    doc = build()
    with open("items.json", "w") as f:
        json.dump(doc, f, indent=2)
        f.write("\n")
    print("wrote items.json:", len(doc["items"]), "items")
    for it in doc["items"]:
        print(it["id"], it["qtype"], "gold", it["gold_letter"], "=",
              it["options"][it["gold_letter"]])
