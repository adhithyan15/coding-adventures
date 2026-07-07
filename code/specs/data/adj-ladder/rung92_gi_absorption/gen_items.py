"""Generate rung-92 (gastroenterology / intestinal-absorption total) items.json for the ADJ-LADDER.

Rung 92 opens the **gastroenterology / intestinal-absorption** panel on the quantitative band — the arithmetic of a
total nutrient absorbed across the gut mucosa. A `baseline_uptake` and a `residual_bolus` are ADDED as two base terms,
a `transport_rate` times a `mucosal_area` gives the active-transport load in the MIDDLE, and all three ADD into the
total. A **product flanked by two added terms** introduces a genuinely NEW arithmetic family on the ladder: `a+b*c+d`,
i.e. `((a+(b*c))+d)`.

This is genuinely new and is the MIRROR of rung-91. Rung 91 shipped `a+b+c*d` — two added terms first, product LAST.
Rung 92 moves the product into the MIDDLE: `a+b*c+d` — one added term, then the product, then another added term. The
distinction matters because the product no longer sits at the tail of the chain: it is bracketed on BOTH sides by a bare
added term. No shipped shape ever placed a product between two independent added terms in one flat chain — rung-91
`a+b+c*d` put the product last, rung-79 `a*b+c/d`/rung-80 `a*b-c/d` led with a product, rung-83 `a-b*c/d`/rung-85
`a*b-c-d` attached bare terms to a single leading product. `a+b*c+d` is the ladder's first
**added-term-plus-a-middle-product-plus-added-term**. The operator order matters: `a+b*c+d` is `((a+(b*c))+d)` (the
product forms first by precedence, then the flat sum picks up the two bare terms on either side), NOT `(a+b)*c+d`
(folding the LEADING term into the product) and NOT `a+b+c*d` (multiplying the WRONG pair — the trailing two — and
adding the transport rate bare) — the two distractors exploit exactly those confusions. Note rung-91's *swapped*
distractor was this exact `a+b*c+d` form; rung 92 promotes it to the GOLD and picks fresh distractors, one of which is
rung-91's gold `a+b+c*d`.

The setup: a `baseline_uptake`, a `transport_rate`, a `mucosal_area`, and a `residual_bolus`. The total is:

  TOTAL ABSORBED    baseline_uptake + transport_rate * mucosal_area + residual_bolus  [ two added terms flanking a product ]
  ACTIVE TRANSPORT  transport_rate * mucosal_area                                     [ the middle product, before the sum ]
  PASSIVE LOAD      baseline_uptake + residual_bolus                                  [ the two flanking base terms, before the product ]

The **total absorbed** is what makes this rung distinctive — it is the ladder's first
**added-term-plus-a-middle-product-plus-added-term**. (The active transport `b*c` and the passive load `a+d` ride
alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-91 shipped their
component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the transport rate by the mucosal area into the active-transport load,
then the flat addition of the baseline uptake, that product, and the residual bolus (the product forming before the sum,
so a+b*c+d evaluates as ((a+(b*c))+d)) — and the harness reads the scalar via the existing `compute_dimensioned`
extractor. No harness/engine change, exactly as rungs 8/16/.../90/91. This rung exercises the engine across an
**added-term-plus-a-middle-product-plus-added-term** — the fact that `a+b*c+d` is `((a+(b*c))+d)` and NOT `(a+b)*c+d`
and NOT `a+b+c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `*` — **no
structural constants** — so no numeric literal appears in any program, and neither the active transport, the passive
load, nor any total figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`baseline_uptake`, `transport_rate`, `mucosal_area`, `residual_bolus`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (baseline_uptake + transport_rate) * mucosal_area + residual_bolus  fold the LEADING term (baseline) into
                                                                                 the product instead of leaving it added
                                                                                 (the classic `a+b*c+d` vs `(a+b)*c+d`
                                                                                 error), and
  SWAPPED    baseline_uptake + transport_rate + mucosal_area * residual_bolus    multiply the WRONG pair (area × bolus)
                                                                                 and add the transport rate bare
                                                                                 (`a+b+c*d` instead of `a+b*c+d`),

which are exactly the mistakes a student makes (folding a neighbouring term into the product, or multiplying the wrong
adjacent pair). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: every quantity is a plain positive number >= 2, so every family member — a sum of positive
terms and positive products — is automatically strictly positive; the tables are chosen so the five family values are
pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BASELINE_UPTAKE, TRANSPORT_RATE, MUCOSAL_AREA, RESIDUAL_BOLUS) — a baseline uptake and a residual bolus to add as two
# flanking base terms, and a transport rate times a mucosal area to add as the middle active-transport load, all plain
# positive numbers >= 2. Every family member is a sum of positive terms / positive products, so positivity is automatic;
# the five family values are asserted pairwise-distinct below.
TABLES = [
    (3, 2, 2, 4),
    (2, 4, 2, 5),
    (3, 3, 2, 4),
    (2, 2, 3, 5),
    (4, 2, 2, 3),
    (2, 3, 3, 4),
    (5, 2, 2, 4),
]

# The option family (5 members), all built from the four observed quantities via + and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "total_absorbed",
        "total nutrient absorbed (the baseline uptake plus the active-transport load plus the residual bolus)",
        "baseline_uptake + transport_rate * mucosal_area + residual_bolus",
    ),
    (
        "active_transport",
        "the active-transport load (the transport rate times the mucosal area, before adding the baseline uptake and residual bolus)",
        "transport_rate * mucosal_area",
    ),
    (
        "passive_load",
        "the passive load (the baseline uptake plus the residual bolus, the two flanking terms before adding the active-transport load)",
        "baseline_uptake + residual_bolus",
    ),
    (
        "crossed",
        "the baseline uptake and transport rate together times the mucosal area, plus the residual bolus, folding the leading term into the product instead of leaving it added (a wrong grouping)",
        "(baseline_uptake + transport_rate) * mucosal_area + residual_bolus",
    ),
    (
        "swapped",
        "the baseline uptake plus the transport rate, plus the mucosal area times the residual bolus, multiplying the wrong pair (a wrong pairing)",
        "baseline_uptake + transport_rate + mucosal_area * residual_bolus",
    ),
]
QUERIED = ["total_absorbed", "active_transport", "passive_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(baseline_uptake, transport_rate, mucosal_area, residual_bolus):
    # Operation order mirrors the ADJ programs exactly (the product forms first by precedence, then the flat sum picks up
    # the two flanking bare terms, so a+b*c+d evaluates as ((a+(b*c))+d)), so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "total_absorbed": baseline_uptake + transport_rate * mucosal_area + residual_bolus,
        "active_transport": transport_rate * mucosal_area,
        "passive_load": baseline_uptake + residual_bolus,
        "crossed": (baseline_uptake + transport_rate) * mucosal_area + residual_bolus,
        "swapped": baseline_uptake + transport_rate + mucosal_area * residual_bolus,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for baseline_uptake, transport_rate, mucosal_area, residual_bolus in TABLES:
        assert (
            baseline_uptake > 0
            and transport_rate > 0
            and mucosal_area > 0
            and residual_bolus > 0
        ), (baseline_uptake, transport_rate, mucosal_area, residual_bolus)
        fv = family_values(baseline_uptake, transport_rate, mucosal_area, residual_bolus)
        # Every family member is a sum of positive terms / positive products, so every value is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, baseline_uptake, transport_rate, mucosal_area, residual_bolus, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    baseline_uptake,
                    transport_rate,
                    mucosal_area,
                    residual_bolus,
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
                baseline_uptake,
                transport_rate,
                mucosal_area,
                residual_bolus,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r92absorp-{idx + 1:02d}",
                "qtype": "gi_absorption_total",
                "stem": (
                    f"An intestinal-absorption study records a baseline uptake of {num(baseline_uptake)} plus a "
                    f"transport rate of {num(transport_rate)} times a mucosal area of {num(mucosal_area)}, plus a "
                    f"residual bolus of {num(residual_bolus)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe baseline_uptake({num(baseline_uptake)})\n"
                    f"observe transport_rate({num(transport_rate)})\n"
                    f"observe mucosal_area({num(mucosal_area)})\n"
                    f"observe residual_bolus({num(residual_bolus)})\n"
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
            "ADJ-LADDER rung 92 — gastrointestinal absorption total from four stated quantities (a NEW panel: "
            "gastroenterology / intestinal-absorption). From a baseline uptake and a residual bolus to add as two "
            "flanking base terms and a transport rate times a mucosal area to add as the middle active-transport load, "
            "compute the total absorbed (baseline_uptake+transport_rate*mucosal_area+residual_bolus), the active "
            "transport (transport_rate*mucosal_area), or the passive load (baseline_uptake+residual_bolus). Each item is "
            "a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries "
            "the arithmetic — a NEW family, AN ADDED TERM PLUS A MIDDLE PRODUCT PLUS AN ADDED TERM a+b*c+d (multiply b by "
            "c, add a on the left and d on the right, so a+b*c+d = ((a+(b*c))+d); this is the MIRROR of rung-91's a+b+c*d "
            "which put the product LAST, and no prior shape placed a product BETWEEN two independent added terms in one "
            "flat chain — e.g. rung-79 a*b+c/d led with a product, rung-83 a-b*c/d attached bare terms to one product) — "
            "and the harness matches the scalar to the printed options. Contamination-safe: every figure is built only "
            "from the four observed quantities via + and * — no constant leaks, and neither the active transport, the "
            "passive load, nor any total figure ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: folding the "
            "leading term into the product ((a+b)*c+d, a wrong grouping) and multiplying the wrong (trailing) pair with "
            "the transport rate added bare (a+b+c*d, a wrong pairing). The core confusion tested is that a+b*c+d is "
            "((a+(b*c))+d), not (a+b)*c+d and not a+b+c*d."
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
