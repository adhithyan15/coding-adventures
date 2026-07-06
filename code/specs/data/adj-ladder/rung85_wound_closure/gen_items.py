"""Generate rung-85 (dermatology wound-closure index) items.json for the ADJ-LADDER.

Rung 85 opens the **dermatology / wound-healing** panel on the quantitative band — the arithmetic of a net wound
closure. A wound gains closure at a `granulation_rate` times a `contraction_span` (a PRODUCT), and TWO separate losses
are then taken off it — an `exudate_loss` and a `debridement_loss`, each SUBTRACTED as its own bare term. A product with
two separate terms subtracted introduces a genuinely NEW arithmetic shape on the ladder: a **product MINUS two bare
terms** — `a*b-c-d`, i.e. `((a*b) - c) - d`.

This is genuinely new: rung-80 was `a*b-c/d` (`(a*b) - (c/d)`, the product minus a QUOTIENT), rung-79 was `a*b+c/d` (the
product PLUS a quotient), and rungs 34/35 were sums/differences of two FULL products; here a plain product has two
SEPARATE bare terms subtracted from it, one after the other. The operator order matters: `a*b-c-d` is `((a*b) - c) - d`
by precedence (the multiply binds first, then the two subtractions apply left-to-right), NOT `a*b-(c-d)` (subtracting the
DIFFERENCE `c-d`, which flips the sign of `d`) and NOT `a*(b-c)-d` (folding the first subtraction INTO the product) — the
two distractors exploit exactly those confusions.

The setup: a `granulation_rate`, a `contraction_span`, an `exudate_loss`, and a `debridement_loss`. The wound closure is:

  WOUND CLOSURE    granulation_rate * contraction_span - exudate_loss - debridement_loss   [ product minus two bare terms ]
  GROSS PRODUCT    granulation_rate * contraction_span                                     [ the closure product, before the losses ]
  TOTAL LOSSES     exudate_loss + debridement_loss                                         [ the two losses summed ]

The **wound closure** is what makes this rung distinctive — it is the ladder's first **product MINUS two separate bare
terms**. (The gross product `a*b` and the total losses `c+d` ride alongside as component readouts, so the panel teaches
the whole calculation — exactly as rungs 47-84 shipped their component sums/products/differences/ratios beside the
headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the granulation rate by the contraction span, then the subtraction of the
exudate loss, then the subtraction of the debridement loss (the multiply before the two subtracts, left-to-right) — and
the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as
rungs 8/16/.../83/84. This rung exercises the engine across **a product minus two bare terms** — the fact that
`a*b-c-d` is `((a*b) - c) - d` and NOT `a*b-(c-d)` and NOT `a*(b-c)-d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*`, `-`, and `+`
— **no structural constants** — so no numeric literal appears in any program, and neither the gross product, the total
losses, nor any closure figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`granulation_rate`, `contraction_span`, `exudate_loss`, `debridement_loss`)
so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    granulation_rate * contraction_span - (exudate_loss - debridement_loss)   subtract the DIFFERENCE of the two
                                                                                       losses instead of both losses,
                                                                                       flipping the sign of the
                                                                                       debridement loss (the classic
                                                                                       `a*b-c-d` vs `a*b-(c-d)` error),
                                                                                       and
  SWAPPED    granulation_rate * (contraction_span - exudate_loss) - debridement_loss   fold the first subtraction INTO
                                                                                       the product, subtracting the
                                                                                       exudate loss from the contraction
                                                                                       span before multiplying
                                                                                       (`a*(b-c)-d` instead of `a*b-c-d`,
                                                                                       a wrong grouping),

which are exactly the mistakes a student makes (grouping the two losses into a difference, or folding a subtraction into
the product). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: the tables keep the guards — `granulation_rate >= 2` (so the swapped value `a*b-a*c-d`
differs from the headline `a*b-c-d` by `c*(a-1) != 0`), `contraction_span > exudate_loss` (so the swapped group
`(b-c)` stays positive), `granulation_rate*contraction_span > exudate_loss + debridement_loss` (wound closure positive),
`granulation_rate*(contraction_span - exudate_loss) > debridement_loss` (swapped positive), and `exudate_loss !=
debridement_loss` (so the crossed value `a*b-c+d` never collapses onto the gross product `a*b`) — so every family member,
including the headline wound closure `a*b-c-d`, is strictly positive; the five family values are pairwise distinct with a
comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (GRANULATION_RATE, CONTRACTION_SPAN, EXUDATE_LOSS, DEBRIDEMENT_LOSS) — a granulation rate to multiply by the
# contraction span (the closure product), and two losses to subtract one after the other, all plain positive numbers.
# The tables satisfy the guards: granulation_rate >= 2 (swapped != headline), contraction_span > exudate_loss (swapped
# group > 0), granulation_rate*contraction_span > exudate_loss+debridement_loss (wound closure > 0),
# granulation_rate*(contraction_span-exudate_loss) > debridement_loss (swapped > 0), and exudate_loss != debridement_loss
# (crossed != gross product). The five family values are asserted pairwise-distinct below.
TABLES = [
    (5, 4, 3, 2),
    (6, 5, 4, 3),
    (4, 6, 2, 5),
    (7, 3, 2, 4),
    (8, 7, 6, 3),
    (3, 8, 5, 4),
    (6, 6, 4, 5),
]

# The option family (5 members), all built from the four observed quantities via *, -, and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "wound_closure",
        "net wound closure (the closure product minus both losses)",
        "granulation_rate * contraction_span - exudate_loss - debridement_loss",
    ),
    (
        "gross_product",
        "the gross product (the granulation rate times the contraction span, before the losses)",
        "granulation_rate * contraction_span",
    ),
    (
        "total_losses",
        "the total losses (the exudate loss plus the debridement loss)",
        "exudate_loss + debridement_loss",
    ),
    (
        "crossed",
        "the closure product minus the DIFFERENCE of the two losses, subtracting only the exudate loss net of the debridement loss instead of both losses (a wrong grouping)",
        "granulation_rate * contraction_span - (exudate_loss - debridement_loss)",
    ),
    (
        "swapped",
        "the exudate loss subtracted from the contraction span BEFORE multiplying by the granulation rate, then the debridement loss taken off, the first subtraction folded into the product (a wrong grouping)",
        "granulation_rate * (contraction_span - exudate_loss) - debridement_loss",
    ),
]
QUERIED = ["wound_closure", "gross_product", "total_losses"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(granulation_rate, contraction_span, exudate_loss, debridement_loss):
    # Operation order mirrors the ADJ programs exactly (the multiply binds first, then the two subtractions apply
    # left-to-right, so a*b-c-d evaluates as ((a*b)-c)-d), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "wound_closure": granulation_rate * contraction_span - exudate_loss - debridement_loss,
        "gross_product": granulation_rate * contraction_span,
        "total_losses": exudate_loss + debridement_loss,
        "crossed": granulation_rate * contraction_span - (exudate_loss - debridement_loss),
        "swapped": granulation_rate * (contraction_span - exudate_loss) - debridement_loss,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for granulation_rate, contraction_span, exudate_loss, debridement_loss in TABLES:
        assert (
            granulation_rate > 0
            and contraction_span > 0
            and exudate_loss > 0
            and debridement_loss > 0
        ), (granulation_rate, contraction_span, exudate_loss, debridement_loss)
        fv = family_values(granulation_rate, contraction_span, exudate_loss, debridement_loss)
        # The tables satisfy the guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, granulation_rate, contraction_span, exudate_loss, debridement_loss, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    granulation_rate,
                    contraction_span,
                    exudate_loss,
                    debridement_loss,
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
                granulation_rate,
                contraction_span,
                exudate_loss,
                debridement_loss,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r85wcl-{idx + 1:02d}",
                "qtype": "wound_closure_index",
                "stem": (
                    f"A wound gains closure at a granulation rate of {num(granulation_rate)} times a contraction span of "
                    f"{num(contraction_span)}, with an exudate loss of {num(exudate_loss)} and a debridement loss of "
                    f"{num(debridement_loss)} each taken off. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe granulation_rate({num(granulation_rate)})\n"
                    f"observe contraction_span({num(contraction_span)})\n"
                    f"observe exudate_loss({num(exudate_loss)})\n"
                    f"observe debridement_loss({num(debridement_loss)})\n"
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
            "ADJ-LADDER rung 85 — dermatology wound-closure index from four stated quantities (a NEW panel: "
            "dermatology / wound-healing). From a granulation rate to multiply by the contraction span (the closure "
            "product) and two losses to subtract, compute the wound closure "
            "(granulation_rate*contraction_span-exudate_loss-debridement_loss), the gross product "
            "(granulation_rate*contraction_span), or the total losses (exudate_loss+debridement_loss). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW shape, PRODUCT MINUS TWO BARE TERMS a*b-c-d (multiply a by b, subtract c, subtract d, so "
            "a*b-c-d = ((a*b)-c)-d; distinct from rung-80 a*b-c/d = (a*b)-(c/d) and from rungs 34/35's sums/differences "
            "of two full products) — and the harness matches the scalar to the printed options. Contamination-safe: "
            "every index is built only from the four observed quantities via *, -, and + — no constant leaks, and "
            "neither the gross product, the total losses, nor any closure figure ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same four quantities, so the distractors are exactly the slips "
            "students make: subtracting the DIFFERENCE of the two losses instead of both (a*b-(c-d), flipping the sign "
            "of the debridement loss, a wrong grouping) and folding the first subtraction into the product "
            "(a*(b-c)-d, a wrong grouping). The core confusion tested is that a*b-c-d is ((a*b)-c)-d, not a*b-(c-d) and "
            "not a*(b-c)-d."
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
