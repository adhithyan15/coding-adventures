"""Generate rung-32 (respiratory exchange ratio) items.json for the ADJ-LADDER.

Rung 32 opens the **pulmonary gas-exchange / respiratory** panel on the quantitative band — the arithmetic
of the respiratory exchange ratio (R), the bedside physiology that compares how much CO2 the blood picks up
across the lung to how much O2 it gives up (R ≈ 0.8 on a mixed diet). It uses the same contamination-safe shape
as the Starling rung (31), the body-mass rung (30), and the serum-protein rung (29): a small table of *observed*
blood-gas contents and a tight family of mutually-confusable formulas built **only from those observed
quantities** (no numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single pair of blood samples across the lung. FOUR gas contents are measured (all in
vol %, mL of gas per dL of blood):

  ARTERIAL_O2    Ca_O2   oxygen content of arterial (post-lung) blood      (higher — the lung loaded it)
  VENOUS_O2      Cv_O2   oxygen content of mixed-venous (pre-lung) blood   (lower — the tissues used it)
  ARTERIAL_CO2   Ca_CO2  CO2 content of arterial blood                     (lower — the lung unloaded it)
  VENOUS_CO2     Cv_CO2  CO2 content of mixed-venous blood                 (higher — the tissues added it)

The respiratory exchange ratio is the **CO2 unloaded divided by the O2 loaded** — a *ratio of two
differences* — `(VENOUS_CO2 − ARTERIAL_CO2) / (ARTERIAL_O2 − VENOUS_O2)`. That is what makes this rung
distinctive: it is a NEW arithmetic shape on the ladder — a quotient whose numerator AND denominator are each
their own difference (rung-29 divided *by* a difference; rung-31 subtracted one difference from another; this
rung divides *one difference by another*). The core confusion this rung tests is pairing the right gas in the
numerator (CO2 output) over the right gas in the denominator (O2 uptake), and getting each difference's
direction right:

  RESPIRATORY EXCHANGE RATIO   (VENOUS_CO2 − ARTERIAL_CO2) / (ARTERIAL_O2 − VENOUS_O2)   [ R = VCO2 / VO2 ]
  CO2 OUTPUT                    VENOUS_CO2 − ARTERIAL_CO2                                 [ the CO2 added, numerator ]
  O2 UPTAKE                     ARTERIAL_O2 − VENOUS_O2                                   [ the O2 removed, denominator ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/30/31. This rung exercises the engine across a
DIVISION of two parenthesised DIFFERENCES.

Contamination-safe by construction: every formula is built only from the four observed contents via `-`, `/`
— **no structural constants** — so every program literal is grounded in the stem. Neither the CO2 output nor
the O2 uptake ever appears as a literal (each is computed from the observed contents). The observed quantities
carry **digit-free identifiers** (`arterial_o2_content`, `venous_o2_content`, `arterial_co2_content`,
`venous_co2_content`) so no numeral hides inside a variable name. The five options are a tight family over the
same quantities: the three real indices plus the two classic slips —

  INVERTED RATIO   (ARTERIAL_O2 − VENOUS_O2) / (VENOUS_CO2 − ARTERIAL_CO2)   the *upside-down* R (O2 over CO2 = 1/R), and
  CROSSED RATIO    (VENOUS_CO2 − ARTERIAL_CO2) / (VENOUS_O2 − ARTERIAL_O2)   the O2 difference taken the WRONG way (= −R),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: R and its inverse are order 1 (R ≈ 0.7-1.0, 1/R ≈ 1.0-1.4), the two gas differences live on the
single-digit vol-% scale (a few mL/dL), and the crossed ratio is R's negative — four positive magnitudes plus
one negative, so no two family values collide; the tables below are chosen so the five family values are
pairwise distinct — with a comfortable margin — for every item, asserted at build time (CO2 output != O2 uptake
so R != 1 and R never equals its inverse; both differences positive so no division by zero and R never equals
its own negative).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (ARTERIAL_O2, VENOUS_O2, ARTERIAL_CO2, VENOUS_CO2) observed gas contents, all in vol % (mL/dL). Arterial O2
# exceeds venous O2 (the lung loaded it) and venous CO2 exceeds arterial CO2 (the tissues added it), so both
# differences are strictly positive. CO2 output != O2 uptake on every row (so R != 1 and R != 1/R). The five
# family values are asserted pairwise-distinct (with margin) below.
#   Ca_O2  = arterial oxygen content        (post-lung, higher)
#   Cv_O2  = mixed-venous oxygen content    (pre-lung, lower)
#   Ca_CO2 = arterial CO2 content           (post-lung, lower)
#   Cv_CO2 = mixed-venous CO2 content       (pre-lung, higher)
TABLES = [
    (20, 15, 44, 48),
    (19, 12, 45, 50),
    (21, 14, 46, 52),
    (18, 12, 43, 48),
    (24, 15, 44, 52),
    (21, 13, 43, 50),
    (26, 16, 46, 54),
]

# The option family (5 members), all built from the observed quantities via `-` / `/`. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    (
        "respiratory_exchange_ratio",
        "respiratory exchange ratio",
        "(venous_co2_content - arterial_co2_content) / (arterial_o2_content - venous_o2_content)",
    ),
    (
        "co2_output",
        "CO2 output across the lung",
        "venous_co2_content - arterial_co2_content",
    ),
    (
        "o2_uptake",
        "O2 uptake across the lung",
        "arterial_o2_content - venous_o2_content",
    ),
    (
        "inverted_ratio",
        "inverted exchange ratio (O2 uptake over CO2 output)",
        "(arterial_o2_content - venous_o2_content) / (venous_co2_content - arterial_co2_content)",
    ),
    (
        "crossed_ratio",
        "crossed ratio (CO2 output over the reversed O2 difference)",
        "(venous_co2_content - arterial_co2_content) / (venous_o2_content - arterial_o2_content)",
    ),
]
QUERIED = ["respiratory_exchange_ratio", "co2_output", "o2_uptake"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(ca_o2, cv_o2, ca_co2, cv_co2):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    o2_uptake = ca_o2 - cv_o2
    co2_output = cv_co2 - ca_co2
    return {
        "respiratory_exchange_ratio": co2_output / o2_uptake,
        "co2_output": co2_output,
        "o2_uptake": o2_uptake,
        "inverted_ratio": o2_uptake / co2_output,
        "crossed_ratio": co2_output / (cv_o2 - ca_o2),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for ca_o2, cv_o2, ca_co2, cv_co2 in TABLES:
        o2_uptake = ca_o2 - cv_o2
        co2_output = cv_co2 - ca_co2
        assert o2_uptake > 0 and co2_output > 0, (ca_o2, cv_o2, ca_co2, cv_co2)
        assert co2_output != o2_uptake, (ca_o2, cv_o2, ca_co2, cv_co2)  # else R == 1 == 1/R
        fv = family_values(ca_o2, cv_o2, ca_co2, cv_co2)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (ca_o2, cv_o2, ca_co2, cv_co2, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, ca_o2, cv_o2, ca_co2, cv_co2, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r32resp-{idx + 1:02d}",
                "qtype": "respiratory_exchange_ratio",
                "stem": (
                    f"Paired blood samples across the lung show an arterial oxygen content of {num(ca_o2)} "
                    f"vol% and a mixed-venous oxygen content of {num(cv_o2)} vol%; the arterial CO2 content is "
                    f"{num(ca_co2)} vol% and the mixed-venous CO2 content is {num(cv_co2)} vol%. What is the "
                    f"patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe arterial_o2_content({num(ca_o2)})\n"
                    f"observe venous_o2_content({num(cv_o2)})\n"
                    f"observe arterial_co2_content({num(ca_co2)})\n"
                    f"observe venous_co2_content({num(cv_co2)})\n"
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
            "ADJ-LADDER rung 32 — respiratory exchange ratio from paired blood-gas contents across the lung "
            "(a NEW panel: pulmonary gas exchange / respiratory). From four stated contents (arterial O2 "
            "Ca_O2, venous O2 Cv_O2, arterial CO2 Ca_CO2, venous CO2 Cv_CO2) compute the respiratory exchange "
            "ratio ((Cv_CO2-Ca_CO2)/(Ca_O2-Cv_O2)), the CO2 output (Cv_CO2-Ca_CO2), or the O2 uptake "
            "(Ca_O2-Cv_O2). Each item is a compute_dimensioned program (observe the four quantities, let "
            "answer = formula); the ADJ engine carries the arithmetic — a NEW shape, a RATIO OF TWO "
            "DIFFERENCES ((Cv_CO2-Ca_CO2)/(Ca_O2-Cv_O2)), so one parenthesised difference is divided by "
            "another — and the harness matches the scalar to the printed options. Contamination-safe: every "
            "index is built only from the four observed contents via - and / — no constant leaks (R is a pure "
            "ratio), and neither the CO2 output nor the O2 uptake ever appears as a literal (each is computed "
            "from the observed contents) — and the observed quantities carry digit-free identifiers so no "
            "numeral hides inside a variable name. The five options are a family over the same quantities, so "
            "the distractors are exactly the slips students make: the inverted ratio "
            "((Ca_O2-Cv_O2)/(Cv_CO2-Ca_CO2), O2 over CO2 = 1/R) and the crossed ratio "
            "((Cv_CO2-Ca_CO2)/(Cv_O2-Ca_O2), the O2 difference reversed = -R). The core confusion tested is "
            "pairing CO2 output over O2 uptake with each difference in the right direction."
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
