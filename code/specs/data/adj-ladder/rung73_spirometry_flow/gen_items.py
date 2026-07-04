"""Generate rung-73 (pulmonology spirometry corrected-flow index) items.json for the ADJ-LADDER.

Rung 73 opens the **pulmonology / spirometry** panel on the quantitative band — the arithmetic of a corrected expiratory
flow. A spirometer reads a `forced_volume`, then a `dead_space` is divided by an `effort_scale` and scaled by a
`breath_rate`, and that scaled quotient is SUBTRACTED from the forced volume. A leading term with a scaled quotient
subtracted introduces a genuinely NEW arithmetic shape on the ladder: a **term minus a scaled quotient** — `a - b/c*d`,
i.e. `a - ((b/c)*d)`.

This is the deliberate contrast to every neighbour so far: rung-72 was `a/b*c-d` (a quotient scaled, then a term
subtracted at the END); rung-73 leads with the whole term and subtracts a scaled quotient. It is the first ladder rung
whose SUBTRAHEND is itself a divide-then-multiply. The operation order inside the subtrahend matters: `b/c*d` is
left-to-right `((b/c)*d) = b*d/c`, NOT `b/(c*d)` — the distractor exploits exactly that confusion. Contrast the other
neighbours: rung-53 was `(a+b+c)/d` (a bare triple sum over a divisor) and rung-68 was `(a+b)*c/d` (a sum scaled then
divided). Here a whole term has a scaled quotient taken away.

The setup: a `forced_volume`, a `dead_space`, an `effort_scale`, and a `breath_rate`. The corrected flow is:

  CORRECTED FLOW     forced_volume - dead_space / effort_scale * breath_rate   [ term minus the scaled dead space ]
  DEAD-SPACE RATIO   dead_space / effort_scale                                 [ the quotient ]
  SCALED DEAD SPACE  dead_space / effort_scale * breath_rate                   [ the quotient scaled, the subtrahend ]

The **corrected flow** is what makes this rung distinctive — it is the ladder's first **leading term with a scaled
quotient subtracted** (the subtrahend is a divide-then-multiply). (The dead-space ratio `dead_space / effort_scale` and
the scaled dead space `dead_space / effort_scale * breath_rate` ride alongside as component readouts, so the panel
teaches the whole calculation — exactly as rungs 47-72 shipped their component sums/products/differences/ratios beside
the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the division of the dead space by the effort scale, the multiplication by the breath rate
(left-to-right), and the subtraction of that scaled quotient from the forced volume — and the harness reads the scalar
via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../71/72. This rung
exercises the engine across **a term minus a scaled quotient** — the fact that `a - b/c*d` is NOT `a - b/(c*d)` and NOT
`a - b*c/d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `*`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the dead-space ratio, the
scaled dead space, nor any corrected figure is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`forced_volume`, `dead_space`, `effort_scale`, `breath_rate`) so
no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    forced_volume - dead_space / (effort_scale * breath_rate)   DIVIDE the dead space by the PRODUCT of the
                                                                         effort scale and breath rate, not divide-then-
                                                                         multiply (the classic `a - b/c*d` vs
                                                                         `a - b/(c*d)` error), and
  SWAPPED    forced_volume - dead_space * effort_scale / breath_rate     MULTIPLY the dead space by the effort scale and
                                                                         divide by the breath rate — the operations
                                                                         swapped (`a - b*c/d` instead of `a - b/c*d`),

which are exactly the mistakes a student makes (folding both denominators into one product, or swapping which quantity
divides and which multiplies inside the subtrahend). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive; the effort scale and the breath rate both exceed one
(so the dead-space quotient differs from the scaled dead space) and differ from each other (so the corrected value
`a - b*d/c` differs from the swapped value `a - b*c/d`); the tables are chosen so the forced volume exceeds both scaled
subtrahends (`forced_volume > dead_space*breath_rate/effort_scale` and `> dead_space*effort_scale/breath_rate`), hence
every family member is positive; the five family values are pairwise distinct with a comfortable margin, asserted at
build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FORCED_VOLUME, DEAD_SPACE, EFFORT_SCALE, BREATH_RATE) — a forced volume, a dead space to divide, an effort scale to
# divide by, and a breath rate to scale by, all plain positive numbers with effort_scale > 1, breath_rate > 1,
# effort_scale != breath_rate, and forced_volume greater than both scaled subtrahends (dead_space*breath_rate/effort_scale
# and dead_space*effort_scale/breath_rate) so every family value (incl. crossed and swapped) stays positive. The five
# family values are asserted pairwise-distinct below.
TABLES = [
    (20, 12, 3, 2),
    (30, 12, 4, 3),
    (30, 10, 5, 2),
    (40, 18, 6, 3),
    (36, 15, 5, 3),
    (28, 8, 2, 3),
    (50, 20, 5, 4),
]

# The option family (5 members), all built from the four observed quantities via /, *, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "corrected_flow",
        "corrected expiratory flow (forced volume minus the scaled dead space)",
        "forced_volume - dead_space / effort_scale * breath_rate",
    ),
    (
        "dead_space_ratio",
        "the dead-space ratio (dead space over the effort scale)",
        "dead_space / effort_scale",
    ),
    (
        "scaled_dead_space",
        "the scaled dead space that is subtracted (dead-space ratio times the breath rate)",
        "dead_space / effort_scale * breath_rate",
    ),
    (
        "crossed",
        "the forced volume minus the dead space divided by the PRODUCT of the effort scale and breath rate, not divide-then-multiply (a wrong scaling)",
        "forced_volume - dead_space / (effort_scale * breath_rate)",
    ),
    (
        "swapped",
        "the forced volume minus the dead space MULTIPLIED by the effort scale and DIVIDED by the breath rate, the operations swapped (a wrong scaling)",
        "forced_volume - dead_space * effort_scale / breath_rate",
    ),
]
QUERIED = ["corrected_flow", "dead_space_ratio", "scaled_dead_space"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(forced_volume, dead_space, effort_scale, breath_rate):
    # Operation order mirrors the ADJ programs exactly (the left-to-right divide-then-multiply forms the scaled dead
    # space, then it is subtracted from the forced volume), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "corrected_flow": forced_volume - dead_space / effort_scale * breath_rate,
        "dead_space_ratio": dead_space / effort_scale,
        "scaled_dead_space": dead_space / effort_scale * breath_rate,
        "crossed": forced_volume - dead_space / (effort_scale * breath_rate),
        "swapped": forced_volume - dead_space * effort_scale / breath_rate,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for forced_volume, dead_space, effort_scale, breath_rate in TABLES:
        assert (
            forced_volume > 0
            and dead_space > 0
            and effort_scale > 0
            and breath_rate > 0
        ), (forced_volume, dead_space, effort_scale, breath_rate)
        # Effort scale and breath rate exceed one so the dead-space quotient differs from the scaled dead space, and they
        # differ from each other so the corrected value (a - b*d/c) differs from the swapped value (a - b*c/d). The forced
        # volume exceeds both scaled subtrahends so every family member (incl. crossed and swapped) is positive.
        assert effort_scale > 1, (forced_volume, dead_space, effort_scale, breath_rate)
        assert breath_rate > 1, (forced_volume, dead_space, effort_scale, breath_rate)
        assert effort_scale != breath_rate, (forced_volume, dead_space, effort_scale, breath_rate)
        fv = family_values(forced_volume, dead_space, effort_scale, breath_rate)
        for key, v in fv.items():
            assert v > 0, (key, forced_volume, dead_space, effort_scale, breath_rate, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    forced_volume,
                    dead_space,
                    effort_scale,
                    breath_rate,
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
                forced_volume,
                dead_space,
                effort_scale,
                breath_rate,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r73spir-{idx + 1:02d}",
                "qtype": "spirometry_flow",
                "stem": (
                    f"A spirometry test records a forced volume of {num(forced_volume)}, a dead space of "
                    f"{num(dead_space)} divided by an effort scale of {num(effort_scale)} and scaled by a breath rate of "
                    f"{num(breath_rate)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe forced_volume({num(forced_volume)})\n"
                    f"observe dead_space({num(dead_space)})\n"
                    f"observe effort_scale({num(effort_scale)})\n"
                    f"observe breath_rate({num(breath_rate)})\n"
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
            "ADJ-LADDER rung 73 — pulmonology spirometry corrected-flow index from four stated quantities (a NEW panel: "
            "pulmonology / spirometry). From a forced volume, a dead space to divide, an effort scale to divide by, and "
            "a breath rate to scale by, compute the corrected flow "
            "(forced_volume-dead_space/effort_scale*breath_rate), the dead-space ratio (dead_space/effort_scale), or "
            "the scaled dead space (dead_space/effort_scale*breath_rate). Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "TERM MINUS A SCALED QUOTIENT a - b/c*d (a leading term with a divide-then-multiply subtrahend subtracted — "
            "contrast rung-72 a/b*c-d which subtracts a bare term at the end; the left-to-right b/c*d = b*d/c, not "
            "b/(c*d)) — and the harness matches the scalar to the printed options. Contamination-safe: every index is "
            "built only from the four observed quantities via /, *, and - — no constant leaks, and neither the "
            "dead-space ratio, the scaled dead space, nor any corrected figure ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the "
            "slips students make: DIVIDING by the PRODUCT (a - b/(c*d), a wrong scaling) and SWAPPING the multiply and "
            "divide (a - b*c/d, a wrong scaling). The core confusion tested is that a - b/c*d is not a - b/(c*d) and "
            "not a - b*c/d."
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
