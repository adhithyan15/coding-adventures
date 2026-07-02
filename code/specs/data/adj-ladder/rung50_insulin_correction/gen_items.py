"""Generate rung-50 (insulin correction dose) items.json for the ADJ-LADDER.

Rung 50 opens the **diabetes / insulin-correction-dosing** panel on the quantitative band — the arithmetic of the
short-acting insulin a correction ("sliding") scale delivers to bring a high blood glucose down to target. The dose is
the patient's correction factor multiplied by how far ABOVE target the glucose sits: the excess is the DIFFERENCE
`current_glucose − target_glucose`, and the dose scales that single difference by the correction factor. Multiplying a
lone factor by a parenthesised difference introduces a genuinely NEW arithmetic shape on the ladder: **distributive
product-over-a-difference** — `a · (b − c)` — one quantity multiplying a parenthesised difference of two.

The setup: a patient with correction factor `correction_factor` (units of insulin per point of glucose above target)
has a current glucose of `current_glucose` and a target of `target_glucose`. The correction dose is the factor times
the glucose excess:

  CORRECTION DOSE   correction_factor · (current_glucose − target_glucose)   [ units — the whole correction ]
  GLUCOSE EXCESS    current_glucose − target_glucose                         [ the difference: how far above target ]
  FACTOR × CURRENT  correction_factor · current_glucose                      [ one distributed term ]

The **correction dose** is what makes this rung distinctive — it is the ladder's first **distributive
product-over-a-difference**: a single factor multiplying a parenthesised difference. Contrast the neighbours already on
the ladder: rung-48 was a *sum times a difference* `(a+b)·(c−d)`, and rung-49 the *distributive product-over-a-SUM*
`a·(b+c)`; neither multiplied a lone factor by a parenthesised DIFFERENCE. (The glucose excess `current_glucose −
target_glucose` and the factor·current product `correction_factor · current_glucose` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47/48/49 shipped their component
sums/products/differences beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the three quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — including the inner `(current_glucose − target_glucose)` difference — and the harness
reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../48/49. This rung exercises the engine across a **single factor times a parenthesised difference** — the
distributive law `a·(b−c) = a·b − a·c` made computable.

Contamination-safe by construction: every formula is built ONLY from the three observed quantities via `·`, `−` and
`+` — **no structural constants** — so no numeric literal appears in any program, and neither the glucose excess, the
factor·current product, nor any correction-dose figure is ever a literal (each is computed from the observed
quantities). The observed quantities carry **digit-free identifiers** (`correction_factor`, `current_glucose`,
`target_glucose`) so no numeral hides inside a variable name.

The five options are a tight family over the same three quantities: the three real readouts plus the two classic
slips —

  SUM VERSION   correction_factor · (current_glucose + target_glucose)   ADD the current and target glucose instead of
                                                                         subtracting them (the excess must be a
                                                                         difference), and
  MISGROUPED    correction_factor · current_glucose − target_glucose     forget to distribute over the target — subtract
                                                                         the raw target glucose instead of the dose it
                                                                         offsets (`− target_glucose`, not
                                                                         `− correction_factor · target_glucose`),

which are exactly the mistakes a student makes (adding quantities that should be subtracted, or breaking the
distributive law by not scaling the subtracted term). Gold rotates A-E by index. QUERIED (used as gold) = the three
real readouts; all five always appear as options.

Distinctness: all three observed quantities are positive with `current_glucose > target_glucose` (the patient is
hyperglycaemic, so a correction is due), so every product, sum and difference is positive; the tables below are chosen
so the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CORRECTION_FACTOR, CURRENT_GLUCOSE, TARGET_GLUCOSE) — correction factor in units per point above target, glucose
# readings as plain positive numbers. All three quantities are strictly positive with CURRENT_GLUCOSE >
# TARGET_GLUCOSE, so the glucose excess is positive. The five family values are asserted pairwise-distinct (with
# margin) below.
TABLES = [
    (3, 40, 15),
    (2, 50, 20),
    (4, 35, 10),
    (5, 28, 12),
    (3, 60, 25),
    (2, 45, 18),
    (6, 30, 14),
]

# The option family (5 members), all built from the three observed quantities via *, - and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "correction_dose",
        "correction dose (the factor times how far the glucose sits above target)",
        "correction_factor * (current_glucose - target_glucose)",
    ),
    (
        "glucose_excess",
        "the glucose excess (current glucose minus the target)",
        "current_glucose - target_glucose",
    ),
    (
        "factor_times_current",
        "the factor times the CURRENT glucose (one distributed term, before offsetting the target)",
        "correction_factor * current_glucose",
    ),
    (
        "sum_version",
        "factor times the current and target glucose ADDED, not subtracted (a wrong excess)",
        "correction_factor * (current_glucose + target_glucose)",
    ),
    (
        "misgrouped",
        "factor times current glucose minus the raw TARGET, forgetting to scale it (broken distribution)",
        "correction_factor * current_glucose - target_glucose",
    ),
]
QUERIED = ["correction_dose", "glucose_excess", "factor_times_current"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(correction_factor, current_glucose, target_glucose):
    # Operation order mirrors the ADJ programs exactly (a single factor times a parenthesised difference), so the
    # Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match
    # tolerance).
    excess = current_glucose - target_glucose
    return {
        "correction_dose": correction_factor * excess,
        "glucose_excess": excess,
        "factor_times_current": correction_factor * current_glucose,
        "sum_version": correction_factor * (current_glucose + target_glucose),
        "misgrouped": correction_factor * current_glucose - target_glucose,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for correction_factor, current_glucose, target_glucose in TABLES:
        assert (
            correction_factor > 0
            and current_glucose > 0
            and target_glucose > 0
            and current_glucose > target_glucose
        ), (correction_factor, current_glucose, target_glucose)
        fv = family_values(correction_factor, current_glucose, target_glucose)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    correction_factor,
                    current_glucose,
                    target_glucose,
                    ORDER[i],
                    ORDER[j],
                    fv,
                )
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (
                key,
                correction_factor,
                current_glucose,
                target_glucose,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r50ins-{idx + 1:02d}",
                "qtype": "insulin_correction",
                "stem": (
                    f"A patient with a correction factor of {num(correction_factor)} units per point has a current "
                    f"glucose of {num(current_glucose)} and a target of {num(target_glucose)}. What is the "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe correction_factor({num(correction_factor)})\n"
                    f"observe current_glucose({num(current_glucose)})\n"
                    f"observe target_glucose({num(target_glucose)})\n"
                    f"let answer = {formula_of[key]}\n"
                    "? answer\n"
                ),
                "answer_from": {"type": "compute_dimensioned", "name": "answer"},
                "options": options,
                "gold_letter": LETTERS[gold_pos],
            })
            idx += 1
    return {
        "description": (
            "ADJ-LADDER rung 50 — insulin correction dose from three stated quantities (a NEW panel: diabetes / "
            "insulin-correction-dosing). From a correction factor plus a current and a target glucose compute the "
            "correction dose (correction_factor*(current_glucose-target_glucose)), the glucose excess "
            "(current_glucose-target_glucose), or the factor-times-current product (correction_factor*current_glucose). "
            "Each item is a compute_dimensioned program (observe the three quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a NEW shape, DISTRIBUTIVE PRODUCT-OVER-A-DIFFERENCE a*(b-c), the first "
            "product on the ladder to multiply a single factor by a parenthesised difference (distinct from rung-48 "
            "sum-times-difference (a+b)*(c-d) and rung-49 distributive product-over-a-sum a*(b+c)) — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only from the three "
            "observed quantities via *, - and + — no constant leaks, and neither the glucose excess, the factor-times-"
            "current product, nor any correction-dose figure ever appears as a literal (each is computed) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same three quantities, so the distractors are exactly the slips students "
            "make: ADDING the current and target glucose instead of subtracting them, and breaking the distributive "
            "law by subtracting the raw target instead of the dose it offsets (a*b-c, not a*(b-c)). The core confusion "
            "tested is distributing the correction factor over the glucose excess."
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
        print(it["id"], it["qtype"], "gold", it["gold_letter"],
              "=", round(it["options"][it["gold_letter"]]["value"], 6))
