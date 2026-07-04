"""Generate rung-67 (ergometry specific work rate) items.json for the ADJ-LADDER.

Rung 67 opens the **sports medicine / ergometry** panel on the quantitative band — the arithmetic of a specific work
rate. A cycle-ergometer test measures the power an athlete adds above a resting baseline (the net power = peak minus
baseline), scales it by the gear ratio, and normalises to the athlete's body mass. Multiplying a DIFFERENCE by a FACTOR
and then dividing by a normaliser introduces a genuinely NEW arithmetic shape on the ladder: a **difference-times-factor
over a divisor** — `(a-b)*c/d`, i.e. `((a-b)*c)/d`.

The setup: a `peak_power`, a `baseline_power`, a `gear_ratio`, and a `body_mass`. The specific work rate is:

  SPECIFIC WORK   (peak_power - baseline_power) * gear_ratio / body_mass   [ geared net power per unit mass ]
  NET POWER       peak_power - baseline_power                              [ the numerator difference ]
  GEARED POWER    (peak_power - baseline_power) * gear_ratio               [ the scaled difference, before dividing ]

The **specific work** is what makes this rung distinctive — it is the ladder's first **difference scaled by a factor,
then divided**. Contrast the neighbours already on the ladder: rung-50 was `a*(b-c)` (a lone factor leading a
parenthesised difference — no division), rung-33 was `(a-b)*(c-d)` (a product of two differences), and rung-51 was
`a*b*c` (a bare triple product). Here a difference is scaled AND normalised. (The net power
`peak_power-baseline_power` and the geared power `(peak_power-baseline_power)*gear_ratio` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-66 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the numerator difference, the multiplication by the gear ratio, and the division by body mass —
and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as
rungs 8/16/.../65/66. This rung exercises the engine across **a scaled difference over a divisor** — the fact that
`(a-b)*c/d` is NOT `(a+b)*c/d` and NOT `(a-b)/c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `*`, and `/`
— **no structural constants** — so no numeric literal appears in any program, and neither the net power, the geared
power, nor any specific-work figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`peak_power`, `baseline_power`, `gear_ratio`, `body_mass`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (peak_power + baseline_power) * gear_ratio / body_mass   SUM the two powers instead of DIFFERENCING them
                                                                      (the classic `(a-b)*c/d` vs `(a+b)*c/d` error), and
  SWAPPED    (peak_power - baseline_power) / gear_ratio * body_mass    SWAP the multiply and divide — divide by the gear
                                                                      and multiply by the mass (`(a-b)/c*d` instead of
                                                                      `(a-b)*c/d`),

which are exactly the mistakes a student makes (adding the two powers, or swapping which quantity multiplies and which
divides). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness: all four observed quantities are strictly positive; the tables are chosen so the peak power exceeds the
baseline power (the net power — and therefore the geared power and the specific work — is positive), the gear ratio and
the body mass both exceed one and differ from each other (so net power != geared power, specific work != geared power,
and specific work != swapped), and the body mass is not the square of the gear ratio (so geared power != swapped); the
five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (PEAK_POWER, BASELINE_POWER, GEAR_RATIO, BODY_MASS) — a peak and a baseline power, a gear ratio to scale by, and a body
# mass to normalise by, all plain positive numbers with peak > baseline, gear_ratio > 1, body_mass > 1, gear_ratio !=
# body_mass, and body_mass != gear_ratio**2. The five family values are asserted pairwise-distinct (with margin) below.
TABLES = [
    (200, 80, 6, 60),
    (180, 60, 8, 80),
    (150, 50, 5, 80),
    (240, 120, 6, 90),
    (200, 40, 8, 80),
    (220, 100, 7, 60),
    (160, 40, 6, 90),
]

# The option family (5 members), all built from the four observed quantities via -, *, and /. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "specific_work",
        "specific work rate (net power geared, per unit body mass)",
        "(peak_power - baseline_power) * gear_ratio / body_mass",
    ),
    (
        "net_power",
        "the net power (peak minus baseline power)",
        "peak_power - baseline_power",
    ),
    (
        "geared_power",
        "the geared power before normalising to body mass (net power times the gear ratio)",
        "(peak_power - baseline_power) * gear_ratio",
    ),
    (
        "crossed",
        "the SUM of the two powers geared and normalised, not their difference (a wrong net power)",
        "(peak_power + baseline_power) * gear_ratio / body_mass",
    ),
    (
        "swapped",
        "the net power DIVIDED by the gear ratio and MULTIPLIED by the body mass, the multiply and divide swapped (a wrong scaling)",
        "(peak_power - baseline_power) / gear_ratio * body_mass",
    ),
]
QUERIED = ["specific_work", "net_power", "geared_power"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(peak_power, baseline_power, gear_ratio, body_mass):
    # Operation order mirrors the ADJ programs exactly (the parenthesised difference formed first, then the left-to-right
    # multiply-then-divide), so the Python option value and the engine result are the same IEEE-double (well within the
    # harness's 1e-9 match tolerance).
    return {
        "specific_work": (peak_power - baseline_power) * gear_ratio / body_mass,
        "net_power": peak_power - baseline_power,
        "geared_power": (peak_power - baseline_power) * gear_ratio,
        "crossed": (peak_power + baseline_power) * gear_ratio / body_mass,
        "swapped": (peak_power - baseline_power) / gear_ratio * body_mass,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for peak_power, baseline_power, gear_ratio, body_mass in TABLES:
        assert (
            peak_power > 0
            and baseline_power > 0
            and gear_ratio > 0
            and body_mass > 0
        ), (peak_power, baseline_power, gear_ratio, body_mass)
        # Net power must be positive (numerator). The gear ratio and body mass exceed one and differ,
        # and body mass is not the gear ratio squared, so no two family members collide structurally.
        assert peak_power > baseline_power, (peak_power, baseline_power, gear_ratio, body_mass)
        assert gear_ratio > 1 and body_mass > 1, (peak_power, baseline_power, gear_ratio, body_mass)
        assert gear_ratio != body_mass, (peak_power, baseline_power, gear_ratio, body_mass)
        assert body_mass != gear_ratio * gear_ratio, (peak_power, baseline_power, gear_ratio, body_mass)
        fv = family_values(peak_power, baseline_power, gear_ratio, body_mass)
        for key, v in fv.items():
            assert v > 0, (key, peak_power, baseline_power, gear_ratio, body_mass, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    peak_power,
                    baseline_power,
                    gear_ratio,
                    body_mass,
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
                peak_power,
                baseline_power,
                gear_ratio,
                body_mass,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r67ergo-{idx + 1:02d}",
                "qtype": "ergometry_specific_work",
                "stem": (
                    f"An ergometer test records a peak power of {num(peak_power)} and a baseline of "
                    f"{num(baseline_power)}, scaled by a gear ratio of {num(gear_ratio)} and normalised to a body mass "
                    f"of {num(body_mass)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe peak_power({num(peak_power)})\n"
                    f"observe baseline_power({num(baseline_power)})\n"
                    f"observe gear_ratio({num(gear_ratio)})\n"
                    f"observe body_mass({num(body_mass)})\n"
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
            "ADJ-LADDER rung 67 — specific work rate from four stated quantities (a NEW panel: sports medicine / "
            "ergometry). From a peak and a baseline power (their difference is the net power), a gear ratio to scale by, "
            "and a body mass to normalise by, compute the specific work "
            "((peak_power-baseline_power)*gear_ratio/body_mass), the net power (peak_power-baseline_power), or the "
            "geared power ((peak_power-baseline_power)*gear_ratio). Each item is a compute_dimensioned program (observe "
            "the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, "
            "DIFFERENCE-TIMES-FACTOR OVER A DIVISOR (a-b)*c/d, the first on the ladder to scale a difference by a factor "
            "and then divide (distinct from rung-50 a*(b-c) with the factor leading, rung-33 (a-b)*(c-d), and rung-51 "
            "a*b*c) — and the harness matches the scalar to the printed options. Contamination-safe: every index is "
            "built only from the four observed quantities via -, *, and / — no constant leaks, and neither the net "
            "power, the geared power, nor any specific-work figure ever appears as a literal (each is computed) — and "
            "the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same four quantities, so the distractors are exactly the slips students "
            "make: SUMMING the two powers ((a+b)*c/d, a wrong net power) and SWAPPING the multiply and divide "
            "((a-b)/c*d, a wrong scaling). The core confusion tested is that (a-b)*c/d is not (a+b)*c/d and not "
            "(a-b)/c*d."
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
