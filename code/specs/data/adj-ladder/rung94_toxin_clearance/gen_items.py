"""Generate rung-94 (toxicology / combined-clearance total) items.json for the ADJ-LADDER.

Rung 94 opens the **toxicology / combined-clearance** panel on the quantitative band — the arithmetic of a toxin's total
clearance capacity. A `renal_fraction` and a `hepatic_fraction` ADD into the organ sum, a `perfusion_rate` and a
`filtration_rate` ADD into the flow sum, and the two sums MULTIPLY into the combined clearance. A **product of two sums**
introduces a genuinely NEW arithmetic family on the ladder: `(a+b)*(c+d)`, i.e. `((a+b)*(c+d))`.

This is genuinely new and COMPLETES the binomial-product family. Rung-89 shipped `(a+b)*(c-d)` (a sum times a
difference) and rung-90 shipped `(a-b)*(c-d)` (a difference times a difference); the **sum-times-sum** corner `(a+b)*(c+d)`
was never shipped. Rung 94 fills it — the last all-additive member of "a binomial times a binomial". No prior shape
multiplied TWO independent sums: rung-84 `(a+b)*c-d` multiplied ONE sum by a bare factor, rung-34's sum-of-two-products
`a*b+c*d` ADDED two products (it never multiplied two sums). The operator order matters: `(a+b)*(c+d)` is
`((a+b)*(c+d))` (each sum forms first inside its parentheses, then the two are multiplied), NOT `a*c+b*d` (the classic
FOIL slip — multiplying the parallel pairs and dropping the cross terms) and NOT `a*d+b*c` (multiplying the diagonal
pairs) — the two distractors exploit exactly those confusions.

The setup: a `renal_fraction`, a `hepatic_fraction`, a `perfusion_rate`, and a `filtration_rate`. The total is:

  COMBINED CLEARANCE   (renal_fraction + hepatic_fraction) * (perfusion_rate + filtration_rate)  [ a product of two sums ]
  ORGAN SUM            renal_fraction + hepatic_fraction                                          [ the left sum, before the product ]
  FLOW SUM             perfusion_rate + filtration_rate                                           [ the right sum, before the product ]

The **combined clearance** is what makes this rung distinctive — it is the ladder's first **product of two sums** and the
sum-times-sum corner that completes the binomial-product family. (The organ sum `a+b` and the flow sum `c+d` ride
alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-93 shipped their
component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the renal and hepatic fractions into the organ sum, the addition of the
perfusion and filtration rates into the flow sum, then the multiplication of the two sums (each sum forming inside its
parentheses before the product, so (a+b)*(c+d) evaluates as ((a+b)*(c+d))) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../92/93. This rung exercises
the engine across a **product of two sums** — the fact that `(a+b)*(c+d)` is `((a+b)*(c+d))` and NOT `a*c+b*d` and NOT
`a*d+b*c` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `*` — **no
structural constants** — so no numeric literal appears in any program, and neither the organ sum, the flow sum, nor any
total figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`renal_fraction`, `hepatic_fraction`, `perfusion_rate`, `filtration_rate`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    renal_fraction * perfusion_rate + hepatic_fraction * filtration_rate  multiply the PARALLEL pairs and drop
                                                                                   the cross terms (the classic FOIL error,
                                                                                   `(a+b)*(c+d)` vs `a*c+b*d`), and
  SWAPPED    renal_fraction * filtration_rate + hepatic_fraction * perfusion_rate  multiply the DIAGONAL pairs (the other
                                                                                   partial product, `a*d+b*c` instead of
                                                                                   `(a+b)*(c+d)`),

which are exactly the mistakes a student makes (multiplying binomials "straight across" and forgetting the cross terms,
or pairing the wrong factors). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: every quantity is a plain positive number >= 2, so every family member — a product of
positive sums / a sum of positive products — is automatically strictly positive; the tables are chosen so the five family
values are pairwise distinct with a comfortable margin (they avoid renal_fraction == hepatic_fraction and
perfusion_rate == filtration_rate, either of which would collide the two partial-product slips, and
renal_fraction+hepatic_fraction == perfusion_rate+filtration_rate, which would collide the two sums), asserted at build.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (RENAL_FRACTION, HEPATIC_FRACTION, PERFUSION_RATE, FILTRATION_RATE) — a renal fraction and a hepatic fraction to add
# into the organ sum, and a perfusion rate and a filtration rate to add into the flow sum, all plain positive numbers
# >= 2. Every family member is a product of positive sums / a sum of positive products, so positivity is automatic; the
# five family values are asserted pairwise-distinct below. The tables keep renal_fraction != hepatic_fraction and
# perfusion_rate != filtration_rate (else the crossed and swapped partial products collide) and
# renal_fraction+hepatic_fraction != perfusion_rate+filtration_rate (else the two sums collide).
TABLES = [
    (2, 3, 4, 5),
    (2, 4, 3, 5),
    (3, 5, 2, 4),
    (2, 5, 3, 6),
    (4, 2, 6, 3),
    (3, 2, 6, 4),
    (2, 6, 4, 3),
]

# The option family (5 members), all built from the four observed quantities via + and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "combined_clearance",
        "combined clearance capacity (the organ sum times the flow sum)",
        "(renal_fraction + hepatic_fraction) * (perfusion_rate + filtration_rate)",
    ),
    (
        "organ_sum",
        "the organ sum (the renal fraction plus the hepatic fraction, before multiplying by the flow sum)",
        "renal_fraction + hepatic_fraction",
    ),
    (
        "flow_sum",
        "the flow sum (the perfusion rate plus the filtration rate, before multiplying by the organ sum)",
        "perfusion_rate + filtration_rate",
    ),
    (
        "crossed",
        "the renal fraction times the perfusion rate plus the hepatic fraction times the filtration rate, multiplying the parallel pairs and dropping the cross terms (a wrong grouping)",
        "renal_fraction * perfusion_rate + hepatic_fraction * filtration_rate",
    ),
    (
        "swapped",
        "the renal fraction times the filtration rate plus the hepatic fraction times the perfusion rate, multiplying the diagonal pairs (a wrong pairing)",
        "renal_fraction * filtration_rate + hepatic_fraction * perfusion_rate",
    ),
]
QUERIED = ["combined_clearance", "organ_sum", "flow_sum"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(renal_fraction, hepatic_fraction, perfusion_rate, filtration_rate):
    # Operation order mirrors the ADJ programs exactly (each sum forms inside its parentheses first, then the two are
    # multiplied, so (a+b)*(c+d) evaluates as ((a+b)*(c+d))), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "combined_clearance": (renal_fraction + hepatic_fraction) * (perfusion_rate + filtration_rate),
        "organ_sum": renal_fraction + hepatic_fraction,
        "flow_sum": perfusion_rate + filtration_rate,
        "crossed": renal_fraction * perfusion_rate + hepatic_fraction * filtration_rate,
        "swapped": renal_fraction * filtration_rate + hepatic_fraction * perfusion_rate,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for renal_fraction, hepatic_fraction, perfusion_rate, filtration_rate in TABLES:
        assert (
            renal_fraction > 0
            and hepatic_fraction > 0
            and perfusion_rate > 0
            and filtration_rate > 0
        ), (renal_fraction, hepatic_fraction, perfusion_rate, filtration_rate)
        fv = family_values(renal_fraction, hepatic_fraction, perfusion_rate, filtration_rate)
        # Every family member is a product of positive sums / a sum of positive products, so every value is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, renal_fraction, hepatic_fraction, perfusion_rate, filtration_rate, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    renal_fraction,
                    hepatic_fraction,
                    perfusion_rate,
                    filtration_rate,
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
                renal_fraction,
                hepatic_fraction,
                perfusion_rate,
                filtration_rate,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r94tox-{idx + 1:02d}",
                "qtype": "toxin_clearance_total",
                "stem": (
                    f"A clearance study records a renal fraction of {num(renal_fraction)} plus a hepatic fraction of "
                    f"{num(hepatic_fraction)}, all times a perfusion rate of {num(perfusion_rate)} plus a filtration "
                    f"rate of {num(filtration_rate)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe renal_fraction({num(renal_fraction)})\n"
                    f"observe hepatic_fraction({num(hepatic_fraction)})\n"
                    f"observe perfusion_rate({num(perfusion_rate)})\n"
                    f"observe filtration_rate({num(filtration_rate)})\n"
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
            "ADJ-LADDER rung 94 — toxin combined-clearance total from four stated quantities (a NEW panel: toxicology / "
            "combined-clearance). From a renal fraction and a hepatic fraction that add into the organ sum and a "
            "perfusion rate and a filtration rate that add into the flow sum, compute the combined clearance "
            "((renal_fraction+hepatic_fraction)*(perfusion_rate+filtration_rate)), the organ sum "
            "(renal_fraction+hepatic_fraction), or the flow sum (perfusion_rate+filtration_rate). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, A PRODUCT OF TWO SUMS (a+b)*(c+d) (add a and b, add c and d, multiply the two "
            "sums, so (a+b)*(c+d) = ((a+b)*(c+d)); this COMPLETES the binomial-product family — rung-89 (a+b)*(c-d) was "
            "sum-times-difference, rung-90 (a-b)*(c-d) difference-times-difference, and the sum-times-sum corner was never "
            "shipped; no prior shape multiplied TWO independent sums, e.g. rung-84 (a+b)*c-d multiplied one sum by a bare "
            "factor and rung-34 a*b+c*d added two products) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every figure is built only from the four observed quantities via + and * — no constant "
            "leaks, and neither the organ sum, the flow sum, nor any total figure ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make: multiplying the parallel pairs and dropping the cross terms (a*c+b*d, the classic FOIL error) "
            "and multiplying the diagonal pairs (a*d+b*c, a wrong pairing). The core confusion tested is that (a+b)*(c+d) "
            "is ((a+b)*(c+d)), not a*c+b*d and not a*d+b*c."
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
