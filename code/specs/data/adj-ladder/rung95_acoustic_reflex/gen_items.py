"""Generate rung-95 (audiology / acoustic-reflex net drive) items.json for the ADJ-LADDER.

Rung 95 opens the **audiology / acoustic-reflex** panel on the quantitative band — the arithmetic of a net acoustic-reflex
response. A `stimulus_level` MINUS a `threshold_level` gives the sensation drive (how far the stimulus is above threshold),
an `ipsi_gain` PLUS a `contra_gain` gives the total pathway gain, and the drive MULTIPLIES the gain sum into the net
reflex. A **difference times a sum** introduces a genuinely NEW arithmetic family on the ladder: `(a-b)*(c+d)`, i.e.
`((a-b)*(c+d))`.

This is genuinely new and COMPLETES the binomial-product 4-corner square. Rung-89 shipped `(a+b)*(c-d)` (sum ×
difference), rung-90 `(a-b)*(c-d)` (difference × difference), rung-94 `(a+b)*(c+d)` (sum × sum); the fourth corner,
**difference × sum** `(a-b)*(c+d)`, was the only one left. Rung 95 fills it — the last "binomial times binomial" with one
difference and one sum. No prior shape multiplied a difference by a sum: rung-84 `(a+b)*c-d` multiplied a sum by a bare
factor, rung-35 `a*b-c*d` subtracted two products. The operator order matters: `(a-b)*(c+d)` is `((a-b)*(c+d))` (each
binomial forms first inside its parentheses, then the two are multiplied), NOT `(a-b)*c+d` (distributing only the first
gain and leaving the second added bare) and NOT `a*(c+d)-b` (multiplying the WHOLE stimulus by the gain sum and only then
subtracting the bare threshold) — the two distractors exploit exactly those confusions.

The setup: a `stimulus_level`, a `threshold_level`, an `ipsi_gain`, and a `contra_gain`. The total is:

  NET REFLEX     (stimulus_level - threshold_level) * (ipsi_gain + contra_gain)  [ a difference times a sum ]
  SENSATION DRIVE stimulus_level - threshold_level                              [ the difference, before the product ]
  GAIN SUM       ipsi_gain + contra_gain                                        [ the sum, before the product ]

The **net reflex** is what makes this rung distinctive — it is the ladder's first **difference times a sum** and the
corner that completes the binomial-product square. (The sensation drive `a-b` and the gain sum `c+d` ride alongside as
component readouts, so the panel teaches the whole calculation — exactly as rungs 47-94 shipped their component
sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the threshold from the stimulus into the drive, the addition of the two gains
into the gain sum, then the multiplication of the difference by the sum (each binomial forming inside its parentheses
before the product, so (a-b)*(c+d) evaluates as ((a-b)*(c+d))) — and the harness reads the scalar via the existing
`compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../93/94. This rung exercises the engine
across a **difference times a sum** — the fact that `(a-b)*(c+d)` is `((a-b)*(c+d))` and NOT `(a-b)*c+d` and NOT
`a*(c+d)-b` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `-`, `+` and `*` —
**no structural constants** — so no numeric literal appears in any program, and neither the sensation drive, the gain
sum, nor any net figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`stimulus_level`, `threshold_level`, `ipsi_gain`, `contra_gain`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (stimulus_level - threshold_level) * ipsi_gain + contra_gain  distribute only the ipsi gain and leave the
                                                                           contra gain added bare, instead of multiplying
                                                                           the drive by the whole gain sum (the classic
                                                                           `(a-b)*(c+d)` vs `(a-b)*c+d` error), and
  SWAPPED    stimulus_level * (ipsi_gain + contra_gain) - threshold_level  multiply the WHOLE stimulus by the gain sum and
                                                                           only then subtract the bare threshold, instead
                                                                           of forming the drive first (`a*(c+d)-b` instead
                                                                           of `(a-b)*(c+d)`),

which are exactly the mistakes a student makes (distributing a product across only part of a sum, or subtracting outside
the parenthesised binomial). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always
appear as options.

Distinctness and positivity: every stimulus exceeds its threshold by at least two (`stimulus_level >= threshold_level + 2`)
and every quantity is a plain positive number >= 2, so the sensation drive is >= 2 and every family member — a product of
a positive difference and a positive sum, or a positive combination — is automatically strictly positive; the
`>= threshold_level + 2` margin also keeps the net reflex distinct from the crossed slip (they would coincide only if the
drive were exactly one). The tables are chosen so the five family values are pairwise distinct with a comfortable margin
(they also avoid sensation-drive == gain-sum), asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (STIMULUS_LEVEL, THRESHOLD_LEVEL, IPSI_GAIN, CONTRA_GAIN) — a stimulus level minus a threshold level for the sensation
# drive (stimulus_level >= threshold_level + 2 so the drive is a positive number >= 2), and an ipsi gain plus a contra
# gain for the total pathway gain, all plain positive numbers >= 2. Every family member is a product of a positive
# difference and a positive sum / a positive combination, so positivity is automatic; the five family values are asserted
# pairwise-distinct below. The >= threshold+2 margin keeps the net reflex distinct from the crossed slip, and the tables
# avoid sensation-drive == gain-sum.
TABLES = [
    (5, 2, 3, 4),
    (6, 2, 2, 5),
    (5, 3, 4, 2),
    (7, 2, 3, 5),
    (6, 4, 2, 3),
    (8, 2, 5, 3),
    (5, 2, 4, 6),
]

# The option family (5 members), all built from the four observed quantities via -, + and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "net_reflex",
        "net acoustic reflex (the sensation drive times the gain sum)",
        "(stimulus_level - threshold_level) * (ipsi_gain + contra_gain)",
    ),
    (
        "sensation_drive",
        "the sensation drive (the stimulus level minus the threshold level, before multiplying by the gain sum)",
        "stimulus_level - threshold_level",
    ),
    (
        "gain_sum",
        "the gain sum (the ipsi gain plus the contra gain, before multiplying by the sensation drive)",
        "ipsi_gain + contra_gain",
    ),
    (
        "crossed",
        "the stimulus level minus the threshold level, times the ipsi gain, plus the contra gain, distributing only the ipsi gain and leaving the contra gain added bare (a wrong grouping)",
        "(stimulus_level - threshold_level) * ipsi_gain + contra_gain",
    ),
    (
        "swapped",
        "the stimulus level times the gain sum, minus the threshold level, multiplying the whole stimulus by the gain sum and only then subtracting the threshold (a wrong pairing)",
        "stimulus_level * (ipsi_gain + contra_gain) - threshold_level",
    ),
]
QUERIED = ["net_reflex", "sensation_drive", "gain_sum"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(stimulus_level, threshold_level, ipsi_gain, contra_gain):
    # Operation order mirrors the ADJ programs exactly (each binomial forms inside its parentheses first, then the two are
    # multiplied, so (a-b)*(c+d) evaluates as ((a-b)*(c+d))), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "net_reflex": (stimulus_level - threshold_level) * (ipsi_gain + contra_gain),
        "sensation_drive": stimulus_level - threshold_level,
        "gain_sum": ipsi_gain + contra_gain,
        "crossed": (stimulus_level - threshold_level) * ipsi_gain + contra_gain,
        "swapped": stimulus_level * (ipsi_gain + contra_gain) - threshold_level,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for stimulus_level, threshold_level, ipsi_gain, contra_gain in TABLES:
        # Every stimulus exceeds its threshold by at least two, so the sensation drive is a positive number >= 2.
        assert (
            stimulus_level >= threshold_level + 2
            and threshold_level > 0
            and ipsi_gain > 0
            and contra_gain > 0
        ), (stimulus_level, threshold_level, ipsi_gain, contra_gain)
        fv = family_values(stimulus_level, threshold_level, ipsi_gain, contra_gain)
        # Every family member is a product of a positive difference and a positive sum / a positive combination, so every
        # value is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, stimulus_level, threshold_level, ipsi_gain, contra_gain, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    stimulus_level,
                    threshold_level,
                    ipsi_gain,
                    contra_gain,
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
                stimulus_level,
                threshold_level,
                ipsi_gain,
                contra_gain,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r95reflex-{idx + 1:02d}",
                "qtype": "acoustic_reflex_net",
                "stem": (
                    f"An acoustic-reflex study records a stimulus level of {num(stimulus_level)} minus a threshold "
                    f"level of {num(threshold_level)}, all times an ipsi gain of {num(ipsi_gain)} plus a contra gain "
                    f"of {num(contra_gain)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe stimulus_level({num(stimulus_level)})\n"
                    f"observe threshold_level({num(threshold_level)})\n"
                    f"observe ipsi_gain({num(ipsi_gain)})\n"
                    f"observe contra_gain({num(contra_gain)})\n"
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
            "ADJ-LADDER rung 95 — acoustic-reflex net drive from four stated quantities (a NEW panel: audiology / "
            "acoustic-reflex). From a stimulus level minus a threshold level for the sensation drive and an ipsi gain "
            "plus a contra gain for the total pathway gain, compute the net reflex "
            "((stimulus_level-threshold_level)*(ipsi_gain+contra_gain)), the sensation drive "
            "(stimulus_level-threshold_level), or the gain sum (ipsi_gain+contra_gain). Each item is a compute_dimensioned "
            "program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW "
            "family, A DIFFERENCE TIMES A SUM (a-b)*(c+d) (subtract b from a, add c and d, multiply the difference by the "
            "sum, so (a-b)*(c+d) = ((a-b)*(c+d)); this COMPLETES the binomial-product 4-corner square — rung-89 (a+b)*(c-d) "
            "sum-times-difference, rung-90 (a-b)*(c-d) difference-times-difference, rung-94 (a+b)*(c+d) sum-times-sum, and "
            "difference-times-sum was the last corner; no prior shape multiplied a difference by a sum, e.g. rung-84 "
            "(a+b)*c-d multiplied a sum by a bare factor and rung-35 a*b-c*d subtracted two products) — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every figure is built only from the four "
            "observed quantities via -, + and * — no constant leaks, and neither the sensation drive, the gain sum, nor "
            "any net figure ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: distributing only the ipsi gain and "
            "leaving the contra gain added bare ((a-b)*c+d, a wrong grouping) and multiplying the whole stimulus by the "
            "gain sum before subtracting the bare threshold (a*(c+d)-b, a wrong pairing). The core confusion tested is "
            "that (a-b)*(c+d) is ((a-b)*(c+d)), not (a-b)*c+d and not a*(c+d)-b. Every stimulus exceeds its threshold by "
            "at least two, so all figures stay strictly positive."
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
