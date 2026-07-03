"""Generate rung-35 (dialysis solute mass balance) items.json for the ADJ-LADDER.

Rung 35 opens the **renal-replacement / dialysis clearance** panel on the quantitative band — the arithmetic of
how much solute a dialyzer actually removes over a session. Solute is conserved: the amount entering on the blood
side (its **volume times its concentration**) minus the amount leaving in the spent effluent (its own volume
times its own concentration) is the amount cleared. It uses the same contamination-safe shape as the
fluid-admixture rung (34), the stroke-work rung (33), and the respiratory rung (32): a small table of *observed*
volumes and concentrations and a tight family of mutually-confusable formulas built **only from those observed
quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single dialysis session's mass balance. FOUR quantities are measured — two volumes (L)
and two concentrations (mmol/L):

  INFLOW_VOLUME          Vi   volume processed on the inflow (blood) side
  INFLOW_CONCENTRATION   Ci   solute concentration entering
  OUTFLOW_VOLUME         Vo   volume of spent effluent leaving
  OUTFLOW_CONCENTRATION  Co   solute concentration leaving (lower — the dialyzer pulled solute out)

The net solute removed is the **amount that entered minus the amount that left** — a *difference of two
products* — `(INFLOW_VOLUME * INFLOW_CONCENTRATION) - (OUTFLOW_VOLUME * OUTFLOW_CONCENTRATION)`. That is what
makes this rung distinctive: it is a NEW arithmetic shape on the ladder — a difference whose TWO terms are each
their own product. This continues the two-operand-composition series: rung-31 subtracted one difference from
another, rung-32 divided one difference by another, rung-33 multiplied one difference by another, rung-34 ADDED
one product to another, and rung-35 SUBTRACTS one product from another. The core confusion this rung tests is
pairing the right volume with the right concentration inside each product (a side's own volume times its own
concentration), rather than crossing them:

  NET SOLUTE REMOVED         (Vi * Ci) - (Vo * Co)   [ solute in − solute out = amount cleared ]
  INFLOW SOLUTE              Vi * Ci                  [ the solute entering, one term ]
  OUTFLOW SOLUTE             Vo * Co                  [ the solute leaving, the other term ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/33/34. This rung exercises the engine across a
SUBTRACTION of two parenthesised PRODUCTS.

Contamination-safe by construction: every formula is built only from the four observed quantities via `*`, `-`,
`+` — **no structural constants** — so every program literal is grounded in the stem. Neither side's solute
amount ever appears as a literal (each is computed from the observed volume and concentration). The observed
quantities carry **digit-free identifiers** (`inflow_volume`, `inflow_concentration`, `outflow_volume`,
`outflow_concentration`) so no numeral hides inside a variable name. The five options are a tight family over the
same quantities: the three real indices plus the two classic slips —

  CROSSED PRODUCTS DIFFERENCE   (Vi * Co) - (Vo * Ci)   each volume paired with the OTHER side's concentration, and
  SUMMED PRODUCTS               (Vi * Ci) + (Vo * Co)   the two amounts ADDED instead of subtracted,

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the summed products is the largest value, the inflow solute next (it exceeds the outflow solute
so the net removed is strictly positive), then the net removed and the outflow solute, and the crossed-products
difference is a small (sometimes negative — the sign-slip) value; the tables below are chosen so the five family
values are pairwise distinct — with a comfortable margin — for every item, asserted at build time (inflow solute
> outflow solute so the net removed is strictly positive, and no two family values collide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (INFLOW_VOLUME, INFLOW_CONCENTRATION, OUTFLOW_VOLUME, OUTFLOW_CONCENTRATION) observed per session. Volumes in
# L, concentrations in mmol/L, so each product is an amount in mmol. The inflow amount (Vi*Ci) exceeds the
# outflow amount (Vo*Co) on every row, so the net solute removed is strictly positive. The five family values
# are asserted pairwise-distinct (with margin) below.
#   Vi = inflow (blood-side) volume        Ci = its concentration (higher, pre-dialysis)
#   Vo = outflow (effluent) volume         Co = its concentration (lower, post-dialysis)
TABLES = [
    (4, 120, 2, 80),
    (5, 130, 3, 70),
    (3, 150, 2, 90),
    (4, 140, 3, 85),
    (5, 110, 2, 95),
    (3, 160, 2, 100),
    (4, 125, 2, 70),
]

# The option family (5 members), all built from the observed quantities via `*` / `-` / `+`. Every identifier
# is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    (
        "net_solute_removed",
        "net solute removed by the dialyzer",
        "(inflow_volume * inflow_concentration) - (outflow_volume * outflow_concentration)",
    ),
    (
        "inflow_solute",
        "solute entering on the inflow side",
        "inflow_volume * inflow_concentration",
    ),
    (
        "outflow_solute",
        "solute leaving in the effluent",
        "outflow_volume * outflow_concentration",
    ),
    (
        "crossed_products_difference",
        "crossed-products difference (each volume with the other side's concentration)",
        "(inflow_volume * outflow_concentration) - (outflow_volume * inflow_concentration)",
    ),
    (
        "summed_products",
        "summed amounts (inflow plus outflow, added not subtracted)",
        "(inflow_volume * inflow_concentration) + (outflow_volume * outflow_concentration)",
    ),
]
QUERIED = ["net_solute_removed", "inflow_solute", "outflow_solute"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(vi, ci, vo, co):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    inflow_amount = vi * ci
    outflow_amount = vo * co
    return {
        "net_solute_removed": inflow_amount - outflow_amount,
        "inflow_solute": inflow_amount,
        "outflow_solute": outflow_amount,
        "crossed_products_difference": (vi * co) - (vo * ci),
        "summed_products": inflow_amount + outflow_amount,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for vi, ci, vo, co in TABLES:
        inflow_amount = vi * ci
        outflow_amount = vo * co
        assert inflow_amount > 0 and outflow_amount > 0, (vi, ci, vo, co)
        assert inflow_amount > outflow_amount, (vi, ci, vo, co)  # net removed strictly positive
        fv = family_values(vi, ci, vo, co)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (vi, ci, vo, co, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, vi, ci, vo, co, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r35dialysis-{idx + 1:02d}",
                "qtype": "dialysis_clearance",
                "stem": (
                    f"Over one dialysis session, {num(vi)} L is processed on the inflow side at a solute "
                    f"concentration of {num(ci)} mmol/L, and {num(vo)} L of spent effluent leaves at "
                    f"{num(co)} mmol/L. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe inflow_volume({num(vi)})\n"
                    f"observe inflow_concentration({num(ci)})\n"
                    f"observe outflow_volume({num(vo)})\n"
                    f"observe outflow_concentration({num(co)})\n"
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
            "ADJ-LADDER rung 35 — net solute removed in a dialysis session from two volumes and two "
            "concentrations (a NEW panel: renal-replacement / dialysis clearance). From four stated quantities "
            "(inflow volume Vi, inflow concentration Ci, outflow volume Vo, outflow concentration Co) compute "
            "the net solute removed ((Vi*Ci)-(Vo*Co)), the inflow solute (Vi*Ci), or the outflow solute "
            "(Vo*Co). Each item is a compute_dimensioned program (observe the four quantities, let answer = "
            "formula); the ADJ engine carries the arithmetic — a NEW shape, a DIFFERENCE OF TWO PRODUCTS "
            "((Vi*Ci)-(Vo*Co)), so one parenthesised product is subtracted from another — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only from the "
            "four observed quantities via *, - and + — no constant leaks (each amount is a pure "
            "volume*concentration product), and neither side's solute amount ever appears as a literal (each is "
            "computed from the observed volume and concentration) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same "
            "quantities, so the distractors are exactly the slips students make: the crossed-products difference "
            "((Vi*Co)-(Vo*Ci), each volume with the other side's concentration) and the summed products "
            "((Vi*Ci)+(Vo*Co), the two amounts added instead of subtracted). The core confusion tested is "
            "subtracting one volume*concentration product from another with each pairing correct."
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
              "=", round(it["options"][it["gold_letter"]]["value"], 4))
