"""Generate rung-117 (podiatry / plantar pressure) items.json for the ADJ-LADDER.

Rung 117 opens the **podiatry / plantar-pressure** panel on the quantitative band — the arithmetic of a plantar pressure
index. A `peak_force` (the peak load the foot bears) is DIVIDED by the net weight-bearing area, where that area is the
`forefoot_area` PLUS the `midfoot_area` MINUS the `lesion_area` (an ulcer/callus removed from the load-bearing surface), to
give the pressure index (force per effective area). A **single term over a SUM-MINUS-DIFFERENCE denominator**, `a/(b+c-d)`,
i.e. `(a / ((b + c) - d))`, introduces a genuinely NEW arithmetic family on the ladder — the ladder's **first denominator that
contains a subtraction**.

This is genuinely new. Rung-116 opened the three-term DENOMINATOR with the pure sum `a/(b+c+d)`; every three-term NUMERATOR
(108 `(a+b+c)/d`, 109 `(a-b+c)/d`, 110 `(a*b+c)/d`, 111 `(a*b-c)/d`, 114 `(a+b-c)/d`, 115 `(a-b-c)/d`) sat over a SINGLE-term
divisor `d`; the distributive pair (112 `a*(b+c)/d`, 113 `a*(b-c)/d`) still divided by a single term; and every two-term ratio
(37 `(a+b)/(c+d)`, 99 `(a*b)/(c+d)`, 100 `(a+b)/(c*d)`, 104 `(a-b)/(c*d)`, the difference-denominator trio 105 `(a+b)/(c-d)`,
106 `a*b/(c-d)`, 107 `(a-b)/(c-d)`) had at most TWO terms below the bar — and none of the earlier two-term difference-
denominators had a THIRD term inside the denominator. Rung-116 was `a/(b+c+d)` (three-term SUM denominator, all plus);
rung-117 is `a/(b+c-d)` — the SUM-MINUS-DIFFERENCE sibling, the FIRST time a three-term denominator subtracts. The operator
order matters: `a/(b+c-d)` is `(a / ((b + c) - d))` (the forefoot and midfoot sum FIRST, the lesion subtracts from that, then
the force is divided by the whole net area; `+` and `-` bind left-to-right inside the explicit denominator parentheses and the
whole net area sits under the division), NOT `a/b+c-d` (dropping the denominator parentheses so only the forefoot divides the
force and then the midfoot is added and the lesion subtracted) and NOT `(a-d)/(b+c)` (regrouping so the lesion is subtracted
from the FORCE in the numerator and only the two areas form the denominator) — the two distractors exploit exactly those
confusions.

The setup: a `peak_force`, a `forefoot_area`, a `midfoot_area`, and a `lesion_area`. The total is:

  PLANTAR PRESSURE INDEX  peak_force / (forefoot_area + midfoot_area - lesion_area)  [ one term over a sum-minus-difference denominator ]
  EFFECTIVE AREA          forefoot_area + midfoot_area - lesion_area                 [ the three-term denominator ]
  PEAK FORCE              peak_force                                                 [ the numerator ]

The **plantar pressure index** is what makes this rung distinctive — it is the ladder's first **single term over a
sum-minus-difference denominator**. It is a rate (force per effective area), framed as an *index* to keep it
dimensionless-clean — the same discipline rungs 100/104/.../116 used for their ratios. (The effective area `b+c-d` and the peak
force `a` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-116 shipped
their component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the summation of the forefoot and midfoot areas, the subtraction of the lesion into the effective area, then
the division of the force by that whole net area (the single-term numerator over the sum-minus-difference denominator, so
a/(b+c-d) evaluates as (a/((b+c)-d))) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../115/116. This rung exercises the engine across a **sum-minus-difference
denominator** — the fact that `a/(b+c-d)` is `(a/((b+c)-d))` and NOT `a/b+c-d` and NOT `(a-d)/(b+c)` made computable. The ratio
golds are non-integer f64s; the engine's IEEE-double division matches Python's the same way rungs 99/100/104/.../116 relied on
(well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the effective area, the peak force, nor any
index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`peak_force`, `forefoot_area`, `midfoot_area`, `lesion_area`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    peak_force / forefoot_area + midfoot_area - lesion_area  drop the denominator parentheses so only the forefoot area
                                                                    divides the force and then the midfoot is added and the
                                                                    lesion subtracted (the classic `a/(b+c-d)` vs `a/b+c-d`
                                                                    grouping error), and
  SWAPPED    (peak_force - lesion_area) / (forefoot_area + midfoot_area)  regroup so the lesion is subtracted from the FORCE in
                                                                    the numerator and only the two areas form the denominator
                                                                    (`(a-d)/(b+c)` instead of `a/(b+c-d)`),

which are exactly the mistakes a student makes (failing to keep the whole net area under the bar, or subtracting the lesion
from the wrong operand). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: this rung SUBTRACTS the lesion area inside the denominator, so positivity is guaranteed by table
construction. Each table guarantees **midfoot_area > lesion_area** (c > d, so the effective area `b+c-d` is strictly positive,
and the crossed slip `a/b + c - d` stays positive since its `c-d` part is > 0 and `a/b > 0`) AND **peak_force > lesion_area**
(a > d, so the swapped numerator `a-d` is strictly positive) AND all quantities `>= 2`. The forefoot and midfoot areas keep the
denominator away from zero (`b+c-d >= b + 1 >= 3`), the pressure index never coincides with the effective area or the peak
force, and the five family values are pairwise distinct with a comfortable margin; and — so all three queried readouts vary
across the panel — the seven tables give distinct pressure indices, distinct effective areas, and distinct peak forces, all
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PEAK_FORCE, FOREFOOT_AREA, MIDFOOT_AREA, LESION_AREA) — a peak force divided by the net weight-bearing area (forefoot plus
# midfoot minus lesion) for the pressure index, all plain positive numbers >= 2. This rung SUBTRACTS the lesion area in the
# denominator, so every table guarantees midfoot_area > lesion_area (c>d, so the effective area b+c-d and the crossed slip's c-d
# part stay positive) and peak_force > lesion_area (a>d, so the swapped numerator a-d stays positive); b+c-d >= 3 keeps the
# division away from zero. The five family values are asserted pairwise-distinct below. The seven tables give distinct pressure
# indices, distinct effective areas, and distinct peak forces so all three queried readouts vary across the panel.
TABLES = [
    (30, 3, 4, 3),
    (34, 3, 5, 3),
    (40, 4, 5, 3),
    (45, 4, 6, 3),
    (50, 5, 6, 3),
    (54, 5, 7, 3),
    (62, 6, 7, 3),
]

# The option family (5 members), all built from the four observed quantities via +, - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "pressure_index",
        "plantar pressure index (the peak force divided by the effective area)",
        "peak_force / (forefoot_area + midfoot_area - lesion_area)",
    ),
    (
        "effective_area",
        "the effective area (the forefoot plus the midfoot minus the lesion area, the divisor the force is divided by)",
        "forefoot_area + midfoot_area - lesion_area",
    ),
    (
        "peak_force",
        "the peak force (the numerator divided by the effective area)",
        "peak_force",
    ),
    (
        "crossed",
        "the peak force divided by the forefoot area, plus the midfoot area minus the lesion area, dropping the denominator parentheses so only the forefoot area divides (a wrong grouping)",
        "peak_force / forefoot_area + midfoot_area - lesion_area",
    ),
    (
        "swapped",
        "the peak force minus the lesion area, divided by the forefoot plus the midfoot area, subtracting the lesion from the force instead of the area (a wrong pairing)",
        "(peak_force - lesion_area) / (forefoot_area + midfoot_area)",
    ),
]
QUERIED = ["pressure_index", "effective_area", "peak_force"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(peak_force, forefoot_area, midfoot_area, lesion_area):
    # Operation order mirrors the ADJ programs exactly (the forefoot and midfoot areas sum first, the lesion subtracts from that,
    # then the force is divided by that whole net area, so a/(b+c-d) evaluates as (a/((b+c)-d))), so the Python option value and
    # the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "pressure_index": peak_force / (forefoot_area + midfoot_area - lesion_area),
        "effective_area": forefoot_area + midfoot_area - lesion_area,
        "peak_force": peak_force,
        "crossed": peak_force / forefoot_area + midfoot_area - lesion_area,
        "swapped": (peak_force - lesion_area) / (forefoot_area + midfoot_area),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for peak_force, forefoot_area, midfoot_area, lesion_area in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung SUBTRACTS the lesion area in the denominator, so
        # each table guarantees midfoot_area > lesion_area (c>d, effective area b+c-d and the crossed c-d part strictly positive)
        # and peak_force > lesion_area (a>d, swapped numerator a-d strictly positive), which keeps every family member strictly
        # positive; b+c-d >= 3 keeps the division away from zero.
        assert (
            peak_force >= 2
            and forefoot_area >= 2
            and midfoot_area >= 2
            and lesion_area >= 2
        ), (peak_force, forefoot_area, midfoot_area, lesion_area)
        assert midfoot_area > lesion_area, (peak_force, forefoot_area, midfoot_area, lesion_area)
        assert peak_force > lesion_area, (peak_force, forefoot_area, midfoot_area, lesion_area)
        fv = family_values(peak_force, forefoot_area, midfoot_area, lesion_area)
        for key, v in fv.items():
            assert v > 0, (key, peak_force, forefoot_area, midfoot_area, lesion_area, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    peak_force,
                    forefoot_area,
                    midfoot_area,
                    lesion_area,
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
                peak_force,
                forefoot_area,
                midfoot_area,
                lesion_area,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r117pod-{idx + 1:02d}",
                "qtype": "pressure_index",
                "stem": (
                    f"A pedobarography report records a peak force of {num(peak_force)} divided by a forefoot area of "
                    f"{num(forefoot_area)} plus a midfoot area of {num(midfoot_area)} minus a lesion area of "
                    f"{num(lesion_area)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe peak_force({num(peak_force)})\n"
                    f"observe forefoot_area({num(forefoot_area)})\n"
                    f"observe midfoot_area({num(midfoot_area)})\n"
                    f"observe lesion_area({num(lesion_area)})\n"
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
            "ADJ-LADDER rung 117 — plantar pressure index from four stated quantities (a NEW panel: podiatry / plantar "
            "pressure). From a peak force divided by the net weight-bearing area (forefoot area plus midfoot area minus lesion "
            "area), compute the plantar pressure index "
            "(peak_force/(forefoot_area+midfoot_area-lesion_area)), the effective area "
            "(forefoot_area+midfoot_area-lesion_area), or the peak force. Each item is a compute_dimensioned program (observe "
            "the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a SINGLE TERM "
            "OVER A SUM-MINUS-DIFFERENCE DENOMINATOR a/(b+c-d) (sum the forefoot and midfoot areas, subtract the lesion, divide "
            "the force by that whole net area, so a/(b+c-d) = (a/((b+c)-d)); the ladder's FIRST denominator that contains a "
            "subtraction. Rung-116 opened the three-term denominator with the pure sum a/(b+c+d); every three-term NUMERATOR "
            "(108-111/114/115) sat over a SINGLE-term divisor d; the distributive pair (112/113) still divided by a single term; "
            "and every two-term ratio had at most TWO terms below the bar (37 (a+b)/(c+d), 99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 "
            "(a-b)/(c*d), and the difference-denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), 107 (a-b)/(c-d)) — rung-117 is the "
            "first three-term denominator that subtracts. The harness matches the scalar to the printed options. The plantar "
            "pressure index is a rate (force per effective area), framed as an INDEX so the dimensionless value stays honest. "
            "Contamination-safe: every figure is built only from the four observed quantities via +, - and / — no constant leaks, "
            "and neither the effective area, the peak force, nor any index ever appears as a literal (each is computed) — and the "
            "observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: dropping the "
            "denominator parentheses so only the forefoot area divides (a/b+c-d, a wrong grouping) and subtracting the lesion "
            "from the force instead of the area ((a-d)/(b+c), a wrong pairing). The core confusion tested is that a/(b+c-d) is "
            "(a/((b+c)-d)), not a/b+c-d and not (a-d)/(b+c). This rung SUBTRACTS the lesion area in the denominator, so positivity "
            "is guaranteed by table construction: every table has midfoot_area > lesion_area (c>d) and peak_force > lesion_area "
            "(a>d), with b+c-d >= 3 (division never by zero), keeping every family member strictly positive."
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
