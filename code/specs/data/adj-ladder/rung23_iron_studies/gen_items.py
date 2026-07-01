"""Generate rung-23 (iron studies) items.json for the ADJ-LADDER.

Rung 23 opens the **iron-metabolism** panel on the quantitative band — the classic iron-deficiency /
anemia-of-chronic-disease / hemochromatosis workup — using the same contamination-safe shape as the
hepatology rung (20): a small table of *observed* laboratory quantities, and a tight family of
mutually-confusable formulas built **only from those observed quantities** (no numeric literal
anywhere in any program), so nothing structural can leak.

The clinical setup is a single iron panel. Three quantities are measured:

  IRON   serum iron                          (ug/dL)
  UIBC   unbound iron-binding capacity       (ug/dL)  — the still-free transferrin binding sites
  FERR   ferritin                            (ng/mL)  — the storage-iron marker

The total iron-binding capacity **TIBC = IRON + UIBC** is *derived from its two observed components*,
never a constant — so the transferrin-saturation family divides by a **sum of observed quantities**,
exactly like the hepatology rung's total-bilirubin denominator (rung 20). Three textbook iron indices
fall out as pure functions of the observed quantities — no constant required:

  TRANSFERRIN SATURATION  IRON / (IRON + UIBC)   [ =IRON/TIBC; low in iron deficiency, high in HH ]
  UNBOUND FRACTION        UIBC / (IRON + UIBC)   [ =UIBC/TIBC; the complement, high in iron deficiency ]
  IRON:UIBC RATIO         IRON / UIBC            [ the saturated-to-free binding-site ratio ]

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/18/19/20/21/22. This
exercises the engine across **division AND an inner addition-in-parentheses** (TIBC = IRON + UIBC) on
a fresh iron-panel stem — the same `(a + b)` grouping the hepatology rung used.

Contamination-safe by construction: every formula is built only from the three observed quantities via
`/`, `+`, and grouping `( )` — **no structural constants** (TIBC is computed, not observed, so no
x-factor appears) — so every program literal is grounded in the stem. The observed quantities carry
**digit-free identifiers** (`serum_iron`, `uibc`, `ferritin`) so no numeral hides inside a variable
name. The five options are a tight family over the same quantities: the three real indices plus the
two classic slips —

  UIBC / IRON              the **inverse** iron:UIBC ratio (written upside-down), and
  IRON / FERRITIN          the iron-to-ferritin ratio (confusing the transport pool with the store),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on table choice: the five family values can collide for special quantity ratios (e.g. saturation
== unbound fraction when IRON == UIBC). The tables below are chosen so the five family values are
pairwise distinct — with a comfortable margin — for every item, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (IRON, UIBC, FERR) observed quantities. The five index-family values are asserted
# pairwise-distinct (with margin) below.
#   IRON = serum iron                    (ug/dL)
#   UIBC = unbound iron-binding capacity (ug/dL)
#   FERR = ferritin                      (ng/mL)
TABLES = [
    (50, 200, 100),
    (60, 140, 150),
    (75, 225, 150),
    (120, 80, 150),
    (90, 210, 180),
    (80, 320, 160),
    (140, 210, 280),
]

# The option family (5 members), all built from the observed quantities via `/`, `+`, grouping. Every
# identifier is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried*
# (used as gold); all five always appear as the options.
FAMILY = [
    ("tsat", "transferrin saturation", "serum_iron / (serum_iron + uibc)"),
    ("unbound_frac", "unbound fraction of total iron-binding capacity", "uibc / (serum_iron + uibc)"),
    ("iron_uibc", "iron-to-UIBC ratio", "serum_iron / uibc"),
    ("uibc_iron", "UIBC-to-iron ratio", "uibc / serum_iron"),
    ("iron_ferritin", "iron-to-ferritin ratio", "serum_iron / ferritin"),
]
QUERIED = ["tsat", "unbound_frac", "iron_uibc"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(iron, uibc, ferritin):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "tsat": iron / (iron + uibc),
        "unbound_frac": uibc / (iron + uibc),
        "iron_uibc": iron / uibc,
        "uibc_iron": uibc / iron,
        "iron_ferritin": iron / ferritin,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for iron, uibc, ferritin in TABLES:
        fv = family_values(iron, uibc, ferritin)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (iron, uibc, ferritin, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, iron, uibc, ferritin, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r23iron-{idx + 1:02d}",
                "qtype": "iron_index",
                "stem": (
                    f"An iron panel shows serum iron {iron} ug/dL, unbound iron-binding capacity (UIBC) "
                    f"{uibc} ug/dL, and ferritin {ferritin} ng/mL. What is the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe serum_iron({iron})\n"
                    f"observe uibc({uibc})\n"
                    f"observe ferritin({ferritin})\n"
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
            "ADJ-LADDER rung 23 — iron studies from a single iron panel (a NEW panel: iron metabolism). "
            "From three stated quantities (serum iron IRON, unbound iron-binding capacity UIBC, ferritin "
            "FERR) compute the transferrin saturation (IRON/(IRON+UIBC)), the unbound fraction of TIBC "
            "(UIBC/(IRON+UIBC)), or the iron-to-UIBC ratio (IRON/UIBC). Each item is a compute_dimensioned "
            "program (observe the three quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a division AND an inner sum-in-parentheses for total iron-binding capacity "
            "(TIBC = IRON + UIBC) — and the harness matches the scalar to the printed options. "
            "Contamination-safe: every index is built only from the three observed quantities via /, +, "
            "and grouping — no constant leaks (TIBC is derived from its components, not observed) — and "
            "the observed quantities carry digit-free identifiers so no numeral hides inside a variable "
            "name. The five options are a family over the same quantities, so the distractors are exactly "
            "the slips students make: the inverse iron:UIBC ratio (UIBC/IRON, written upside-down) and "
            "the iron-to-ferritin ratio (IRON/FERR, confusing the transport pool with the store)."
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
