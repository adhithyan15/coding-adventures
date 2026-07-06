"""Generate rung-83 (hepatology hepatic-clearance index) items.json for the ADJ-LADDER.

Rung 83 opens the **hepatology / clearance** panel on the quantitative band — the arithmetic of a net hepatic clearance.
A `portal_load` is presented to the liver, and an uptake term is subtracted from it: an `uptake_rate` is multiplied by a
`sinusoid_flow` (a PRODUCT) and that product is spread over a `clearance_divisor` (making a product-over-divisor,
`uptake_rate * sinusoid_flow / clearance_divisor`), and this whole extracted term is SUBTRACTED from the portal load. A
bare leading term minus a product-over-divisor introduces a genuinely NEW arithmetic shape on the ladder: a **term MINUS
a product-quotient** — `a-b*c/d`, i.e. `a - (b*c)/d`.

This is genuinely new: rung-73 was `a-b/c*d` (`a - (b/c)*d`, the second operand DIVIDED then multiplied), and rungs 34/35
were sums/differences of two full products; here the subtracted term is a three-factor product-over-divisor
`(b*c)/d` taken off a bare first operand. The operator order matters: `a-b*c/d` is `a - ((b*c)/d)` by precedence (the
multiply and divide bind before the subtraction, left-to-right so `b*c/d = (b*c)/d`), NOT `(a-b)*c/d` (subtracting `b`
from `a` FIRST and then scaling by `c/d`) and NOT `a-b/c*d` (dividing `b` by `c` first, then times `d`) — the two
distractors exploit exactly those confusions (and each lands on a DIFFERENT already-shipped shape: `(a-b)*c/d` is
rung-67's, `a-b/c*d` is rung-73's).

The setup: a `portal_load`, an `uptake_rate`, a `sinusoid_flow`, and a `clearance_divisor`. The hepatic clearance is:

  HEPATIC CLEARANCE   portal_load - uptake_rate * sinusoid_flow / clearance_divisor   [ term minus a product-quotient ]
  EXTRACTION LOAD     uptake_rate * sinusoid_flow / clearance_divisor                 [ the subtracted product-quotient term ]
  UPTAKE PRODUCT      uptake_rate * sinusoid_flow                                     [ the numerator product, before dividing ]

The **hepatic clearance** is what makes this rung distinctive — it is the ladder's first **bare term MINUS a
product-over-divisor**. (The extraction load `b*c/d` and the uptake product `b*c` ride alongside as component readouts,
so the panel teaches the whole calculation — exactly as rungs 47-82 shipped their component sums/products/differences/
ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the uptake rate by the sinusoid flow, the division of that product by the
clearance divisor, and the subtraction of the extracted term from the portal load (multiply/divide before subtract) —
and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as
rungs 8/16/.../81/82. This rung exercises the engine across **a term minus a product-quotient** — the fact that
`a-b*c/d` is `a - ((b*c)/d)` and NOT `(a-b)*c/d` and NOT `a-b/c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `*`, and `/`
— **no structural constants** — so no numeric literal appears in any program, and neither the extraction load, the
uptake product, nor any clearance figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`portal_load`, `uptake_rate`, `sinusoid_flow`, `clearance_divisor`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (portal_load - uptake_rate) * sinusoid_flow / clearance_divisor   subtract the uptake rate from the portal
                                                                               load FIRST and then scale the whole
                                                                               difference by the sinusoid flow over the
                                                                               clearance divisor (the classic `a-b*c/d`
                                                                               vs `(a-b)*c/d` error, rung-67's shape),
                                                                               and
  SWAPPED    portal_load - uptake_rate / sinusoid_flow * clearance_divisor      DIVIDE the uptake rate by the sinusoid
                                                                               flow and then MULTIPLY by the clearance
                                                                               divisor — reordering the product-quotient
                                                                               (`a-b/c*d` instead of `a-b*c/d`, rung-73's
                                                                               shape),

which are exactly the mistakes a student makes (grouping the subtraction into the numerator, or dividing before
multiplying inside the subtracted term). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all
five always appear as options.

Distinctness and positivity: the tables keep all three guards — `portal_load > uptake_rate*sinusoid_flow/clearance_divisor`
(hepatic clearance positive), `portal_load > uptake_rate` (crossed positive), and `portal_load >
uptake_rate*clearance_divisor/sinusoid_flow` (swapped positive) — so every family member, including the headline hepatic
clearance `a-b*c/d`, is strictly positive; the five family values are pairwise distinct with a comfortable margin,
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PORTAL_LOAD, UPTAKE_RATE, SINUSOID_FLOW, CLEARANCE_DIVISOR) — a portal load presented to the liver, an uptake rate to
# multiply by the sinusoid flow, and a clearance divisor to divide that product by before subtracting it from the portal
# load, all plain positive numbers. The tables satisfy all three guards: portal_load > uptake_rate*sinusoid_flow/
# clearance_divisor (hepatic clearance > 0), portal_load > uptake_rate (crossed > 0), and portal_load >
# uptake_rate*clearance_divisor/sinusoid_flow (swapped > 0). The five family values are asserted pairwise-distinct below.
TABLES = [
    (30, 4, 6, 3),
    (40, 6, 5, 2),
    (36, 8, 3, 4),
    (48, 10, 6, 5),
    (60, 12, 4, 6),
    (45, 5, 8, 4),
    (54, 9, 6, 3),
]

# The option family (5 members), all built from the four observed quantities via -, *, and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "hepatic_clearance",
        "net hepatic clearance (the portal load minus the extraction load)",
        "portal_load - uptake_rate * sinusoid_flow / clearance_divisor",
    ),
    (
        "extraction_load",
        "the extraction load (uptake rate times the sinusoid flow, over the clearance divisor)",
        "uptake_rate * sinusoid_flow / clearance_divisor",
    ),
    (
        "uptake_product",
        "the uptake product (uptake rate times the sinusoid flow)",
        "uptake_rate * sinusoid_flow",
    ),
    (
        "crossed",
        "the portal load MINUS the uptake rate, all scaled by the sinusoid flow over the clearance divisor, not the extraction subtracted from the portal load (a wrong grouping)",
        "(portal_load - uptake_rate) * sinusoid_flow / clearance_divisor",
    ),
    (
        "swapped",
        "the uptake rate DIVIDED by the sinusoid flow then MULTIPLIED by the clearance divisor, subtracted from the portal load, the product-quotient reordered (a wrong grouping)",
        "portal_load - uptake_rate / sinusoid_flow * clearance_divisor",
    ),
]
QUERIED = ["hepatic_clearance", "extraction_load", "uptake_product"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(portal_load, uptake_rate, sinusoid_flow, clearance_divisor):
    # Operation order mirrors the ADJ programs exactly (the multiply and the divide bind before the subtract, per
    # precedence, and b*c/d evaluates left-to-right as (b*c)/d), so the Python option value and the engine result are
    # the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "hepatic_clearance": portal_load - uptake_rate * sinusoid_flow / clearance_divisor,
        "extraction_load": uptake_rate * sinusoid_flow / clearance_divisor,
        "uptake_product": uptake_rate * sinusoid_flow,
        "crossed": (portal_load - uptake_rate) * sinusoid_flow / clearance_divisor,
        "swapped": portal_load - uptake_rate / sinusoid_flow * clearance_divisor,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for portal_load, uptake_rate, sinusoid_flow, clearance_divisor in TABLES:
        assert (
            portal_load > 0
            and uptake_rate > 0
            and sinusoid_flow > 0
            and clearance_divisor > 0
        ), (portal_load, uptake_rate, sinusoid_flow, clearance_divisor)
        fv = family_values(portal_load, uptake_rate, sinusoid_flow, clearance_divisor)
        # The tables satisfy all three guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, portal_load, uptake_rate, sinusoid_flow, clearance_divisor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    portal_load,
                    uptake_rate,
                    sinusoid_flow,
                    clearance_divisor,
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
                portal_load,
                uptake_rate,
                sinusoid_flow,
                clearance_divisor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r83hcl-{idx + 1:02d}",
                "qtype": "hepatic_clearance_index",
                "stem": (
                    f"A liver is presented with a portal load of {num(portal_load)} and extracts at an uptake rate of "
                    f"{num(uptake_rate)} times a sinusoid flow of {num(sinusoid_flow)}, spread over a clearance divisor "
                    f"of {num(clearance_divisor)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe portal_load({num(portal_load)})\n"
                    f"observe uptake_rate({num(uptake_rate)})\n"
                    f"observe sinusoid_flow({num(sinusoid_flow)})\n"
                    f"observe clearance_divisor({num(clearance_divisor)})\n"
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
            "ADJ-LADDER rung 83 — hepatology hepatic-clearance index from four stated quantities (a NEW panel: "
            "hepatology / clearance). From a portal load, an uptake rate to multiply by the sinusoid flow, and a "
            "clearance divisor to divide that product by before subtracting, compute the hepatic clearance "
            "(portal_load - uptake_rate*sinusoid_flow/clearance_divisor), the extraction load "
            "(uptake_rate*sinusoid_flow/clearance_divisor), or the uptake product (uptake_rate*sinusoid_flow). Each "
            "item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine "
            "carries the arithmetic — a NEW shape, TERM MINUS A PRODUCT-QUOTIENT a-b*c/d (multiply b by c, divide by d, "
            "subtract from a, so a-b*c/d = a-((b*c)/d); distinct from rung-73 a-b/c*d = a-(b/c)*d and from rungs 34/35's "
            "sums/differences of two full products) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via -, *, and / — no "
            "constant leaks, and neither the extraction load, the uptake product, nor any clearance figure ever appears "
            "as a literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same four quantities, so the "
            "distractors are exactly the slips students make: subtracting the uptake rate from the portal load FIRST "
            "then scaling ((a-b)*c/d, rung-67's shape, a wrong grouping) and reordering the product-quotient to divide "
            "before multiplying (a-b/c*d, rung-73's shape, a wrong grouping). The core confusion tested is that "
            "a-b*c/d is a-((b*c)/d), not (a-b)*c/d and not a-b/c*d."
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
