"""Generate rung-27 (urine anion gap) items.json for the ADJ-LADDER.

Rung 27 opens the **urinary electrolyte** panel on the quantitative band — the arithmetic of the
*urine anion gap* (UAG), the bedside calculation that distinguishes renal from gastrointestinal causes
of a normal-anion-gap (hyperchloremic) metabolic acidosis. It uses the same contamination-safe shape as
the mineral rung (26), the lipid rung (25), and the iron rung (23): a small table of *observed*
laboratory quantities and a tight family of mutually-confusable formulas built **only from those
observed quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single spot-urine electrolyte panel. Three quantities are measured:

  NA   urine sodium       (mEq/L)
  K    urine potassium    (mEq/L)
  CL   urine chloride     (mEq/L)

The urine anion gap falls out as a pure function of the three observed quantities — no constant
required. Crucially this rung introduces a **SUM inside a difference** — the compound shape
`(NA + K) - CL` — contrasted directly against the plain difference and plain sum forms, so the family
exercises the engine across `+` and `-` composed together on a fresh panel (the earlier rungs cycled
through pure ratios, subtraction-in-numerator, and a product; this rung adds addition and a two-step
add-then-subtract):

  URINE ANION GAP        (NA + K) - CL   [ =UAG; negative → GI HCO3 loss, positive → renal (RTA) ]
  SODIUM-CHLORIDE DIFF   NA - CL         [ the plain difference — the UAG *without* the potassium term ]
  SODIUM-POTASSIUM SUM   NA + K          [ the plain cation sum — the UAG *without* the chloride term ]

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/…/25/26. The core
confusion this rung tests is **which terms enter the gap**: dropping the potassium term (NA - CL) or
dropping the chloride term (NA + K) are exactly the two slips a student makes when reconstructing the
UAG, and both are *additive* neighbours of the true value (off by exactly one observed quantity).

Contamination-safe by construction: every formula is built only from the three observed quantities via
`+`, `-` — **no structural constants** — so every program literal is grounded in the stem. The observed
quantities carry **digit-free identifiers** (`urine_sodium`, `urine_potassium`, `urine_chloride`) so no
numeral hides inside a variable name. The five options are a tight family over the same quantities: the
three additive forms above plus the two classic *ratio* slips —

  NA / CL        the sodium-to-chloride ratio (a plausible urine-panel ratio — but the gap is a
                 difference of concentrations, not a ratio), and
  K / NA         the potassium-to-sodium ratio (the same ratio mix-up on the other pair),

which are exactly the mistakes a student makes when reaching for a ratio instead of a gap. Gold rotates
A-E by index.

Note on scale: the additive forms live on the concentration scale (tens of mEq/L) while the two ratios
are order 1, so a ratio never collides with an additive value. The tables below are chosen so the five
family values are pairwise distinct — with a comfortable margin — for every item, asserted at build time
(K != 0 and CL != 0 so the gap differs from each plain form; the two ratios kept distinct so NA*NA is
never CL*K).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (NA, K, CL) observed quantities, all in mEq/L. The five index-family values are asserted
# pairwise-distinct (with margin) below.
#   NA = urine sodium
#   K  = urine potassium
#   CL = urine chloride
TABLES = [
    (40, 20, 30),
    (30, 25, 40),
    (50, 15, 40),
    (60, 30, 50),
    (35, 20, 45),
    (45, 25, 35),
    (55, 20, 40),
]

# The option family (5 members), all built from the observed quantities via `+` / `-` / `/`. Every
# identifier is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried*
# (used as gold); all five always appear as the options.
FAMILY = [
    ("urine_anion_gap", "urine anion gap", "(urine_sodium + urine_potassium) - urine_chloride"),
    ("sodium_chloride_diff", "sodium-minus-chloride difference", "urine_sodium - urine_chloride"),
    ("sodium_potassium_sum", "sodium-plus-potassium sum", "urine_sodium + urine_potassium"),
    ("sodium_chloride_ratio", "sodium-to-chloride ratio", "urine_sodium / urine_chloride"),
    ("potassium_sodium_ratio", "potassium-to-sodium ratio", "urine_potassium / urine_sodium"),
]
QUERIED = ["urine_anion_gap", "sodium_chloride_diff", "sodium_potassium_sum"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(na, k, cl):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "urine_anion_gap": (na + k) - cl,
        "sodium_chloride_diff": na - cl,
        "sodium_potassium_sum": na + k,
        "sodium_chloride_ratio": na / cl,
        "potassium_sodium_ratio": k / na,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for na, k, cl in TABLES:
        assert k != 0 and cl != 0, (na, k, cl)  # gap differs from each plain form by one term
        fv = family_values(na, k, cl)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (na, k, cl, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, na, k, cl, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r27uag-{idx + 1:02d}",
                "qtype": "urine_anion_gap",
                "stem": (
                    f"A spot-urine electrolyte panel shows urine sodium {na} mEq/L, potassium {k} mEq/L, "
                    f"and chloride {cl} mEq/L. What is the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe urine_sodium({na})\n"
                    f"observe urine_potassium({k})\n"
                    f"observe urine_chloride({cl})\n"
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
            "ADJ-LADDER rung 27 — urine anion gap from a single spot-urine electrolyte panel (a NEW "
            "panel: urinary electrolytes / acid-base). From three stated quantities (urine sodium NA, "
            "potassium K, chloride CL) compute the urine anion gap ((NA+K)-CL), the sodium-minus-chloride "
            "difference (NA-CL), or the sodium-plus-potassium sum (NA+K). Each item is a "
            "compute_dimensioned program (observe the three quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — a two-step SUM-inside-a-difference ((NA+K)-CL) contrasted "
            "with the plain difference and plain sum, exercising the engine across + and - composed "
            "together on a fresh urinary panel — and the harness matches the scalar to the printed "
            "options. Contamination-safe: every index is built only from the three observed quantities "
            "via + and - (and / for the two ratio distractors) — no constant leaks — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same quantities, so the distractors are exactly the slips "
            "students make: dropping the potassium term (NA-CL) or the chloride term (NA+K) — each an "
            "additive neighbour off by one observed quantity — plus reaching for a ratio (NA/CL, K/NA) "
            "instead of a gap. The core confusion tested is which terms enter the gap."
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
