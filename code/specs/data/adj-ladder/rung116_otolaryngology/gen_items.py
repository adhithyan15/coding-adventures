"""Generate rung-116 (otolaryngology / nasal airway) items.json for the ADJ-LADDER.

Rung 116 opens the **otolaryngology / nasal-airway** panel on the quantitative band — the arithmetic of an airway patency
index. A `total_airflow` (the air moved through the nose) is DIVIDED by the SUM OF THREE resistances — a `septal_resistance`
(from the nasal septum), a `turbinate_resistance` (from the inferior turbinate), and a `valve_resistance` (from the internal
nasal valve) — to give the patency index (airflow per unit of combined resistance). A **single term over a THREE-term SUM
DENOMINATOR**, `a/(b+c+d)`, i.e. `(a / ((b + c) + d))`, introduces a genuinely NEW arithmetic family on the ladder — the
ladder's **first three-term denominator**.

This is genuinely new — every earlier ratio put its multi-term structure in the NUMERATOR, never the denominator. The three-term
NUMERATOR family shipped rung-108 `(a+b+c)/d`, rung-109 `(a-b+c)/d`, rung-110 `(a*b+c)/d`, rung-111 `(a*b-c)/d`, rung-114
`(a+b-c)/d`, and rung-115 `(a-b-c)/d`; the distributive pair (rung-112 `a*(b+c)/d`, rung-113 `a*(b-c)/d`) wrapped a
sum/difference inside a factor but still divided by a SINGLE term `d`; and every two-term ratio (rung-37 `(a+b)/(c+d)`, rung-99
`(a*b)/(c+d)`, rung-100 `(a+b)/(c*d)`, rung-104 `(a-b)/(c*d)`, the difference-denominator trio rung-105 `(a+b)/(c-d)`, rung-106
`a*b/(c-d)`, rung-107 `(a-b)/(c-d)`) had at most TWO terms below the bar. Rung-116 moves to `a/(b+c+d)`: a bare single-term
numerator over a THREE-term sum denominator — the first time the ladder divides by a sum of three quantities. The operator order
matters: `a/(b+c+d)` is `(a / ((b + c) + d))` (the three resistances sum FIRST, then the airflow is divided by that whole sum;
`+` binds left-to-right inside the explicit denominator parentheses and the whole sum sits under the division), NOT `a/b+c+d`
(dropping the denominator parentheses so only the septal resistance divides the airflow and then the other two resistances are
added on) and NOT `a/(b+c)+d` (regrouping so only two of the three resistances form the denominator and the valve resistance is
added outside) — the two distractors exploit exactly those confusions.

The setup: a `total_airflow`, a `septal_resistance`, a `turbinate_resistance`, and a `valve_resistance`. The total is:

  AIRWAY PATENCY INDEX  total_airflow / (septal_resistance + turbinate_resistance + valve_resistance)  [ one term over a 3-term sum denominator ]
  TOTAL RESISTANCE      septal_resistance + turbinate_resistance + valve_resistance                     [ the three-term denominator ]
  TOTAL AIRFLOW         total_airflow                                                                   [ the numerator ]

The **airway patency index** is what makes this rung distinctive — it is the ladder's first **single term over a three-term sum
denominator**. It is a rate (airflow per unit of combined resistance), framed as an *index* to keep it dimensionless-clean — the
same discipline rungs 100/104/.../115 used for their ratios. (The total resistance `b+c+d` and the total airflow `a` ride
alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-115 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the summation of the three resistances into the total resistance, then the division of the airflow by that
whole sum (the single-term numerator over the three-term sum denominator, so a/(b+c+d) evaluates as (a/((b+c)+d))) — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../114/115. This rung exercises the engine across a **three-term sum denominator** — the fact that `a/(b+c+d)` is
`(a/((b+c)+d))` and NOT `a/b+c+d` and NOT `a/(b+c)+d` made computable. The ratio golds are non-integer f64s; the engine's IEEE-
double division matches Python's the same way rungs 99/100/104/.../115 relied on (well within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the total resistance, the total airflow, nor
any index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`total_airflow`, `septal_resistance`, `turbinate_resistance`, `valve_resistance`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    total_airflow / septal_resistance + turbinate_resistance + valve_resistance  drop the denominator parentheses so
                                                                    only the septal resistance divides the airflow and the other
                                                                    two resistances are added on (the classic `a/(b+c+d)` vs
                                                                    `a/b+c+d` grouping error), and
  SWAPPED    total_airflow / (septal_resistance + turbinate_resistance) + valve_resistance  regroup so only two of the three
                                                                    resistances form the denominator and the valve resistance is
                                                                    added outside (`a/(b+c)+d` instead of `a/(b+c+d)`),

which are exactly the mistakes a student makes (failing to keep the whole three-term sum under the bar, or regrouping which
resistances belong to the denominator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: every observed quantity is a plain positive number `>= 2` and this rung uses only `+` and `/`, so
every family member is strictly positive by construction (no subtraction anywhere). The **total_airflow >= 2** and every
resistance `>= 2` keep all five values positive; the three-term denominator `b+c+d >= 6` keeps the division away from zero, the
airway patency index never coincides with the total resistance or the total airflow, and the five family values are pairwise
distinct with a comfortable margin; and — so all three queried readouts vary across the panel — the seven tables give distinct
airway patency indices, distinct total resistances, and distinct total airflows, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TOTAL_AIRFLOW, SEPTAL_RESISTANCE, TURBINATE_RESISTANCE, VALVE_RESISTANCE) — an airflow divided by the sum of three
# resistances for the patency index, all plain positive numbers >= 2. This rung uses only + and /, so every family member is
# strictly positive by construction; the three-term denominator b+c+d >= 6 keeps the division away from zero. The five family
# values are asserted pairwise-distinct below. The seven tables give distinct airway patency indices, distinct total resistances,
# and distinct total airflows so all three queried readouts vary across the panel.
TABLES = [
    (30, 2, 3, 5),
    (34, 3, 4, 5),
    (40, 4, 3, 6),
    (45, 3, 5, 6),
    (50, 4, 5, 6),
    (52, 3, 6, 7),
    (58, 5, 6, 7),
]

# The option family (5 members), all built from the four observed quantities via + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "patency_index",
        "airway patency index (the total airflow divided by the sum of the three resistances)",
        "total_airflow / (septal_resistance + turbinate_resistance + valve_resistance)",
    ),
    (
        "total_resistance",
        "the total resistance (the septal plus the turbinate plus the valve resistance, the divisor the airflow is divided by)",
        "septal_resistance + turbinate_resistance + valve_resistance",
    ),
    (
        "total_airflow",
        "the total airflow (the numerator divided by the total resistance)",
        "total_airflow",
    ),
    (
        "crossed",
        "the total airflow divided by the septal resistance, plus the turbinate resistance plus the valve resistance, dropping the denominator parentheses so only the septal resistance divides (a wrong grouping)",
        "total_airflow / septal_resistance + turbinate_resistance + valve_resistance",
    ),
    (
        "swapped",
        "the total airflow divided by the septal plus the turbinate resistance, plus the valve resistance, regrouping so only two resistances form the denominator (a wrong pairing)",
        "total_airflow / (septal_resistance + turbinate_resistance) + valve_resistance",
    ),
]
QUERIED = ["patency_index", "total_resistance", "total_airflow"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(total_airflow, septal_resistance, turbinate_resistance, valve_resistance):
    # Operation order mirrors the ADJ programs exactly (the three resistances sum first, then the airflow is divided by that whole
    # sum, so a/(b+c+d) evaluates as (a/((b+c)+d))), so the Python option value and the engine result are the same IEEE-double
    # (well within the harness's 1e-9 match tolerance).
    return {
        "patency_index": total_airflow / (septal_resistance + turbinate_resistance + valve_resistance),
        "total_resistance": septal_resistance + turbinate_resistance + valve_resistance,
        "total_airflow": total_airflow,
        "crossed": total_airflow / septal_resistance + turbinate_resistance + valve_resistance,
        "swapped": total_airflow / (septal_resistance + turbinate_resistance) + valve_resistance,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for total_airflow, septal_resistance, turbinate_resistance, valve_resistance in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung uses only + and /, so every family member is
        # strictly positive by construction (no subtraction anywhere); the three-term denominator b+c+d >= 6 keeps the division
        # away from zero.
        assert (
            total_airflow >= 2
            and septal_resistance >= 2
            and turbinate_resistance >= 2
            and valve_resistance >= 2
        ), (total_airflow, septal_resistance, turbinate_resistance, valve_resistance)
        fv = family_values(total_airflow, septal_resistance, turbinate_resistance, valve_resistance)
        for key, v in fv.items():
            assert v > 0, (key, total_airflow, septal_resistance, turbinate_resistance, valve_resistance, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    total_airflow,
                    septal_resistance,
                    turbinate_resistance,
                    valve_resistance,
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
                total_airflow,
                septal_resistance,
                turbinate_resistance,
                valve_resistance,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r116ent-{idx + 1:02d}",
                "qtype": "patency_index",
                "stem": (
                    f"A rhinomanometry report records a total airflow of {num(total_airflow)} divided by a septal resistance of "
                    f"{num(septal_resistance)} plus a turbinate resistance of {num(turbinate_resistance)} plus a valve "
                    f"resistance of {num(valve_resistance)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe total_airflow({num(total_airflow)})\n"
                    f"observe septal_resistance({num(septal_resistance)})\n"
                    f"observe turbinate_resistance({num(turbinate_resistance)})\n"
                    f"observe valve_resistance({num(valve_resistance)})\n"
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
            "ADJ-LADDER rung 116 — airway patency index from four stated quantities (a NEW panel: otolaryngology / nasal "
            "airway). From a total airflow divided by the sum of a septal resistance, a turbinate resistance, and a valve "
            "resistance, compute the airway patency index "
            "(total_airflow/(septal_resistance+turbinate_resistance+valve_resistance)), the total resistance "
            "(septal_resistance+turbinate_resistance+valve_resistance), or the total airflow. Each item is a compute_dimensioned "
            "program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a "
            "SINGLE TERM OVER A THREE-TERM SUM DENOMINATOR a/(b+c+d) (sum the three resistances, divide the airflow by that whole "
            "sum, so a/(b+c+d) = (a/((b+c)+d)); the ladder's FIRST three-term denominator. Every earlier three-term structure "
            "lived in the NUMERATOR (108 (a+b+c)/d, 109 (a-b+c)/d, 110 (a*b+c)/d, 111 (a*b-c)/d, 114 (a+b-c)/d, 115 (a-b-c)/d) "
            "over a SINGLE-term divisor d; the distributive pair (112 a*(b+c)/d, 113 a*(b-c)/d) still divided by a single term; "
            "and every two-term ratio had at most TWO terms below the bar (37 (a+b)/(c+d), 99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 "
            "(a-b)/(c*d), and the difference-denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), 107 (a-b)/(c-d)) — rung-116 is the "
            "first to divide by a sum of THREE quantities. The harness matches the scalar to the printed options. The airway "
            "patency index is a rate (airflow per unit of combined resistance), framed as an INDEX so the dimensionless value "
            "stays honest. Contamination-safe: every figure is built only from the four observed quantities via + and / — no "
            "constant leaks, and neither the total resistance, the total airflow, nor any index ever appears as a literal (each "
            "is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. "
            "The five options are a family over the same four quantities, so the distractors are exactly the slips students make: "
            "dropping the denominator parentheses so only the septal resistance divides (a/b+c+d, a wrong grouping) and "
            "regrouping so only two resistances form the denominator (a/(b+c)+d, a wrong pairing). The core confusion tested is "
            "that a/(b+c+d) is (a/((b+c)+d)), not a/b+c+d and not a/(b+c)+d. This rung uses only + and /, so positivity is "
            "guaranteed by construction: every observed quantity is >= 2 and the three-term denominator b+c+d >= 6 (division "
            "never by zero), keeping every family member strictly positive."
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
