"""Generate rung-59 (reconstituted drug concentration) items.json for the ADJ-LADDER.

Rung 59 opens the **oncology infusion pharmacy** panel on the quantitative band — the arithmetic of preparing a
cytotoxic infusion. A drug is drawn from several vials into a bag, but the bag's labelled volume overstates the fluid
that can actually be delivered (manufacturers add an **overfill** so the full labelled dose can be withdrawn). The total
drug MASS is the amount per vial times the number of vials (a PRODUCT); the NET deliverable VOLUME is the labelled
volume minus the overfill (a DIFFERENCE); and the concentration actually delivered is the mass over the net volume.
Dividing a product by a difference introduces a genuinely NEW arithmetic shape on the ladder: a **product over a
difference** — `(a · b) / (c - d)` — a parenthesised product in the numerator over a parenthesised difference in the
denominator.

The setup: a bag is prepared from `vial_count` vials of `vial_strength` mg each, in a bag whose labelled `total_volume`
includes an `overfill_volume` that cannot be delivered. The delivered concentration is the total mass over the net
volume:

  CONCENTRATION   (vial_strength · vial_count) / (total_volume - overfill_volume)   [ the delivered strength ]
  TOTAL MASS      vial_strength · vial_count                                         [ the product: total drug ]
  NET VOLUME      total_volume - overfill_volume                                     [ the difference: deliverable fluid ]

The **concentration** is what makes this rung distinctive — it is the ladder's first **product over a difference**: a
parenthesised product divided by a parenthesised difference. Contrast the neighbours already on the ladder: rung-58 was
a *product over a SUM* `(a·b)/(c+d)` and rung-15 a *product over a PRODUCT* `(a·b)/(c·d)`; neither divided a PRODUCT by a
DIFFERENCE. (The total mass `vial_strength · vial_count` and the net volume `total_volume - overfill_volume` ride
alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-58 shipped their
component products/sums/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including the parenthesised product and difference and their quotient — and the harness reads
the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../57/58.
This rung exercises the engine across a **product divided by a difference** — the fact that `(a·b)/(c-d)` is NOT
`a·b/c - d` and NOT `(a·b)/c` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `·`, `-` and `/`
— **no structural constants** — so no numeric literal appears in any program, and neither the total mass, the net
volume, nor any concentration figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`vial_strength`, `vial_count`, `total_volume`, `overfill_volume`) so no
numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  MISGROUPED         (vial_strength · vial_count) / total_volume - overfill_volume   divide the mass by the LABELLED
                                                                                     volume, then subtract the overfill
                                                                                     (`… / total - overfill`, not
                                                                                     `… / (total - overfill)`), and
  MASS OVER TOTAL    (vial_strength · vial_count) / total_volume                     divide the mass by the LABELLED
                                                                                     volume alone, forgetting the
                                                                                     overfill,

which are exactly the mistakes a student makes (breaking the grouping so the mass divides only the labelled volume, or
dividing by the labelled volume instead of the net deliverable volume). Gold rotates A-E by index. QUERIED (used as
gold) = the three real readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive and `total_volume > overfill_volume`, so the net volume
and every product, difference and quotient is positive; the tables below are chosen so the five family values are
pairwise distinct with a comfortable margin (and the misgrouped slip stays positive), asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (VIAL_STRENGTH, VIAL_COUNT, TOTAL_VOLUME, OVERFILL_VOLUME) — the drug amount in one vial (mg) and the number of vials
# (their product is the total mass), and the labelled bag volume and the un-deliverable overfill (their difference is
# the net deliverable volume), all plain positive numbers with total_volume > overfill_volume. The five family values
# are asserted pairwise-distinct (with margin) below, and the misgrouped slip is asserted positive.
TABLES = [
    (200, 2, 50, 5),
    (100, 3, 60, 4),
    (50, 6, 40, 5),
    (80, 5, 50, 6),
    (120, 3, 80, 4),
    (90, 4, 45, 6),
    (250, 2, 100, 4),
]

# The option family (5 members), all built from the four observed quantities via *, - and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "concentration",
        "the delivered concentration (the total drug mass over the net deliverable volume)",
        "(vial_strength * vial_count) / (total_volume - overfill_volume)",
    ),
    (
        "total_mass",
        "the total drug mass (strength per vial times the number of vials)",
        "vial_strength * vial_count",
    ),
    (
        "net_volume",
        "the net deliverable volume (labelled volume minus the overfill)",
        "total_volume - overfill_volume",
    ),
    (
        "misgrouped",
        "the mass divided by the LABELLED volume, with the overfill subtracted off (broken grouping)",
        "(vial_strength * vial_count) / total_volume - overfill_volume",
    ),
    (
        "mass_over_total",
        "the mass divided by the LABELLED volume alone (forgetting the overfill)",
        "(vial_strength * vial_count) / total_volume",
    ),
]
QUERIED = ["concentration", "total_mass", "net_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(vial_strength, vial_count, total_volume, overfill_volume):
    # Operation order mirrors the ADJ programs exactly (a parenthesised product over a parenthesised difference; and, for
    # the misgrouped slip, the mass-over-total quotient binds tighter than the trailing subtraction), so the Python
    # option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    mass = vial_strength * vial_count
    net = total_volume - overfill_volume
    return {
        "concentration": mass / net,
        "total_mass": mass,
        "net_volume": net,
        "misgrouped": mass / total_volume - overfill_volume,
        "mass_over_total": mass / total_volume,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for vial_strength, vial_count, total_volume, overfill_volume in TABLES:
        assert (
            vial_strength > 0
            and vial_count > 0
            and total_volume > 0
            and overfill_volume > 0
            and total_volume > overfill_volume
        ), (vial_strength, vial_count, total_volume, overfill_volume)
        fv = family_values(vial_strength, vial_count, total_volume, overfill_volume)
        # The misgrouped slip must stay positive so it reads as a plausible mistake, not an obvious negative.
        assert fv["misgrouped"] > 0, (vial_strength, vial_count, total_volume, overfill_volume, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    vial_strength,
                    vial_count,
                    total_volume,
                    overfill_volume,
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
                vial_strength,
                vial_count,
                total_volume,
                overfill_volume,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r59conc-{idx + 1:02d}",
                "qtype": "reconstituted_concentration",
                "stem": (
                    f"A chemotherapy bag is prepared from {num(vial_count)} vials of {num(vial_strength)} mg each, in a "
                    f"bag labelled {num(total_volume)} mL of which {num(overfill_volume)} mL is un-deliverable overfill. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe vial_strength({num(vial_strength)})\n"
                    f"observe vial_count({num(vial_count)})\n"
                    f"observe total_volume({num(total_volume)})\n"
                    f"observe overfill_volume({num(overfill_volume)})\n"
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
            "ADJ-LADDER rung 59 — reconstituted drug concentration from four stated quantities (a NEW panel: oncology "
            "infusion pharmacy). From a strength per vial and a vial count (their product is the total drug mass) and a "
            "labelled bag volume and an un-deliverable overfill (their difference is the net deliverable volume), "
            "compute the delivered concentration "
            "((vial_strength*vial_count)/(total_volume-overfill_volume)), the total mass "
            "(vial_strength*vial_count), or the net volume (total_volume-overfill_volume). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW shape, PRODUCT OVER A DIFFERENCE (a*b)/(c-d), the first on the ladder to divide a "
            "product by a difference (distinct from rung-58 product-over-sum (a*b)/(c+d) and rung-15 "
            "product-over-product (a*b)/(c*d)) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the four observed quantities via *, - and / — no "
            "constant leaks, and neither the total mass, the net volume, nor any concentration figure ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same four quantities, so the distractors "
            "are exactly the slips students make: breaking the grouping so the mass divides only the labelled volume "
            "((a*b)/c-d, not (a*b)/(c-d)), and dividing by the labelled volume alone ((a*b)/c). The core confusion "
            "tested is dividing a product by a grouped difference."
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
