"""Generate rung-97 (obstetrics / amniotic-fluid-index load) items.json for the ADJ-LADDER.

Rung 97 opens the **obstetrics / amniotic-fluid-index** panel on the quantitative band — the arithmetic of an amniotic
fluid load index. An `anterior_pocket` PLUS a `posterior_pocket` gives the two-quadrant pocket sum (how much fluid the two
sampled quadrants hold together), that pocket sum MULTIPLIES the `probe_factor` (the ultrasound depth-scaling factor) into
the scaled pockets, and a `residual_volume` is ADDED on to give the total amniotic load. A **binomial sum times a bare
factor, plus a term** introduces a genuinely NEW arithmetic family on the ladder: `(a+b)*c+d`, i.e. `(((a+b)*c)+d)`.

This is genuinely new. It is the **sign-flip sibling of rung-84** `(a+b)*c-d` (a binomial sum times a factor MINUS a term)
and completes the (a±b)*c±d quartet alongside rung-96 `(a-b)*c+d`: rung-84 summed and subtracted, rung-96 took the
difference and added, rung-97 sums and adds. No prior shape multiplied a binomial SUM by a bare factor and ADDED a term:
rung-84 subtracted the trailing term, rung-68 `(a+b)*c/d` divided the sum-product by a term (not add), rung-94
`(a+b)*(c+d)` multiplied a sum by a SUM (both binomials), rung-34 `a*b+c*d` added two products. The operator order
matters: `(a+b)*c+d` is `(((a+b)*c)+d)` (the sum forms first inside its parentheses, then it is multiplied by the factor,
then the term is added), NOT `a+b*c+d` (dropping the parentheses so the factor multiplies only the posterior pocket) and
NOT `(a+b)+c*d` (adding the bare pocket sum and multiplying the factor into the residual volume instead) — the two
distractors exploit exactly those confusions.

The setup: an `anterior_pocket`, a `posterior_pocket`, a `probe_factor`, and a `residual_volume`. The total is:

  AMNIOTIC LOAD       (anterior_pocket + posterior_pocket) * probe_factor + residual_volume  [ a sum times a factor, plus a term ]
  POCKET SUM          anterior_pocket + posterior_pocket                                     [ the sum, before the product ]
  SCALED POCKETS      (anterior_pocket + posterior_pocket) * probe_factor                    [ the sum times the factor, before adding the term ]

The **amniotic load** is what makes this rung distinctive — it is the ladder's first **binomial SUM times a bare factor,
plus a trailing term**, the sign-flip sibling of rung-84's `(a+b)*c-d` and the last corner of the (a±b)*c±d quartet
(84 (a+b)*c-d, 96 (a-b)*c+d, 97 (a+b)*c+d, with (a-b)*c-d still open). (The pocket sum `a+b` and the scaled pockets `(a+b)*c`
ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-96 shipped their
component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the two quadrant pockets into the pocket sum, the multiplication of that sum by
the probe factor into the scaled pockets, then the addition of the residual volume (the sum forming inside its parentheses
before the product, and the product forming before the trailing addition, so (a+b)*c+d evaluates as (((a+b)*c)+d)) — and
the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../95/96. This rung exercises the engine across a **sum times a factor, plus a term** — the fact that `(a+b)*c+d` is
`(((a+b)*c)+d)` and NOT `a+b*c+d` and NOT `(a+b)+c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `*` — **no
structural constants** — so no numeric literal appears in any program, and neither the pocket sum, the scaled pockets, nor
any load figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`anterior_pocket`, `posterior_pocket`, `probe_factor`, `residual_volume`) so no numeral hides
inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    anterior_pocket + posterior_pocket * probe_factor + residual_volume  drop the parentheses so the probe factor
                                                                                 multiplies only the posterior pocket and
                                                                                 the bare anterior pocket and residual
                                                                                 volume are left added (the classic
                                                                                 `(a+b)*c+d` vs `a+b*c+d` error), and
  SWAPPED    (anterior_pocket + posterior_pocket) + probe_factor * residual_volume  add the bare pocket sum and multiply the
                                                                                 probe factor into the residual volume
                                                                                 instead, mispairing which terms are added
                                                                                 and which are multiplied (`(a+b)+c*d`
                                                                                 instead of `(a+b)*c+d`),

which are exactly the mistakes a student makes (dropping the parentheses around the binomial before applying the factor,
or mispairing the factor with the trailing term). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness and positivity: every observed quantity is a plain positive number >= 2, so every family member — a product
of a positive sum and a positive factor plus a positive term, a positive sum, or a positive combination — is automatically
strictly positive with no subtraction anywhere. The tables are chosen so the five family values are pairwise distinct with
a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (ANTERIOR_POCKET, POSTERIOR_POCKET, PROBE_FACTOR, RESIDUAL_VOLUME) — an anterior pocket plus a posterior pocket for the
# two-quadrant pocket sum, a probe factor that scales it, and a residual volume added on, all plain positive numbers >= 2.
# Every family member is a product of a positive sum and a positive factor plus a positive term / a positive sum / a
# positive combination, so positivity is automatic (no subtraction anywhere); the five family values are asserted
# pairwise-distinct below.
TABLES = [
    (3, 2, 4, 6),
    (4, 3, 2, 5),
    (2, 5, 3, 6),
    (5, 2, 3, 4),
    (4, 2, 5, 3),
    (6, 3, 2, 5),
    (2, 6, 4, 3),
]

# The option family (5 members), all built from the four observed quantities via + and *. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "amniotic_load",
        "total amniotic load (the pocket sum times the probe factor, plus the residual volume)",
        "(anterior_pocket + posterior_pocket) * probe_factor + residual_volume",
    ),
    (
        "pocket_sum",
        "the pocket sum (the anterior pocket plus the posterior pocket, before multiplying by the probe factor)",
        "anterior_pocket + posterior_pocket",
    ),
    (
        "scaled_pockets",
        "the scaled pockets (the pocket sum times the probe factor, before adding the residual volume)",
        "(anterior_pocket + posterior_pocket) * probe_factor",
    ),
    (
        "crossed",
        "the anterior pocket plus the posterior pocket times the probe factor, plus the residual volume, dropping the parentheses so the probe factor multiplies only the posterior pocket (a wrong grouping)",
        "anterior_pocket + posterior_pocket * probe_factor + residual_volume",
    ),
    (
        "swapped",
        "the pocket sum plus the probe factor times the residual volume, adding the bare pocket sum and multiplying the probe factor into the residual volume instead (a wrong pairing)",
        "(anterior_pocket + posterior_pocket) + probe_factor * residual_volume",
    ),
]
QUERIED = ["amniotic_load", "pocket_sum", "scaled_pockets"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(anterior_pocket, posterior_pocket, probe_factor, residual_volume):
    # Operation order mirrors the ADJ programs exactly (the sum forms inside its parentheses first, then it is multiplied
    # by the probe factor, then the residual volume is added, so (a+b)*c+d evaluates as (((a+b)*c)+d)), so the Python
    # option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "amniotic_load": (anterior_pocket + posterior_pocket) * probe_factor + residual_volume,
        "pocket_sum": anterior_pocket + posterior_pocket,
        "scaled_pockets": (anterior_pocket + posterior_pocket) * probe_factor,
        "crossed": anterior_pocket + posterior_pocket * probe_factor + residual_volume,
        "swapped": (anterior_pocket + posterior_pocket) + probe_factor * residual_volume,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for anterior_pocket, posterior_pocket, probe_factor, residual_volume in TABLES:
        # Every observed quantity is a plain positive number >= 2, so every family member is strictly positive (no
        # subtraction anywhere).
        assert (
            anterior_pocket >= 2
            and posterior_pocket >= 2
            and probe_factor >= 2
            and residual_volume >= 2
        ), (anterior_pocket, posterior_pocket, probe_factor, residual_volume)
        fv = family_values(anterior_pocket, posterior_pocket, probe_factor, residual_volume)
        # Every family member is a product of a positive sum and a positive factor plus a positive term / a positive sum /
        # a positive combination, so every value is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, anterior_pocket, posterior_pocket, probe_factor, residual_volume, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    anterior_pocket,
                    posterior_pocket,
                    probe_factor,
                    residual_volume,
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
                anterior_pocket,
                posterior_pocket,
                probe_factor,
                residual_volume,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r97amnio-{idx + 1:02d}",
                "qtype": "amniotic_fluid_load",
                "stem": (
                    f"An amniotic-fluid study records an anterior pocket of {num(anterior_pocket)} plus a posterior "
                    f"pocket of {num(posterior_pocket)}, times a probe factor of {num(probe_factor)}, plus a residual "
                    f"volume of {num(residual_volume)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe anterior_pocket({num(anterior_pocket)})\n"
                    f"observe posterior_pocket({num(posterior_pocket)})\n"
                    f"observe probe_factor({num(probe_factor)})\n"
                    f"observe residual_volume({num(residual_volume)})\n"
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
            "ADJ-LADDER rung 97 — amniotic fluid load from four stated quantities (a NEW panel: obstetrics / "
            "amniotic-fluid-index). From an anterior pocket plus a posterior pocket for the pocket sum, a probe factor "
            "that scales it, and a residual volume added on, compute the amniotic load "
            "((anterior_pocket+posterior_pocket)*probe_factor+residual_volume), the pocket sum "
            "(anterior_pocket+posterior_pocket), or the scaled pockets "
            "((anterior_pocket+posterior_pocket)*probe_factor). Each item is a compute_dimensioned program (observe the "
            "four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A SUM TIMES A "
            "FACTOR, PLUS A TERM (a+b)*c+d (add a and b, multiply the sum by c, add d, so (a+b)*c+d = (((a+b)*c)+d); this "
            "is the SIGN-FLIP SIBLING of rung-84 (a+b)*c-d, which summed the inner terms and SUBTRACTED the trailing term "
            "— rung-97 sums and ADDS, completing the (a±b)*c±d quartet alongside rung-96 (a-b)*c+d; no prior shape "
            "multiplied a binomial SUM by a bare factor and added a term, e.g. rung-68 (a+b)*c/d divided a sum-product by a "
            "term and rung-94 (a+b)*(c+d) multiplied a sum by a sum) — and the harness matches the scalar to the printed "
            "options. Contamination-safe: every figure is built only from the four observed quantities via + and * — no "
            "constant leaks, and neither the pocket sum, the scaled pockets, nor any load figure ever appears as a literal "
            "(each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a "
            "variable name. The five options are a family over the same four quantities, so the distractors are exactly "
            "the slips students make: dropping the parentheses so the probe factor multiplies only the posterior pocket "
            "(a+b*c+d, a wrong grouping) and adding the bare pocket sum while multiplying the probe factor into the "
            "residual volume ((a+b)+c*d, a wrong pairing). The core confusion tested is that (a+b)*c+d is (((a+b)*c)+d), "
            "not a+b*c+d and not (a+b)+c*d. Every observed quantity is at least two, so all figures stay strictly positive."
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
