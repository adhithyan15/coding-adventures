"""Generate rung-47 (toxic-ingestion concentration) items.json for the ADJ-LADDER.

Rung 47 opens the **toxicology / ingestion-concentration** panel on the quantitative band — the arithmetic of the
concentration a drug reaches after an overdose, once the ingested dose distributes into the body water actually
available to it. The ingested dose is a PRODUCT (`tablet_count · drug_per_tablet` — how many tablets times the drug
in each); it distributes not into the whole body water but into the *effective* volume, the body water minus a
non-distributing deficit (`body_water − deficit`), a DIFFERENCE. The resulting concentration is therefore the dose
divided by that effective volume. This rung introduces a genuinely NEW arithmetic shape on the ladder:
**product-over-difference** — `(a · b) / (c − d)` — a product in the numerator divided by a difference in the
denominator.

The setup: a patient ingests `tablet_count` tablets, each containing `drug_per_tablet` mg. The drug distributes into
the effective volume, the total body water `body_water` (litres) minus a non-distributing `deficit` (litres). The
ingested concentration is the ingested dose divided by the effective volume:

  INGESTION CONCENTRATION   (tablet_count · drug_per_tablet) / (body_water − deficit)   [ mg per litre ]
  INGESTED DOSE             tablet_count · drug_per_tablet                              [ the numerator: total drug ]
  EFFECTIVE VOLUME          body_water − deficit                                        [ the denominator ]

The **ingestion concentration** is what makes this rung distinctive — it is the ladder's first
**product-over-difference**: a parenthesised product divided by a parenthesised difference. Contrast the neighbours
already on the ladder: rung-42 was a *sum-over-difference* `(a+b)/(c−d)`, rung-44 a *product-over-sum* `(a·b)/(c+d)`,
rung-45 a *difference-over-product* `(a−b)/(c·d)`, rung-46 a *sum-over-product* `(a+b)/(c·d)`; none divided a PRODUCT
by a DIFFERENCE. (The ingested dose `tablet_count · drug_per_tablet` and the effective volume `body_water − deficit`
ride alongside as the two component quantities, so the panel teaches the whole calculation — exactly as rung-46
shipped its combined cost and patient-days beside the headline rate.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic — including the inner `(tablet_count · drug_per_tablet)` product and the `(body_water −
deficit)` difference — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../45/46. This rung exercises the engine across a **product divided by a
difference**.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `·`, `−` and
`/` — **no structural constants** — so no numeric literal appears in any program, and neither the ingested dose, the
effective volume, nor any concentration is ever a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`tablet_count`, `drug_per_tablet`, `body_water`, `deficit`) so
no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic
slips —

  DOSE OVER WATER ONLY  (tablet_count · drug_per_tablet) / body_water                   divide by the TOTAL body
                                                                                        water, forgetting the deficit,
                                                                                        and
  SUMMED DENOMINATOR    (tablet_count · drug_per_tablet) / (body_water + deficit)       ADD the deficit to the body
                                                                                        water instead of subtracting
                                                                                        it,

which are exactly the mistakes a student makes (dropping the correction, or adding a quantity that should be
subtracted). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness: all four observed quantities are positive with `body_water > deficit`, so every product is positive and
every denominator (the effective volume, the total water, and their sum) is strictly positive (no division by zero);
the tables below are chosen so the five family values are pairwise distinct with a comfortable margin, asserted at
build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TABLET_COUNT, DRUG_PER_TABLET, BODY_WATER, DEFICIT) — tablets are plain counts, drug_per_tablet in mg, body_water
# and deficit in litres. All four quantities are strictly positive and BODY_WATER > DEFICIT, so the effective volume
# (and every denominator) is strictly positive. The five family values are asserted pairwise-distinct (with margin)
# below.
TABLES = [
    (20, 50, 42, 6),
    (15, 40, 36, 6),
    (25, 20, 40, 10),
    (30, 30, 50, 5),
    (10, 60, 48, 8),
    (18, 25, 45, 9),
    (40, 15, 44, 4),
]

# The option family (5 members), all built from the four observed quantities via *, - and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "ingestion_concentration",
        "ingestion concentration (ingested dose spread over the effective volume)",
        "(tablet_count * drug_per_tablet) / (body_water - deficit)",
    ),
    (
        "ingested_dose",
        "the ingested dose (tablets times the drug in each)",
        "tablet_count * drug_per_tablet",
    ),
    (
        "effective_volume",
        "the effective volume (body water minus the deficit)",
        "body_water - deficit",
    ),
    (
        "dose_over_water_only",
        "ingested dose over the TOTAL body water, forgetting the deficit (a wrong denominator)",
        "(tablet_count * drug_per_tablet) / body_water",
    ),
    (
        "summed_denominator",
        "ingested dose over body water PLUS the deficit (added instead of subtracted)",
        "(tablet_count * drug_per_tablet) / (body_water + deficit)",
    ),
]
QUERIED = ["ingestion_concentration", "ingested_dose", "effective_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tablet_count, drug_per_tablet, body_water, deficit):
    # Operation order mirrors the ADJ programs exactly (product in the numerator, difference in the denominator), so
    # the Python option value and the engine result are the same IEEE-double (well within the harness's 1e-9 match
    # tolerance).
    dose = tablet_count * drug_per_tablet
    effective_volume = body_water - deficit
    return {
        "ingestion_concentration": dose / effective_volume,
        "ingested_dose": dose,
        "effective_volume": effective_volume,
        "dose_over_water_only": dose / body_water,
        "summed_denominator": dose / (body_water + deficit),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for tablet_count, drug_per_tablet, body_water, deficit in TABLES:
        assert (
            tablet_count > 0
            and drug_per_tablet > 0
            and body_water > 0
            and deficit > 0
            and body_water > deficit
        ), (tablet_count, drug_per_tablet, body_water, deficit)
        fv = family_values(tablet_count, drug_per_tablet, body_water, deficit)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    tablet_count,
                    drug_per_tablet,
                    body_water,
                    deficit,
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
                tablet_count,
                drug_per_tablet,
                body_water,
                deficit,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r47ing-{idx + 1:02d}",
                "qtype": "ingestion_concentration",
                "stem": (
                    f"A patient ingests {num(tablet_count)} tablets, each containing {num(drug_per_tablet)} mg. The "
                    f"drug distributes into the effective volume, {num(body_water)} L of body water minus a "
                    f"{num(deficit)} L deficit. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe tablet_count({num(tablet_count)})\n"
                    f"observe drug_per_tablet({num(drug_per_tablet)})\n"
                    f"observe body_water({num(body_water)})\n"
                    f"observe deficit({num(deficit)})\n"
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
            "ADJ-LADDER rung 47 — toxic-ingestion concentration from four stated quantities (a NEW panel: "
            "toxicology / ingestion-concentration). From a tablet count and a per-tablet dose plus a body-water volume "
            "and a non-distributing deficit compute the ingestion concentration ((tablet_count*drug_per_tablet)/"
            "(body_water-deficit)), the ingested dose (tablet_count*drug_per_tablet), or the effective volume "
            "(body_water-deficit). Each item is a compute_dimensioned program (observe the four quantities, let answer "
            "= formula); the ADJ engine carries the arithmetic — a NEW shape, PRODUCT-OVER-DIFFERENCE (a*b)/(c-d), the "
            "first quotient on the ladder to divide a parenthesised product by a parenthesised difference (distinct "
            "from rung-42 sum-over-difference (a+b)/(c-d), rung-44 product-over-sum (a*b)/(c+d), rung-45 "
            "difference-over-product (a-b)/(c*d), and rung-46 sum-over-product (a+b)/(c*d)) — and the harness matches "
            "the scalar to the printed options. Contamination-safe: every index is built only from the four observed "
            "quantities via *, - and / — no constant leaks, and neither the ingested dose, the effective volume, nor "
            "any concentration ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the "
            "same four quantities, so the distractors are exactly the slips students make: dividing by the TOTAL body "
            "water (dropping the deficit), and ADDING the deficit instead of subtracting it. The core confusion tested "
            "is dividing the ingested-dose product by the effective-volume difference."
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
