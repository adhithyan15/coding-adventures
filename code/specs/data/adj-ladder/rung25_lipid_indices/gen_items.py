"""Generate rung-25 (lipid indices) items.json for the ADJ-LADDER.

Rung 25 opens the **lipid-panel** on the quantitative band — the cardiovascular-risk arithmetic of a
fasting lipid profile (atherogenic ratios, insulin-resistance surrogate, non-HDL cholesterol) — using
the same contamination-safe shape as the oxygen rung (24) and the iron rung (23): a small table of
*observed* laboratory quantities and a tight family of mutually-confusable formulas built **only from
those observed quantities** (no numeric literal anywhere in any program), so nothing structural can
leak.

The clinical setup is a single fasting lipid panel. Three quantities are measured:

  TC    total cholesterol     (mg/dL)
  HDL   HDL cholesterol       (mg/dL) — the "good" fraction
  TG    triglycerides         (mg/dL)

Three textbook lipid indices fall out as pure functions of the observed quantities — no constant
required. Two are **pure ratios** and one puts a **derived difference in the numerator** (non-HDL
cholesterol = TC − HDL, computed from its two observed components, never a constant — the same
subtraction-in-numerator shape the oxygen rung used):

  TC:HDL RATIO         TC / HDL            [ atherogenic / cardiac-risk ratio; higher = worse ]
  TG:HDL RATIO         TG / HDL            [ insulin-resistance surrogate; higher = worse ]
  NON-HDL FRACTION     (TC − HDL) / TC     [ =non-HDL/TC; the atherogenic-cholesterol share ]

Each index is a `compute_dimensioned` program (observe the three quantities + `let answer = formula`);
the ADJ engine carries the arithmetic and the harness reads the scalar via the existing
`compute_dimensioned` extractor — no harness/engine change, exactly as rungs 8/16/18/19/20/21/22/23/24.
This exercises the engine across **division AND an inner subtraction-in-parentheses** (non-HDL =
TC − HDL) on a fresh lipid-panel stem, mixing two pure ratios with one difference-in-numerator index.

Contamination-safe by construction: every formula is built only from the three observed quantities via
`/`, `−`, and grouping `( )` — **no structural constants** (non-HDL is computed, not observed, so no
x-factor appears) — so every program literal is grounded in the stem. The observed quantities carry
**digit-free identifiers** (`total_cholesterol`, `hdl`, `triglycerides`) so no numeral hides inside a
variable name. The five options are a tight family over the same quantities: the three real indices plus
the two classic slips —

  HDL / TC             the **inverse** TC:HDL ratio (the HDL fraction, written upside-down), and
  TC / TG              the total-cholesterol-to-triglyceride ratio (a plausible but non-standard mix),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on table choice: the five family values can collide for special quantity ratios. The tables below
are chosen so the five family values are pairwise distinct — with a comfortable margin — for every item,
asserted at build time. HDL is always below TC (a physiologic panel), so the non-HDL fraction stays a
fraction in (0, 1).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (TC, HDL, TG) observed quantities, all mg/dL. The five index-family values are asserted
# pairwise-distinct (with margin) below. TC > HDL for every row (non-HDL fraction in (0, 1)).
#   TC  = total cholesterol
#   HDL = HDL cholesterol
#   TG  = triglycerides
TABLES = [
    (200, 50, 150),
    (240, 40, 200),
    (180, 60, 90),
    (220, 55, 100),
    (150, 50, 100),
    (210, 42, 168),
    (250, 50, 125),
]

# The option family (5 members), all built from the observed quantities via `/`, `-`, grouping. Every
# identifier is DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried*
# (used as gold); all five always appear as the options.
FAMILY = [
    ("tc_hdl", "total-cholesterol-to-HDL ratio", "total_cholesterol / hdl"),
    ("tg_hdl", "triglyceride-to-HDL ratio", "triglycerides / hdl"),
    ("non_hdl_frac", "non-HDL fraction of total cholesterol",
     "(total_cholesterol - hdl) / total_cholesterol"),
    ("hdl_tc", "HDL fraction of total cholesterol", "hdl / total_cholesterol"),
    ("tc_tg", "total-cholesterol-to-triglyceride ratio", "total_cholesterol / triglycerides"),
]
QUERIED = ["tc_hdl", "tg_hdl", "non_hdl_frac"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(tc, hdl, tg):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "tc_hdl": tc / hdl,
        "tg_hdl": tg / hdl,
        "non_hdl_frac": (tc - hdl) / tc,
        "hdl_tc": hdl / tc,
        "tc_tg": tc / tg,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0
    for tc, hdl, tg in TABLES:
        assert tc > hdl, (tc, hdl)  # non-HDL fraction must be a fraction in (0, 1)
        fv = family_values(tc, hdl, tg)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[k] for k in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (tc, hdl, tg, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k] for k in ORDER if abs(fv[k] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, tc, hdl, tg, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r25lip-{idx + 1:02d}",
                "qtype": "lipid_index",
                "stem": (
                    f"A fasting lipid panel shows total cholesterol {tc} mg/dL, HDL cholesterol {hdl} "
                    f"mg/dL, and triglycerides {tg} mg/dL. What is the patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe total_cholesterol({tc})\n"
                    f"observe hdl({hdl})\n"
                    f"observe triglycerides({tg})\n"
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
            "ADJ-LADDER rung 25 — lipid indices from a single fasting lipid panel (a NEW panel: lipid "
            "metabolism / cardiovascular risk). From three stated quantities (total cholesterol TC, HDL "
            "cholesterol HDL, triglycerides TG) compute the TC:HDL atherogenic ratio (TC/HDL), the TG:HDL "
            "insulin-resistance surrogate (TG/HDL), or the non-HDL fraction ((TC-HDL)/TC). Each item is a "
            "compute_dimensioned program (observe the three quantities, let answer = formula); the ADJ "
            "engine carries the arithmetic — two pure divisions AND an inner difference-in-parentheses for "
            "the non-HDL cholesterol (non-HDL = TC - HDL) — and the harness matches the scalar to the "
            "printed options. Contamination-safe: every index is built only from the three observed "
            "quantities via /, -, and grouping — no constant leaks (non-HDL is derived from its components, "
            "not observed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same quantities, so the "
            "distractors are exactly the slips students make: the inverse TC:HDL ratio (HDL/TC, the HDL "
            "fraction written upside-down) and the total-cholesterol-to-triglyceride ratio (TC/TG, a "
            "plausible but non-standard mix)."
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
