"""Generate rung-36 (transfusion component pooling) items.json for the ADJ-LADDER.

Rung 36 opens the **transfusion medicine / blood banking** panel on the quantitative band — the arithmetic of
what concentration you get when two component units are pooled into one bag. Each unit carries an *amount* equal
to its **volume times its concentration**; pool two units and the resulting concentration is the **total amount
divided by the total volume** — a volume-weighted average, NOT a plain average of the two concentrations. It uses
the same contamination-safe shape as the dialysis rung (35), the admixture rung (34), and the stroke-work rung
(33): a small table of *observed* volumes and concentrations and a tight family of mutually-confusable formulas
built **only from those observed quantities** (no numeric literal anywhere in any program), so nothing structural
can leak.

The clinical setup is a single pool of two component units. FOUR quantities are measured — two volumes (dL) and
two concentrations (×10³/µL):

  VOLUME_FIRST           V1   volume of the first unit
  CONCENTRATION_FIRST    C1   cell concentration of the first unit
  VOLUME_SECOND          V2   volume of the second unit
  CONCENTRATION_SECOND   C2   cell concentration of the second unit

The pooled concentration is the **combined amount over the combined volume** — a *sum of products divided by a
sum of the weights* — `(V1*C1 + V2*C2) / (V1 + V2)`. That is what makes this rung distinctive: it is a NEW
arithmetic shape on the ladder — a **weighted average**, the first three-way composition (two products summed,
then divided by a summed pair). The two-operand-composition series (rungs 31-35: a difference / ratio / product /
sum / difference of two sub-expressions) is complete; this rung nests a sum-of-products over a sum-of-weights. The
core confusion this rung tests is dividing the total amount by the total *volume* (the correct weights), rather
than crossing the volumes with the wrong concentrations or dividing by the wrong denominator:

  POOLED CONCENTRATION   (V1*C1 + V2*C2) / (V1 + V2)   [ total cells ÷ total volume = volume-weighted mean ]
  TOTAL CONTENT          V1*C1 + V2*C2                 [ the combined cell amount, the numerator ]
  TOTAL VOLUME           V1 + V2                       [ the combined volume, the denominator ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/34/35. This rung exercises the engine across a
SUM-OF-PRODUCTS DIVIDED BY A SUM (a weighted average).

Contamination-safe by construction: every formula is built only from the four observed quantities via `*`, `+`,
`/` — **no structural constants** — so every program literal is grounded in the stem. Neither the combined
content nor the combined volume ever appears as a literal (each is computed from the observed quantities). The
observed quantities carry **digit-free identifiers** (`volume_first`, `concentration_first`, `volume_second`,
`concentration_second`) so no numeral hides inside a variable name. The five options are a tight family over the
same quantities: the three real indices plus the two classic slips —

  CROSSED WEIGHTED   (V1*C2 + V2*C1) / (V1 + V2)   each volume paired with the OTHER unit's concentration, and
  CONTENT OVER COUNTS (V1*C1 + V2*C2) / (C1 + C2)  the amount divided by the sum of CONCENTRATIONS, not volumes,

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the total content is order 1e3 (two products summed), the pooled and crossed concentrations sit on
the concentration scale (between the two counts), the total volume is single-digit, and the content-over-counts
is order 1 (an amount divided by a large count-sum); the tables below are chosen so the five family values are
pairwise distinct — with a comfortable margin — for every item, asserted at build time (the two concentrations
differ so the weighted average is not degenerate, and no two family values collide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (VOLUME_FIRST, CONCENTRATION_FIRST, VOLUME_SECOND, CONCENTRATION_SECOND) observed per pool. Volumes in dL,
# concentrations in ×10³/µL, so each product is a cell amount. The two concentrations differ on every row (so
# the volume-weighted mean differs from a plain mean and from the crossed variant). The five family values are
# asserted pairwise-distinct (with margin) below.
#   V1 = first unit's volume       C1 = its concentration
#   V2 = second unit's volume      C2 = its concentration
TABLES = [
    (2, 150, 3, 250),
    (4, 120, 2, 300),
    (3, 200, 1, 320),
    (2, 180, 4, 260),
    (1, 140, 3, 280),
    (3, 160, 2, 240),
    (4, 210, 1, 150),
]

# The option family (5 members), all built from the observed quantities via `*` / `+` / `/`. Every identifier
# is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    (
        "pooled_concentration",
        "pooled concentration of the combined bag",
        "(volume_first * concentration_first + volume_second * concentration_second) / (volume_first + volume_second)",
    ),
    (
        "total_content",
        "total cell content of the combined bag",
        "volume_first * concentration_first + volume_second * concentration_second",
    ),
    (
        "total_volume",
        "total volume of the combined bag",
        "volume_first + volume_second",
    ),
    (
        "crossed_weighted",
        "crossed-weighted concentration (each volume with the other unit's concentration)",
        "(volume_first * concentration_second + volume_second * concentration_first) / (volume_first + volume_second)",
    ),
    (
        "content_over_counts",
        "content divided by the sum of concentrations (wrong denominator)",
        "(volume_first * concentration_first + volume_second * concentration_second) / (concentration_first + concentration_second)",
    ),
]
QUERIED = ["pooled_concentration", "total_content", "total_volume"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(v1, c1, v2, c2):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    content = v1 * c1 + v2 * c2
    volume = v1 + v2
    return {
        "pooled_concentration": content / volume,
        "total_content": content,
        "total_volume": volume,
        "crossed_weighted": (v1 * c2 + v2 * c1) / volume,
        "content_over_counts": content / (c1 + c2),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for v1, c1, v2, c2 in TABLES:
        assert v1 > 0 and c1 > 0 and v2 > 0 and c2 > 0, (v1, c1, v2, c2)
        assert c1 != c2, (v1, c1, v2, c2)  # distinct counts → non-degenerate weighted mean
        fv = family_values(v1, c1, v2, c2)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (v1, c1, v2, c2, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, v1, c1, v2, c2, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r36pool-{idx + 1:02d}",
                "qtype": "transfusion_pooling",
                "stem": (
                    f"Two component units are pooled into one bag: the first is {num(v1)} dL at a cell "
                    f"concentration of {num(c1)} ×10³/µL, and the second is {num(v2)} dL at {num(c2)} "
                    f"×10³/µL. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe volume_first({num(v1)})\n"
                    f"observe concentration_first({num(c1)})\n"
                    f"observe volume_second({num(v2)})\n"
                    f"observe concentration_second({num(c2)})\n"
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
            "ADJ-LADDER rung 36 — pooled concentration of two transfusion component units from two volumes and "
            "two concentrations (a NEW panel: transfusion medicine / blood banking). From four stated quantities "
            "(first volume V1, first concentration C1, second volume V2, second concentration C2) compute the "
            "pooled concentration ((V1*C1+V2*C2)/(V1+V2)), the total content (V1*C1+V2*C2), or the total volume "
            "(V1+V2). Each item is a compute_dimensioned program (observe the four quantities, let answer = "
            "formula); the ADJ engine carries the arithmetic — a NEW shape, a WEIGHTED AVERAGE (a sum of products "
            "divided by a sum of the weights, (V1*C1+V2*C2)/(V1+V2)), the first three-way composition on the "
            "ladder — and the harness matches the scalar to the printed options. Contamination-safe: every index "
            "is built only from the four observed quantities via *, + and / — no constant leaks (a weighted mean "
            "needs no constant), and neither the combined content nor the combined volume ever appears as a "
            "literal (each is computed from the observed quantities) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family "
            "over the same quantities, so the distractors are exactly the slips students make: the "
            "crossed-weighted concentration ((V1*C2+V2*C1)/(V1+V2), volumes paired with the wrong counts) and the "
            "content-over-counts ((V1*C1+V2*C2)/(C1+C2), dividing by the sum of concentrations instead of "
            "volumes). The core confusion tested is dividing the total amount by the total VOLUME, the correct "
            "weights."
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
