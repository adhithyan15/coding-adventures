"""Generate rung-74 (nephrology dialysis corrected-clearance index) items.json for the ADJ-LADDER.

Rung 74 opens the **nephrology / dialysis-clearance** panel on the quantitative band — the arithmetic of a corrected
solute clearance. A dialysis machine reads a `blood_flow`, then a `solute_load` is divided by a `membrane_factor` and
scaled by a `session_gain`, and that scaled quotient is ADDED to the blood flow. A leading term with a scaled quotient
added introduces a genuinely NEW arithmetic shape on the ladder: a **term plus a scaled quotient** — `a + b/c*d`, i.e.
`a + ((b/c)*d)`.

This is the deliberate contrast to rung-73's `a - b/c*d` (pulmonology spirometry), which SUBTRACTS the scaled quotient;
rung-74 ADDS it — the plus-counterpart, sharing the same divide-then-multiply subtrahend structure. The operation order
inside the added term matters: `b/c*d` is left-to-right `((b/c)*d) = b*d/c`, NOT `b/(c*d)` — the distractor exploits
exactly that confusion. Contrast the other neighbours: rung-53 was `(a+b+c)/d` (a bare triple sum over a divisor) and
rung-68 was `(a+b)*c/d` (a sum scaled then divided). Here a whole term has a scaled quotient added to it.

The setup: a `blood_flow`, a `solute_load`, a `membrane_factor`, and a `session_gain`. The corrected clearance is:

  CORRECTED CLEARANCE  blood_flow + solute_load / membrane_factor * session_gain   [ term plus the scaled load ]
  LOAD RATIO           solute_load / membrane_factor                               [ the quotient ]
  SCALED LOAD          solute_load / membrane_factor * session_gain                [ the quotient scaled, the addend ]

The **corrected clearance** is what makes this rung distinctive — it is the ladder's first **leading term with a scaled
quotient added** (the addend is a divide-then-multiply). (The load ratio `solute_load / membrane_factor` and the scaled
load `solute_load / membrane_factor * session_gain` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-73 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the division of the solute load by the membrane factor, the multiplication by the session gain
(left-to-right), and the addition of that scaled quotient to the blood flow — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../72/73. This rung exercises
the engine across **a term plus a scaled quotient** — the fact that `a + b/c*d` is NOT `a + b/(c*d)` and NOT `a + b*c/d`
made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `*`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the load ratio, the scaled
load, nor any corrected figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`blood_flow`, `solute_load`, `membrane_factor`, `session_gain`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    blood_flow + solute_load / (membrane_factor * session_gain)   DIVIDE the solute load by the PRODUCT of the
                                                                           membrane factor and session gain, not divide-
                                                                           then-multiply (the classic `a + b/c*d` vs
                                                                           `a + b/(c*d)` error), and
  SWAPPED    blood_flow + solute_load * membrane_factor / session_gain     MULTIPLY the solute load by the membrane
                                                                           factor and divide by the session gain — the
                                                                           operations swapped (`a + b*c/d` instead of
                                                                           `a + b/c*d`),

which are exactly the mistakes a student makes (folding both denominators into one product, or swapping which quantity
divides and which multiplies inside the added term). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive, so every family member — a sum of positive terms — is
automatically positive; the membrane factor and the session gain both exceed one (so the load-ratio quotient differs
from the scaled load) and differ from each other (so the corrected value `a + b*d/c` differs from the swapped value
`a + b*c/d`); the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BLOOD_FLOW, SOLUTE_LOAD, MEMBRANE_FACTOR, SESSION_GAIN) — a blood flow, a solute load to divide, a membrane factor to
# divide by, and a session gain to scale by, all plain positive numbers with membrane_factor > 1, session_gain > 1, and
# membrane_factor != session_gain. Because every family value is a sum of positive terms, positivity is automatic; the
# five family values are asserted pairwise-distinct below.
TABLES = [
    (20, 12, 3, 2),
    (30, 12, 4, 3),
    (30, 10, 5, 2),
    (40, 18, 6, 3),
    (36, 15, 5, 3),
    (28, 8, 2, 3),
    (50, 20, 5, 4),
]

# The option family (5 members), all built from the four observed quantities via /, *, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "corrected_clearance",
        "corrected solute clearance (blood flow plus the scaled solute load)",
        "blood_flow + solute_load / membrane_factor * session_gain",
    ),
    (
        "load_ratio",
        "the load ratio (solute load over the membrane factor)",
        "solute_load / membrane_factor",
    ),
    (
        "scaled_load",
        "the scaled load that is added (load ratio times the session gain)",
        "solute_load / membrane_factor * session_gain",
    ),
    (
        "crossed",
        "the blood flow plus the solute load divided by the PRODUCT of the membrane factor and session gain, not divide-then-multiply (a wrong scaling)",
        "blood_flow + solute_load / (membrane_factor * session_gain)",
    ),
    (
        "swapped",
        "the blood flow plus the solute load MULTIPLIED by the membrane factor and DIVIDED by the session gain, the operations swapped (a wrong scaling)",
        "blood_flow + solute_load * membrane_factor / session_gain",
    ),
]
QUERIED = ["corrected_clearance", "load_ratio", "scaled_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(blood_flow, solute_load, membrane_factor, session_gain):
    # Operation order mirrors the ADJ programs exactly (the left-to-right divide-then-multiply forms the scaled load,
    # then it is added to the blood flow), so the Python option value and the engine result are the same IEEE-double
    # (well within the harness's 1e-9 match tolerance).
    return {
        "corrected_clearance": blood_flow + solute_load / membrane_factor * session_gain,
        "load_ratio": solute_load / membrane_factor,
        "scaled_load": solute_load / membrane_factor * session_gain,
        "crossed": blood_flow + solute_load / (membrane_factor * session_gain),
        "swapped": blood_flow + solute_load * membrane_factor / session_gain,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for blood_flow, solute_load, membrane_factor, session_gain in TABLES:
        assert (
            blood_flow > 0
            and solute_load > 0
            and membrane_factor > 0
            and session_gain > 0
        ), (blood_flow, solute_load, membrane_factor, session_gain)
        # Membrane factor and session gain exceed one so the load-ratio quotient differs from the scaled load, and they
        # differ from each other so the corrected value (a + b*d/c) differs from the swapped value (a + b*c/d). Every
        # family member is a sum of positive terms, so positivity is automatic.
        assert membrane_factor > 1, (blood_flow, solute_load, membrane_factor, session_gain)
        assert session_gain > 1, (blood_flow, solute_load, membrane_factor, session_gain)
        assert membrane_factor != session_gain, (blood_flow, solute_load, membrane_factor, session_gain)
        fv = family_values(blood_flow, solute_load, membrane_factor, session_gain)
        for key, v in fv.items():
            assert v > 0, (key, blood_flow, solute_load, membrane_factor, session_gain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    blood_flow,
                    solute_load,
                    membrane_factor,
                    session_gain,
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
                blood_flow,
                solute_load,
                membrane_factor,
                session_gain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r74dial-{idx + 1:02d}",
                "qtype": "dialysis_clearance",
                "stem": (
                    f"A dialysis session records a blood flow of {num(blood_flow)}, a solute load of "
                    f"{num(solute_load)} divided by a membrane factor of {num(membrane_factor)} and scaled by a session "
                    f"gain of {num(session_gain)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe blood_flow({num(blood_flow)})\n"
                    f"observe solute_load({num(solute_load)})\n"
                    f"observe membrane_factor({num(membrane_factor)})\n"
                    f"observe session_gain({num(session_gain)})\n"
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
            "ADJ-LADDER rung 74 — nephrology dialysis corrected-clearance index from four stated quantities (a NEW "
            "panel: nephrology / dialysis-clearance). From a blood flow, a solute load to divide, a membrane factor to "
            "divide by, and a session gain to scale by, compute the corrected clearance "
            "(blood_flow+solute_load/membrane_factor*session_gain), the load ratio (solute_load/membrane_factor), or "
            "the scaled load (solute_load/membrane_factor*session_gain). Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "TERM PLUS A SCALED QUOTIENT a + b/c*d (a leading term with a divide-then-multiply addend added — contrast "
            "rung-73 a-b/c*d which subtracts the scaled quotient; the left-to-right b/c*d = b*d/c, not b/(c*d)) — and "
            "the harness matches the scalar to the printed options. Contamination-safe: every index is built only from "
            "the four observed quantities via /, *, and + — no constant leaks, and neither the load ratio, the scaled "
            "load, nor any corrected figure ever appears as a literal (each is computed) — and the observed quantities "
            "carry digit-free identifiers so no numeral hides inside a variable name. The five options are a family "
            "over the same four quantities, so the distractors are exactly the slips students make: DIVIDING by the "
            "PRODUCT (a + b/(c*d), a wrong scaling) and SWAPPING the multiply and divide (a + b*c/d, a wrong scaling). "
            "The core confusion tested is that a + b/c*d is not a + b/(c*d) and not a + b*c/d."
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
