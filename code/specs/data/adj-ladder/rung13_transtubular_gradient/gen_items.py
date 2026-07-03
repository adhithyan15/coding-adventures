"""Generate rung-13 (transtubular potassium gradient) items.json for the ADJ-LADDER.

Rung 13 stays in the quantitative-clinical band and reuses, verbatim, the contamination-safe
shape of the renal-indices rung (rung 9, fractional excretion): a small table of *observed*
quantities and a tight family of mutually-confusable formulas that are **pure
multiplication/division of those observed quantities** — no numeric literal anywhere.

The clinical setting is a potassium-disorder work-up. When a patient is hypo- or hyperkalaemic,
the nephrologist asks: *is the kidney handling potassium appropriately?* The bedside index that
answers this is the **transtubular potassium gradient (TTKG)** — it estimates the potassium
gradient across the cortical collecting duct, correcting the urine-to-plasma potassium ratio for
the water that has been reabsorbed (read off the urine-to-plasma osmolality ratio). We observe four
quantities:

  UK    urine potassium        (mEq/L)
  PK    plasma potassium       (mEq/L)
  Uosm  urine osmolality       (mOsm/kg)
  Posm  plasma osmolality      (mOsm/kg)

From these, the three bedside ratios fall out as **pure ratios of products** — exact, and needing
no constant at all:

  TTKG (transtubular potassium gradient) = (UK · Posm) / (PK · Uosm)   [ = (UK/PK) / (Uosm/Posm) ]
  U/P-K  (urine-to-plasma potassium ratio)   = UK / PK                 [dimensionless]
  U/P-osm (urine-to-plasma osmolality ratio) = Uosm / Posm             [dimensionless]

TTKG is exactly the U/P potassium ratio *divided by* the U/P osmolality ratio: the osmolality ratio
undoes the concentrating effect of water reabsorption, so what is left is the gradient the distal
nephron actually established. (The textbook TTKG carries no multiplier — unlike FENa's cosmetic
`× 100` — so not even a rendering constant leaks here.)

Each queried index is a `compute_dimensioned` program (observe the four quantities + `let answer =
formula`); the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 4/7/7b/7c/8/9.

Contamination-safe by construction: every formula is built only from the four observed quantities
via multiplication and division — **no structural constants** — so every program literal is
grounded in the stem, and every identifier (uk/pk/uosm/posm/answer) is digit-free. The five options
are a tight family of ratios over the same four quantities: the three real ratios {TTKG, U/P-K,
U/P-osm} plus the two classic slips —

  * inverted TTKG            (PK · Uosm) / (UK · Posm)   — flipping the whole gradient, and
  * wrong-direction correction (UK · Uosm) / (PK · Posm) — *multiplying* by the U/P osmolality ratio
    instead of dividing by it (correcting for water reabsorption in the wrong direction).

Gold rotates A–E by index.

Note on table choice: several family values coincide for special quantity ratios, so the tables
below are chosen so the five family values are pairwise distinct for every item, and this is
asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (UK, PK, Uosm, Posm) observed quantities. The five ratios are asserted pairwise-distinct below.
#   UK   = urine potassium   (mEq/L)
#   PK   = plasma potassium  (mEq/L)
#   Uosm = urine osmolality  (mOsm/kg)
#   Posm = plasma osmolality (mOsm/kg)
TABLES = [
    (40, 4, 580, 290),
    (60, 5, 600, 300),
    (20, 4, 400, 280),
    (80, 6, 800, 288),
    (30, 3, 450, 285),
    (50, 4.5, 500, 295),
    (15, 5, 360, 300),
]

# The option family (5 members), all multiplication/division over the observed quantities
# uk/pk/uosm/posm.  key -> (display name, formula-as-adj)
# Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    ("ttkg", "transtubular potassium gradient (TTKG)", "(uk * posm) / (pk * uosm)"),
    ("ukpk", "urine-to-plasma potassium ratio", "uk / pk"),
    ("uposm", "urine-to-plasma osmolality ratio", "uosm / posm"),
    ("ttkg_inv", "inverted transtubular potassium gradient", "(pk * uosm) / (uk * posm)"),
    ("wrongdir", "urine-to-plasma potassium ratio multiplied by the osmolality ratio",
     "(uk * uosm) / (pk * posm)"),
]
QUERIED = ["ttkg", "ukpk", "uposm"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(uk, pk, uosm, posm):
    return {
        "ttkg": (uk * posm) / (pk * uosm),
        "ukpk": uk / pk,
        "uposm": uosm / posm,
        "ttkg_inv": (pk * uosm) / (uk * posm),
        "wrongdir": (uk * uosm) / (pk * posm),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for uk, pk, uosm, posm in TABLES:
        fv = family_values(uk, pk, uosm, posm)
        assert len({round(fv[k], 12) for k in ORDER}) == 5, (uk, pk, uosm, posm, fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 12) for v in opts_vals}) == 5, (key, uk, pk, uosm, posm, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r13ttkg-{idx + 1:02d}",
                "qtype": "renal_potassium_index",
                "stem": (
                    f"A patient being worked up for a potassium disorder has a urine potassium of "
                    f"{uk} mEq/L, a plasma potassium of {pk} mEq/L, a urine osmolality of {uosm} "
                    f"mOsm/kg, and a plasma osmolality of {posm} mOsm/kg. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe uk({uk})\n"
                    f"observe pk({pk})\n"
                    f"observe uosm({uosm})\n"
                    f"observe posm({posm})\n"
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
            "ADJ-LADDER rung 13 — the transtubular potassium gradient and its sibling ratios, from a "
            "paired urine/plasma chemistry (potassium-disorder work-up). From four stated quantities "
            "(urine K, plasma K, urine osmolality, plasma osmolality) compute the transtubular "
            "potassium gradient (TTKG = (UK*Posm)/(PK*Uosm)), the urine-to-plasma potassium ratio "
            "(UK/PK), or the urine-to-plasma osmolality ratio (Uosm/Posm). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic and the harness matches the scalar to the printed options. "
            "Contamination-safe: every ratio is a pure product/quotient of the four observed "
            "quantities — no constant leaks at all (TTKG has no rendering multiplier). The five "
            "options are a family of ratios over the same quantities, so the distractors are exactly "
            "the slips students make: inverting the whole gradient, or multiplying by the osmolality "
            "ratio instead of dividing by it (correcting for water reabsorption in the wrong "
            "direction)."
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
