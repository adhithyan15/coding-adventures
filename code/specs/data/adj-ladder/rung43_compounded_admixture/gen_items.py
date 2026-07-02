"""Generate rung-43 (compounded IV admixture total drug mass) items.json for the ADJ-LADDER.

Rung 43 opens the **compounding pharmacy / IV admixture** panel on the quantitative band — the arithmetic of the
total drug mass in a bag compounded from SEVERAL additives, each contributing (its volume x its concentration).
This rung introduces a genuinely NEW arithmetic shape on the ladder: a **sum of THREE products** —
`a*b + c*d + e*f`. It extends rung-34 (which summed TWO products) to three, so it is the natural generalisation of
that shape rather than a repeat.

The setup: three additives go into one bag. Each additive `i` contributes `volume_i * concentration_i` of drug.
The total drug mass is the sum of the three per-additive amounts:

  TOTAL MASS   volume_a*concentration_a + volume_b*concentration_b + volume_c*concentration_c   [ all three products summed ]
  FIRST TWO    volume_a*concentration_a + volume_b*concentration_b                               [ the first two additives only ]
  LAST TWO     volume_b*concentration_b + volume_c*concentration_c                               [ the last two additives only ]

The **total mass** is what makes this rung distinctive — it is the ladder's first **sum of three products**: three
`volume*concentration` products added together. Contrast the neighbours: rung-34 summed *two* products
(`a*b + c*d`), rung-38/39/40 were flat three-term diff/sum/product of single terms; none summed three *products*.
(The first-two and last-two partial sums ride alongside as the two-additive sub-totals, so the panel teaches the
whole build-up — exactly as rung-34 shipped its two component products and rung-42 its component quantities beside
the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the six quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — three `volume*concentration` products summed — and the harness reads the scalar via
the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../41/42. This rung
exercises the engine across a **sum of three products** (`(va*ca) + (vb*cb) + (vc*cc)`).

Contamination-safe by construction: every formula is built ONLY from the six observed quantities via `*` and `+` —
**no structural constants** — so no numeric literal appears in any program, and no per-additive amount or total
ever appears as a literal (each is computed from the observed volumes and concentrations). The observed quantities
carry **digit-free identifiers** (`volume_a`, `concentration_a`, `volume_b`, `concentration_b`, `volume_c`,
`concentration_c`) so no numeral hides inside a variable name.

The five options are a tight family over the same six quantities: the three real readouts plus the two classic
slips —

  CROSSED       volume_a*concentration_b + volume_b*concentration_c + volume_c*concentration_a   each volume paired
                                                                                                 with the WRONG
                                                                                                 additive's
                                                                                                 concentration, and
  ALL SUMMED    volume_a + concentration_a + volume_b + concentration_b + volume_c + concentration_c   every quantity
                                                                                                 added, ignoring the
                                                                                                 products,

which are exactly the mistakes a student makes (mismatching volume with concentration, or adding raw quantities
instead of forming each additive's product first). Gold rotates A-E by index. QUERIED (used as gold) = the three
real readouts; all five always appear as options.

Distinctness: all six observed quantities are positive, so every product and sum is positive; the tables below are
chosen so the five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (VOLUME_A, CONCENTRATION_A, VOLUME_B, CONCENTRATION_B, VOLUME_C, CONCENTRATION_C) — volumes in mL, concentrations
# in mg/mL, all strictly positive. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (2, 10, 3, 20, 1, 30),
    (4, 5, 2, 15, 3, 10),
    (1, 40, 2, 20, 5, 4),
    (3, 10, 1, 50, 2, 25),
    (2, 25, 4, 10, 1, 60),
    (5, 4, 3, 20, 2, 15),
    (2, 30, 1, 40, 3, 10),
]

# The option family (5 members), all built from the six observed quantities via * and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "total_mass",
        "total drug mass in the bag (all three additive amounts summed)",
        "volume_a * concentration_a + volume_b * concentration_b + volume_c * concentration_c",
    ),
    (
        "first_two",
        "combined mass of the first two additives only",
        "volume_a * concentration_a + volume_b * concentration_b",
    ),
    (
        "last_two",
        "combined mass of the last two additives only",
        "volume_b * concentration_b + volume_c * concentration_c",
    ),
    (
        "crossed",
        "each volume multiplied by the WRONG additive's concentration (a mismatched pairing)",
        "volume_a * concentration_b + volume_b * concentration_c + volume_c * concentration_a",
    ),
    (
        "all_summed",
        "every quantity added together, ignoring the volume-times-concentration products",
        "volume_a + concentration_a + volume_b + concentration_b + volume_c + concentration_c",
    ),
]
QUERIED = ["total_mass", "first_two", "last_two"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(va, ca, vb, cb, vc, cc):
    # Operation order mirrors the ADJ programs exactly (products bind tighter than +, left-folded sums), so the
    # Python option value and the engine result are the same IEEE-double (within the harness's 1e-9 tolerance).
    return {
        "total_mass": va * ca + vb * cb + vc * cc,
        "first_two": va * ca + vb * cb,
        "last_two": vb * cb + vc * cc,
        "crossed": va * cb + vb * cc + vc * ca,
        "all_summed": va + ca + vb + cb + vc + cc,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for va, ca, vb, cb, vc, cc in TABLES:
        assert all(q > 0 for q in (va, ca, vb, cb, vc, cc)), (va, ca, vb, cb, vc, cc)
        fv = family_values(va, ca, vb, cb, vc, cc)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (va, ca, vb, cb, vc, cc, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, va, ca, vb, cb, vc, cc, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r43mix-{idx + 1:02d}",
                "qtype": "compounded_admixture",
                "stem": (
                    f"An IV bag is compounded from three additives: {num(va)} mL of a {num(ca)} mg/mL drug, "
                    f"{num(vb)} mL of a {num(cb)} mg/mL drug, and {num(vc)} mL of a {num(cc)} mg/mL drug. "
                    f"What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe volume_a({num(va)})\n"
                    f"observe concentration_a({num(ca)})\n"
                    f"observe volume_b({num(vb)})\n"
                    f"observe concentration_b({num(cb)})\n"
                    f"observe volume_c({num(vc)})\n"
                    f"observe concentration_c({num(cc)})\n"
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
            "ADJ-LADDER rung 43 — compounded IV admixture total drug mass from six stated quantities (a NEW panel: "
            "compounding pharmacy / IV admixture). Three additives each contribute volume*concentration of drug; "
            "compute the total mass (va*ca + vb*cb + vc*cc), the first-two subtotal (va*ca + vb*cb), or the last-two "
            "subtotal (vb*cb + vc*cc). Each item is a compute_dimensioned program (observe the six quantities, let "
            "answer = formula); the ADJ engine carries the arithmetic — a NEW shape, a SUM OF THREE PRODUCTS "
            "(va*ca + vb*cb + vc*cc), extending rung-34's sum-of-two-products to three — and the harness matches the "
            "scalar to the printed options. Contamination-safe: every index is built only from the six observed "
            "quantities via * and + — no constant leaks, and no per-additive amount or total ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral "
            "hides inside a variable name. The five options are a family over the same six quantities, so the "
            "distractors are exactly the slips students make: pairing each volume with the WRONG additive's "
            "concentration, and adding every raw quantity instead of forming each product first. The core confusion "
            "tested is summing the three volume-times-concentration products."
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
