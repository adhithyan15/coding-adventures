"""Generate rung-86 (rheumatology joint-fluid index) items.json for the ADJ-LADDER.

Rung 86 opens the **rheumatology / joint-fluid** panel on the quantitative band — the arithmetic of a net joint-fluid
volume. A `synovial_rate` is multiplied by a `joint_span` AND by an `effusion_factor` — a THREE-FACTOR product — and an
`aspiration_drain` is then SUBTRACTED. A three-factor product with a bare term subtracted introduces a genuinely NEW
arithmetic shape on the ladder: a **three-factor product MINUS a term** — `a*b*c-d`, i.e. `((a*b*c) - d)`.

This is genuinely new: every prior product shape on the ladder multiplied only TWO observed factors (rung-79 `a*b+c/d`,
rung-80 `a*b-c/d`, rung-85 `a*b-c-d`, and rungs 34/35's sums/differences of two two-factor products); rung-86 is the
first to multiply THREE observed factors together before subtracting. The operator order matters: `a*b*c-d` is
`((a*b*c) - d)` by precedence (all three multiplies bind first, left-to-right, then the subtraction applies), NOT
`a*b*(c-d)` (subtracting `d` from `c` INSIDE the third factor) and NOT `a*b-c*d` (multiplying only the first two factors
and subtracting the product of the other two) — the two distractors exploit exactly those confusions.

The setup: a `synovial_rate`, a `joint_span`, an `effusion_factor`, and an `aspiration_drain`. The joint fluid is:

  JOINT FLUID     synovial_rate * joint_span * effusion_factor - aspiration_drain   [ three-factor product minus a term ]
  GROSS PRODUCT   synovial_rate * joint_span * effusion_factor                      [ the three-factor product, before draining ]
  BASE PRODUCT    synovial_rate * joint_span                                        [ the first two factors, before the effusion factor ]

The **joint fluid** is what makes this rung distinctive — it is the ladder's first **three-factor product MINUS a term**.
(The gross product `a*b*c` and the base product `a*b` ride alongside as component readouts, so the panel teaches the
whole calculation — exactly as rungs 47-85 shipped their component sums/products/differences/ratios beside the headline
figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the synovial rate by the joint span by the effusion factor (three factors,
left-to-right), then the subtraction of the aspiration drain (the multiplies before the subtract) — and the harness reads
the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../84/85.
This rung exercises the engine across **a three-factor product minus a term** — the fact that `a*b*c-d` is `((a*b*c) - d)`
and NOT `a*b*(c-d)` and NOT `a*b-c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*` and `-` — **no
structural constants** — so no numeric literal appears in any program, and neither the gross product, the base product,
nor any fluid figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`synovial_rate`, `joint_span`, `effusion_factor`, `aspiration_drain`) so no numeral hides
inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    synovial_rate * joint_span * (effusion_factor - aspiration_drain)    subtract the aspiration drain from the
                                                                                  effusion factor INSIDE the third factor,
                                                                                  instead of after the whole product (the
                                                                                  classic `a*b*c-d` vs `a*b*(c-d)` error),
                                                                                  and
  SWAPPED    synovial_rate * joint_span - effusion_factor * aspiration_drain      multiply only the first TWO factors and
                                                                                  subtract the PRODUCT of the other two, a
                                                                                  wrong pairing (`a*b-c*d` instead of
                                                                                  `a*b*c-d`),

which are exactly the mistakes a student makes (folding the final subtraction into the last factor, or pairing the four
quantities into two products). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: the tables keep the guards — every factor `>= 2`, `effusion_factor > aspiration_drain` by
at least two (so the crossed factor `(c-d)` stays positive AND the crossed value never collapses onto the base product),
`synovial_rate*joint_span*effusion_factor > aspiration_drain` (joint fluid positive), and
`synovial_rate*joint_span > effusion_factor*aspiration_drain` (swapped positive) — so every family member, including the
headline joint fluid `a*b*c-d`, is strictly positive; the five family values are pairwise distinct with a comfortable
margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (SYNOVIAL_RATE, JOINT_SPAN, EFFUSION_FACTOR, ASPIRATION_DRAIN) — a synovial rate to multiply by the joint span and the
# effusion factor (a three-factor product), and an aspiration drain to subtract from that product, all plain positive
# numbers >= 2. The tables satisfy the guards: effusion_factor - aspiration_drain >= 2 (crossed factor > 0 and crossed !=
# base product), synovial_rate*joint_span*effusion_factor > aspiration_drain (joint fluid > 0), and
# synovial_rate*joint_span > effusion_factor*aspiration_drain (swapped > 0). The five family values are asserted
# pairwise-distinct below.
TABLES = [
    (3, 3, 4, 2),
    (4, 4, 5, 3),
    (5, 2, 4, 2),
    (5, 5, 6, 4),
    (7, 3, 5, 3),
    (6, 7, 7, 5),
    (4, 5, 6, 3),
]

# The option family (5 members), all built from the four observed quantities via * and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "joint_fluid",
        "net joint fluid (the three-factor product minus the aspiration drain)",
        "synovial_rate * joint_span * effusion_factor - aspiration_drain",
    ),
    (
        "gross_product",
        "the gross product (the synovial rate times the joint span times the effusion factor, before draining)",
        "synovial_rate * joint_span * effusion_factor",
    ),
    (
        "base_product",
        "the base product (the synovial rate times the joint span, before the effusion factor)",
        "synovial_rate * joint_span",
    ),
    (
        "crossed",
        "the synovial rate times the joint span, all scaled by the effusion factor MINUS the aspiration drain, with the drain taken off the effusion factor inside the third factor instead of off the whole product (a wrong grouping)",
        "synovial_rate * joint_span * (effusion_factor - aspiration_drain)",
    ),
    (
        "swapped",
        "the synovial rate times the joint span, MINUS the effusion factor times the aspiration drain, pairing the four quantities into two products instead of one three-factor product (a wrong pairing)",
        "synovial_rate * joint_span - effusion_factor * aspiration_drain",
    ),
]
QUERIED = ["joint_fluid", "gross_product", "base_product"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(synovial_rate, joint_span, effusion_factor, aspiration_drain):
    # Operation order mirrors the ADJ programs exactly (all three multiplies bind first, left-to-right, then the
    # subtraction applies, so a*b*c-d evaluates as ((a*b*c)-d)), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "joint_fluid": synovial_rate * joint_span * effusion_factor - aspiration_drain,
        "gross_product": synovial_rate * joint_span * effusion_factor,
        "base_product": synovial_rate * joint_span,
        "crossed": synovial_rate * joint_span * (effusion_factor - aspiration_drain),
        "swapped": synovial_rate * joint_span - effusion_factor * aspiration_drain,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for synovial_rate, joint_span, effusion_factor, aspiration_drain in TABLES:
        assert (
            synovial_rate > 0
            and joint_span > 0
            and effusion_factor > 0
            and aspiration_drain > 0
        ), (synovial_rate, joint_span, effusion_factor, aspiration_drain)
        fv = family_values(synovial_rate, joint_span, effusion_factor, aspiration_drain)
        # The tables satisfy the guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, synovial_rate, joint_span, effusion_factor, aspiration_drain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    synovial_rate,
                    joint_span,
                    effusion_factor,
                    aspiration_drain,
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
                synovial_rate,
                joint_span,
                effusion_factor,
                aspiration_drain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r86jfl-{idx + 1:02d}",
                "qtype": "joint_fluid_index",
                "stem": (
                    f"A joint is filled at a synovial rate of {num(synovial_rate)} times a joint span of "
                    f"{num(joint_span)} times an effusion factor of {num(effusion_factor)}, with an aspiration drain of "
                    f"{num(aspiration_drain)} carried off. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe synovial_rate({num(synovial_rate)})\n"
                    f"observe joint_span({num(joint_span)})\n"
                    f"observe effusion_factor({num(effusion_factor)})\n"
                    f"observe aspiration_drain({num(aspiration_drain)})\n"
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
            "ADJ-LADDER rung 86 — rheumatology joint-fluid index from four stated quantities (a NEW panel: "
            "rheumatology / joint-fluid). From a synovial rate to multiply by the joint span and the effusion factor (a "
            "three-factor product) and an aspiration drain to subtract, compute the joint fluid "
            "(synovial_rate*joint_span*effusion_factor-aspiration_drain), the gross product "
            "(synovial_rate*joint_span*effusion_factor), or the base product (synovial_rate*joint_span). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW shape, THREE-FACTOR PRODUCT MINUS A TERM a*b*c-d (multiply a by b by c, subtract d, so "
            "a*b*c-d = ((a*b*c)-d); distinct from every prior product shape, which multiplied only two observed factors, "
            "e.g. rung-80 a*b-c/d and rung-85 a*b-c-d) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via * and - — no constant "
            "leaks, and neither the gross product, the base product, nor any fluid figure ever appears as a literal (each "
            "is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a "
            "variable name. The five options are a family over the same four quantities, so the distractors are exactly "
            "the slips students make: taking the aspiration drain off the effusion factor inside the third factor "
            "(a*b*(c-d), a wrong grouping) and pairing the four quantities into two products (a*b-c*d, a wrong pairing). "
            "The core confusion tested is that a*b*c-d is ((a*b*c)-d), not a*b*(c-d) and not a*b-c*d."
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
