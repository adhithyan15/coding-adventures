"""Generate rung-56 (cardiac stroke work) items.json for the ADJ-LADDER.

Rung 56 opens the **cardiology / ventricular-mechanics** panel on the quantitative band — the arithmetic of the work a
ventricle does in one beat. The work is the pressure the ventricle generates against times the volume it moves: the
DRIVING pressure is the systolic-minus-diastolic DIFFERENCE, and the ejected volume is the forward stroke volume PLUS
any regurgitant volume — a SUM. The stroke work is the driving pressure times the ejected volume. Multiplying a
difference by a sum introduces a genuinely NEW arithmetic shape on the ladder: **difference-times-sum** —
`(a − b) · (c + d)` — a parenthesised difference multiplied by a parenthesised sum.

The setup: a ventricle sees a systolic pressure `systolic_pressure` and a diastolic pressure `diastolic_pressure`, and
ejects a forward volume `forward_volume` plus a regurgitant volume `regurgitant_volume`. The stroke work is the driving
pressure times the ejected volume:

  STROKE WORK        (systolic_pressure − diastolic_pressure) · (forward_volume + regurgitant_volume)   [ the whole work ]
  DRIVING PRESSURE   systolic_pressure − diastolic_pressure                                             [ the difference ]
  EJECTED VOLUME     forward_volume + regurgitant_volume                                                [ the sum ]

The **stroke work** is what makes this rung distinctive — it is the ladder's first **difference-times-sum**: a
parenthesised difference multiplied by a parenthesised sum. Contrast the neighbours already on the ladder: rung-55 was a
*product of two SUMS* `(a+b)·(c+d)` and rung-48 a *sum times a difference* `(a+b)·(c−d)`; neither multiplied a DIFFERENCE
by a SUM. (The driving pressure `systolic_pressure − diastolic_pressure` and the ejected volume `forward_volume +
regurgitant_volume` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs
47-55 shipped their component sums/products/differences beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including the parenthesised difference and sum and their product — and the harness reads the
scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../54/55. This
rung exercises the engine across a **difference times a sum** — the distributive law `(a−b)·(c+d) = ac+ad−bc−bd` made
computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `−` and `·`
— **no structural constants** — so no numeric literal appears in any program, and neither the driving pressure, the
ejected volume, nor any stroke-work figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`systolic_pressure`, `diastolic_pressure`, `forward_volume`,
`regurgitant_volume`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  SUM VERSION   (systolic_pressure − diastolic_pressure) + (forward_volume + regurgitant_volume)   ADD the driving
                                                                                                   pressure and the
                                                                                                   ejected volume instead
                                                                                                   of multiplying, and
  MISGROUPED    (systolic_pressure − diastolic_pressure) · forward_volume + regurgitant_volume      multiply the driving
                                                                                                   pressure by only the
                                                                                                   FORWARD volume, then add
                                                                                                   the regurgitant volume
                                                                                                   on (`… · forward +
                                                                                                   regurgitant`, not `… ·
                                                                                                   (forward + regurgitant)`),

which are exactly the mistakes a student makes (adding a pressure and a volume that should be multiplied, or breaking the
grouping so the pressure multiplies only the first volume term). Gold rotates A-E by index. QUERIED (used as gold) = the
three real readouts; all five always appear as options.

Distinctness: all four observed quantities are positive with `systolic_pressure > diastolic_pressure` (the driving
pressure is positive), so every sum, difference and product is positive; the tables below are chosen so the five family
values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SYSTOLIC_PRESSURE, DIASTOLIC_PRESSURE, FORWARD_VOLUME, REGURGITANT_VOLUME) — two pressures (mmHg) with
# SYSTOLIC > DIASTOLIC, and two volumes (mL), all plain positive numbers. The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (120, 80, 60, 20),
    (140, 90, 55, 25),
    (110, 70, 70, 10),
    (130, 85, 50, 30),
    (150, 100, 45, 15),
    (125, 75, 65, 35),
    (135, 95, 40, 28),
]

# The option family (5 members), all built from the four observed quantities via +, - and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "stroke_work",
        "stroke work (the driving pressure times the ejected volume)",
        "(systolic_pressure - diastolic_pressure) * (forward_volume + regurgitant_volume)",
    ),
    (
        "driving_pressure",
        "the driving pressure (systolic minus diastolic)",
        "systolic_pressure - diastolic_pressure",
    ),
    (
        "ejected_volume",
        "the ejected volume (forward plus regurgitant)",
        "forward_volume + regurgitant_volume",
    ),
    (
        "sum_version",
        "the driving pressure and the ejected volume ADDED, not multiplied (a wrong work)",
        "(systolic_pressure - diastolic_pressure) + (forward_volume + regurgitant_volume)",
    ),
    (
        "misgrouped",
        "the driving pressure times only the FORWARD volume, with regurgitant added on (broken grouping)",
        "(systolic_pressure - diastolic_pressure) * forward_volume + regurgitant_volume",
    ),
]
QUERIED = ["stroke_work", "driving_pressure", "ejected_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(systolic_pressure, diastolic_pressure, forward_volume, regurgitant_volume):
    # Operation order mirrors the ADJ programs exactly (a parenthesised difference times a parenthesised sum; and, for
    # the misgrouped slip, the driving-times-forward product binds tighter than the trailing addition), so the Python
    # option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    drive = systolic_pressure - diastolic_pressure
    ejected = forward_volume + regurgitant_volume
    return {
        "stroke_work": drive * ejected,
        "driving_pressure": drive,
        "ejected_volume": ejected,
        "sum_version": drive + ejected,
        "misgrouped": drive * forward_volume + regurgitant_volume,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for systolic_pressure, diastolic_pressure, forward_volume, regurgitant_volume in TABLES:
        assert (
            systolic_pressure > diastolic_pressure > 0
            and forward_volume > 0
            and regurgitant_volume > 0
        ), (systolic_pressure, diastolic_pressure, forward_volume, regurgitant_volume)
        fv = family_values(systolic_pressure, diastolic_pressure, forward_volume, regurgitant_volume)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    systolic_pressure,
                    diastolic_pressure,
                    forward_volume,
                    regurgitant_volume,
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
                systolic_pressure,
                diastolic_pressure,
                forward_volume,
                regurgitant_volume,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r56work-{idx + 1:02d}",
                "qtype": "stroke_work",
                "stem": (
                    f"A ventricle sees a systolic pressure of {num(systolic_pressure)} mmHg and a diastolic pressure of "
                    f"{num(diastolic_pressure)} mmHg, and ejects a forward volume of {num(forward_volume)} mL plus a "
                    f"regurgitant volume of {num(regurgitant_volume)} mL. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe systolic_pressure({num(systolic_pressure)})\n"
                    f"observe diastolic_pressure({num(diastolic_pressure)})\n"
                    f"observe forward_volume({num(forward_volume)})\n"
                    f"observe regurgitant_volume({num(regurgitant_volume)})\n"
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
            "ADJ-LADDER rung 56 — cardiac stroke work from four stated quantities (a NEW panel: cardiology / "
            "ventricular-mechanics). From a systolic and a diastolic pressure and a forward and a regurgitant volume "
            "compute the stroke work "
            "((systolic_pressure-diastolic_pressure)*(forward_volume+regurgitant_volume)), the driving pressure "
            "(systolic_pressure-diastolic_pressure), or the ejected volume (forward_volume+regurgitant_volume). Each "
            "item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine "
            "carries the arithmetic — a NEW shape, DIFFERENCE-TIMES-SUM (a-b)*(c+d), the first on the ladder to multiply "
            "a difference by a sum (distinct from rung-55 product-of-two-sums (a+b)*(c+d) and rung-48 "
            "sum-times-difference (a+b)*(c-d)) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via +, - and * — no "
            "constant leaks, and neither the driving pressure, the ejected volume, nor any stroke-work figure ever "
            "appears as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make: ADDING the driving pressure and the ejected volume instead "
            "of multiplying them, and breaking the grouping so the pressure multiplies only the forward volume "
            "((a-b)*c+d, not (a-b)*(c+d)). The core confusion tested is multiplying a grouped difference by a grouped "
            "sum."
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
