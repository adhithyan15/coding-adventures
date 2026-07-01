"""Generate rung-31 (Starling net filtration pressure) items.json for the ADJ-LADDER.

Rung 31 opens the **capillary fluid-exchange / microcirculation** panel on the quantitative band — the
arithmetic of the Starling net filtration pressure, the bedside physiology that decides whether fluid moves OUT
of a capillary (filtration → edema) or back IN (reabsorption). It uses the same contamination-safe shape as the
body-mass rung (30), the serum-protein rung (29), and the coagulation rung (28): a small table of *observed*
pressures and a tight family of mutually-confusable formulas built **only from those observed quantities** (no
numeric literal anywhere in any program), so nothing structural can leak.

The clinical setup is a single capillary. FOUR pressures are measured (all in mmHg):

  CAPILLARY_HYDROSTATIC     Pc   blood pressure pushing fluid OUT of the capillary
  INTERSTITIAL_HYDROSTATIC  Pi   tissue pressure pushing fluid back IN
  CAPILLARY_ONCOTIC         πc   plasma-protein pull holding fluid IN
  INTERSTITIAL_ONCOTIC      πi   tissue-protein pull drawing fluid OUT

The net filtration pressure is the **hydrostatic gradient minus the oncotic gradient** — a *difference of two
differences* — `(Pc − Pi) − (πc − πi)`. That is what makes this rung distinctive: it is a NEW arithmetic shape
on the ladder — a difference whose two operands are THEMSELVES differences (rung-24 put a difference in the
numerator; rung-27 summed inside a difference; rung-29 divided by a difference; rung-30 divided by a square;
this rung *subtracts one difference from another*). The core confusion this rung tests is remembering that the
oncotic gradient is *subtracted* (it opposes filtration), not added:

  NET FILTRATION PRESSURE   (Pc − Pi) − (πc − πi)  [ >0 → filtration, <0 → reabsorption ]
  HYDROSTATIC GRADIENT       Pc − Pi               [ the outward push alone ]
  ONCOTIC GRADIENT           πc − πi               [ the inward pull alone ]

Each index is a `compute_dimensioned` program (observe the four quantities + `let answer = formula`); the ADJ
engine carries the arithmetic and the harness reads the scalar via the existing `compute_dimensioned`
extractor — no harness/engine change, exactly as rungs 8/16/…/29/30. This rung exercises the engine across a
SUBTRACTION of two parenthesised DIFFERENCES.

Contamination-safe by construction: every formula is built only from the four observed quantities via `-`, `+`
— **no structural constants** — so every program literal is grounded in the stem. Neither gradient value ever
appears as a literal (each is computed from the observed pressures). The observed quantities carry **digit-free
identifiers** (`capillary_hydrostatic`, `interstitial_hydrostatic`, `capillary_oncotic`, `interstitial_oncotic`)
so no numeral hides inside a variable name. The five options are a tight family over the same quantities: the
three real indices plus the two classic slips —

  TOTAL PRESSURE SUM      (Pc − Pi) + (πc − πi)   the *sum* of the gradients (adding instead of subtracting), and
  REVERSED NET            (πc − πi) − (Pc − Pi)   the *inverted* net (oncotic minus hydrostatic, sign flipped),

which are exactly the mistakes a student makes. Gold rotates A-E by index.

Note on scale: the net filtration pressure is a small positive number (a few mmHg, favouring filtration at the
arteriolar end), the reversed net is its negative, the two gradients live on the tens-of-mmHg scale, and the sum
is the largest — five well-separated magnitudes, so no two family values collide; the tables below are chosen so
the five family values are pairwise distinct — with a comfortable margin — for every item, asserted at build
time (net != 0 so the net and its reversal never coincide).
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CAPILLARY_HYDROSTATIC, INTERSTITIAL_HYDROSTATIC, CAPILLARY_ONCOTIC, INTERSTITIAL_ONCOTIC) observed pressures,
# all in mmHg. The five index-family values are asserted pairwise-distinct (with margin) below. Each row keeps
# the net filtration pressure strictly positive (arteriolar end) so the net and its sign-flipped reversal never
# coincide.
#   Pc  = capillary hydrostatic pressure   (pushes fluid OUT)
#   Pi  = interstitial hydrostatic pressure (pushes fluid IN)
#   PIc = capillary (plasma) oncotic pressure (holds fluid IN)
#   PIi = interstitial oncotic pressure (draws fluid OUT)
TABLES = [
    (35, 2, 28, 8),
    (30, 3, 25, 6),
    (40, 5, 30, 10),
    (25, 4, 22, 7),
    (38, 6, 26, 9),
    (33, 3, 24, 5),
    (36, 4, 27, 7),
]

# The option family (5 members), all built from the observed quantities via `-` / `+`. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all
# five always appear as the options.
FAMILY = [
    (
        "net_filtration_pressure",
        "net filtration pressure",
        "(capillary_hydrostatic - interstitial_hydrostatic) - (capillary_oncotic - interstitial_oncotic)",
    ),
    (
        "hydrostatic_gradient",
        "hydrostatic pressure gradient",
        "capillary_hydrostatic - interstitial_hydrostatic",
    ),
    (
        "oncotic_gradient",
        "oncotic pressure gradient",
        "capillary_oncotic - interstitial_oncotic",
    ),
    (
        "total_pressure_sum",
        "sum of the hydrostatic and oncotic gradients",
        "(capillary_hydrostatic - interstitial_hydrostatic) + (capillary_oncotic - interstitial_oncotic)",
    ),
    (
        "reversed_net",
        "reversed net pressure (oncotic gradient minus hydrostatic gradient)",
        "(capillary_oncotic - interstitial_oncotic) - (capillary_hydrostatic - interstitial_hydrostatic)",
    ),
]
QUERIED = ["net_filtration_pressure", "hydrostatic_gradient", "oncotic_gradient"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(pc, pi, pic, pii):
    # Operation order mirrors the ADJ program exactly, so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    hyd = pc - pi
    onc = pic - pii
    return {
        "net_filtration_pressure": hyd - onc,
        "hydrostatic_gradient": hyd,
        "oncotic_gradient": onc,
        "total_pressure_sum": hyd + onc,
        "reversed_net": onc - hyd,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for pc, pi, pic, pii in TABLES:
        assert pc - pi != pic - pii, (pc, pi, pic, pii)  # else the net is 0 and coincides with its reversal
        fv = family_values(pc, pi, pic, pii)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (pc, pi, pic, pii, ORDER[i], ORDER[j], fv)
        for key in QUERIED:
            gold_val = fv[key]
            gold_pos = idx % 5
            others = [fv[k2] for k2 in ORDER if abs(fv[k2] - gold_val) > 1e-12]
            opts_vals = others[:]
            opts_vals.insert(gold_pos, gold_val)
            opts_vals = opts_vals[:5]
            if abs(opts_vals[gold_pos] - gold_val) > 1e-12:
                opts_vals[gold_pos] = gold_val
            assert len({round(v, 9) for v in opts_vals}) == 5, (key, pc, pi, pic, pii, opts_vals)
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r31starling-{idx + 1:02d}",
                "qtype": "starling_filtration",
                "stem": (
                    f"A capillary has a hydrostatic pressure of {num(pc)} mmHg and the surrounding "
                    f"interstitium a hydrostatic pressure of {num(pi)} mmHg; the plasma oncotic pressure is "
                    f"{num(pic)} mmHg and the interstitial oncotic pressure is {num(pii)} mmHg. What is the "
                    f"patient's {name_of[key]}?"
                ),
                "program": (
                    f"observe capillary_hydrostatic({num(pc)})\n"
                    f"observe interstitial_hydrostatic({num(pi)})\n"
                    f"observe capillary_oncotic({num(pic)})\n"
                    f"observe interstitial_oncotic({num(pii)})\n"
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
            "ADJ-LADDER rung 31 — Starling net filtration pressure from a single capillary (a NEW panel: "
            "capillary fluid exchange / microcirculation). From four stated pressures (capillary hydrostatic "
            "Pc, interstitial hydrostatic Pi, capillary/plasma oncotic PIc, interstitial oncotic PIi) compute "
            "the net filtration pressure ((Pc-Pi)-(PIc-PIi)), the hydrostatic gradient (Pc-Pi), or the oncotic "
            "gradient (PIc-PIi). Each item is a compute_dimensioned program (observe the four quantities, let "
            "answer = formula); the ADJ engine carries the arithmetic — a NEW shape, a DIFFERENCE OF TWO "
            "DIFFERENCES ((Pc-Pi)-(PIc-PIi)), so one parenthesised gradient is subtracted from another — and "
            "the harness matches the scalar to the printed options. Contamination-safe: every index is built "
            "only from the four observed pressures via - and + — no constant leaks, and neither gradient ever "
            "appears as a literal (each is computed from the observed pressures) — and the observed quantities "
            "carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same quantities, so the distractors are exactly the slips students make: the sum "
            "of the gradients ((Pc-Pi)+(PIc-PIi), adding instead of subtracting) and the reversed net "
            "((PIc-PIi)-(Pc-Pi), sign flipped). The core confusion tested is remembering that the oncotic "
            "gradient is subtracted (it opposes filtration), not added."
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
