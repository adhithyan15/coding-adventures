"""Generate rung-91 (nutrition / protein-delivery total) items.json for the ADJ-LADDER.

Rung 91 opens the **nutrition / protein-delivery** panel on the quantitative band — the arithmetic of a total protein
delivered. An `oral_protein` and an `enteral_protein` are ADDED as two base terms, an `infusion_rate` times
`infusion_hours` gives the parenteral load, and all three ADD into the total. **Two added terms plus a product**
introduces a genuinely NEW arithmetic family on the ladder: `a+b+c*d`, i.e. `((a+b)+(c*d))`.

This is genuinely new and opens a NEW family. The **binomial-product family** — rung-89's `(a+b)*(c-d)` and rung-90's
`(a-b)*(c-d)` — is now COMPLETE. Rung 91 moves to an **additive-chain-plus-product**: a three-way sum whose last term
is itself a product. No shipped shape ever summed TWO independent added terms and a product in one flat chain — the
earlier add/sub shapes attached at most one product or quotient to a single leading term (rung-79 `a*b+c/d`, rung-80
`a*b-c/d`, rung-83 `a-b*c/d`, rung-85 `a*b-c-d` subtracted two bare terms from ONE product). `a+b+c*d` is the ladder's
first **two-added-terms-plus-a-product**. The operator order matters: `a+b+c*d` is `((a+b)+(c*d))` (the product forms
first by precedence, then the flat sum), NOT `a+(b+c)*d` (folding the middle sum into the product) and NOT `a+b*c+d`
(multiplying the wrong pair and adding the last term bare) — the two distractors exploit exactly those confusions.

The setup: an `oral_protein`, an `enteral_protein`, an `infusion_rate`, and an `infusion_hours`. The total is:

  TOTAL PROTEIN     oral_protein + enteral_protein + infusion_rate * infusion_hours  [ two added terms plus a product ]
  ORAL-ENTERAL SUM  oral_protein + enteral_protein                                   [ the two base terms, before the product ]
  PARENTERAL LOAD   infusion_rate * infusion_hours                                   [ the infusion product, before the sum ]

The **total protein** is what makes this rung distinctive — it is the ladder's first **two-added-terms-plus-a-product**.
(The oral-enteral sum `a+b` and the parenteral load `c*d` ride alongside as component readouts, so the panel teaches the
whole calculation — exactly as rungs 47-90 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the oral and enteral protein into a base sum, the multiplication of the
infusion rate by the infusion hours into a parenteral load, then the flat addition of the three (the product forming
before the sum, so a+b+c*d evaluates as ((a+b)+(c*d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../89/90. This rung exercises the engine
across a **two-added-terms-plus-a-product** — the fact that `a+b+c*d` is `((a+b)+(c*d))` and NOT `a+(b+c)*d` and NOT
`a+b*c+d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `*` — **no
structural constants** — so no numeric literal appears in any program, and neither the oral-enteral sum, the parenteral
load, nor any total figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`oral_protein`, `enteral_protein`, `infusion_rate`, `infusion_hours`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    oral_protein + (enteral_protein + infusion_rate) * infusion_hours  fold the middle sum (enteral + rate)
                                                                                INTO the product instead of leaving the
                                                                                two terms added (the classic `a+b+c*d`
                                                                                vs `a+(b+c)*d` error), and
  SWAPPED    oral_protein + enteral_protein * infusion_rate + infusion_hours    multiply the WRONG pair (enteral × rate)
                                                                                and add the infusion hours bare
                                                                                (`a+b*c+d` instead of `a+b+c*d`),

which are exactly the mistakes a student makes (folding a neighbouring term into the product, or multiplying the wrong
adjacent pair). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: every quantity is a plain positive number >= 2, so every family member — a sum of positive
terms and positive products — is automatically strictly positive; the tables are chosen so the five family values are
pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (ORAL_PROTEIN, ENTERAL_PROTEIN, INFUSION_RATE, INFUSION_HOURS) — an oral protein and an enteral protein to add as two
# base terms, and an infusion rate times infusion hours to add as a parenteral load, all plain positive numbers >= 2.
# Every family member is a sum of positive terms / positive products, so positivity is automatic; the five family values
# are asserted pairwise-distinct below.
TABLES = [
    (2, 2, 2, 4),
    (2, 3, 2, 5),
    (2, 4, 2, 2),
    (2, 5, 2, 7),
    (2, 6, 2, 3),
    (2, 7, 3, 2),
    (3, 2, 3, 3),
]

# The option family (5 members), all built from the four observed quantities via + and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "total_protein",
        "total protein delivered (the oral and enteral protein plus the parenteral load)",
        "oral_protein + enteral_protein + infusion_rate * infusion_hours",
    ),
    (
        "oral_enteral_sum",
        "the oral-enteral sum (the oral protein plus the enteral protein, before adding the parenteral load)",
        "oral_protein + enteral_protein",
    ),
    (
        "parenteral_load",
        "the parenteral load (the infusion rate times the infusion hours, before adding the oral and enteral protein)",
        "infusion_rate * infusion_hours",
    ),
    (
        "crossed",
        "the oral protein plus the enteral protein and infusion rate together times the infusion hours, folding the middle sum into the product instead of leaving the two terms added (a wrong grouping)",
        "oral_protein + (enteral_protein + infusion_rate) * infusion_hours",
    ),
    (
        "swapped",
        "the oral protein plus the enteral protein times the infusion rate, plus the infusion hours added bare, multiplying the wrong pair (a wrong pairing)",
        "oral_protein + enteral_protein * infusion_rate + infusion_hours",
    ),
]
QUERIED = ["total_protein", "oral_enteral_sum", "parenteral_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(oral_protein, enteral_protein, infusion_rate, infusion_hours):
    # Operation order mirrors the ADJ programs exactly (the product forms first by precedence, then the flat sum, so
    # a+b+c*d evaluates as ((a+b)+(c*d))), so the Python option value and the engine result are the same IEEE-double
    # (well within the harness's 1e-9 match tolerance).
    return {
        "total_protein": oral_protein + enteral_protein + infusion_rate * infusion_hours,
        "oral_enteral_sum": oral_protein + enteral_protein,
        "parenteral_load": infusion_rate * infusion_hours,
        "crossed": oral_protein + (enteral_protein + infusion_rate) * infusion_hours,
        "swapped": oral_protein + enteral_protein * infusion_rate + infusion_hours,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for oral_protein, enteral_protein, infusion_rate, infusion_hours in TABLES:
        assert (
            oral_protein > 0
            and enteral_protein > 0
            and infusion_rate > 0
            and infusion_hours > 0
        ), (oral_protein, enteral_protein, infusion_rate, infusion_hours)
        fv = family_values(oral_protein, enteral_protein, infusion_rate, infusion_hours)
        # Every family member is a sum of positive terms / positive products, so every value is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, oral_protein, enteral_protein, infusion_rate, infusion_hours, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    oral_protein,
                    enteral_protein,
                    infusion_rate,
                    infusion_hours,
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
                oral_protein,
                enteral_protein,
                infusion_rate,
                infusion_hours,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r91prot-{idx + 1:02d}",
                "qtype": "protein_delivery_total",
                "stem": (
                    f"A nutrition order records an oral protein of {num(oral_protein)} plus an enteral protein of "
                    f"{num(enteral_protein)}, plus an infusion rate of {num(infusion_rate)} times infusion hours "
                    f"of {num(infusion_hours)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe oral_protein({num(oral_protein)})\n"
                    f"observe enteral_protein({num(enteral_protein)})\n"
                    f"observe infusion_rate({num(infusion_rate)})\n"
                    f"observe infusion_hours({num(infusion_hours)})\n"
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
            "ADJ-LADDER rung 91 — nutrition protein-delivery total from four stated quantities (a NEW panel: nutrition / "
            "protein-delivery). From an oral protein and an enteral protein to add as two base terms and an infusion rate "
            "times infusion hours to add as a parenteral load, compute the total protein "
            "(oral_protein+enteral_protein+infusion_rate*infusion_hours), the oral-enteral sum "
            "(oral_protein+enteral_protein), or the parenteral load (infusion_rate*infusion_hours). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW family, TWO ADDED TERMS PLUS A PRODUCT a+b+c*d (add a and b, multiply c by d, add the "
            "three, so a+b+c*d = ((a+b)+(c*d)); the binomial-product family (rung-89 (a+b)*(c-d), rung-90 (a-b)*(c-d)) is "
            "COMPLETE, and no prior add/sub shape summed TWO independent added terms and a product in one flat chain — "
            "e.g. rung-79 a*b+c/d, rung-83 a-b*c/d attached one term to a single product) — and the harness matches the "
            "scalar to the printed options. Contamination-safe: every figure is built only from the four observed "
            "quantities via + and * — no constant leaks, and neither the oral-enteral sum, the parenteral load, nor any "
            "total figure ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: folding the middle sum into the product "
            "(a+(b+c)*d, a wrong grouping) and multiplying the wrong pair with the last term added bare (a+b*c+d, a wrong "
            "pairing). The core confusion tested is that a+b+c*d is ((a+b)+(c*d)), not a+(b+c)*d and not a+b*c+d."
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
