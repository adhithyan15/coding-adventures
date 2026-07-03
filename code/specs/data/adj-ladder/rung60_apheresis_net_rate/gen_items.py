"""Generate rung-60 (apheresis net collection rate) items.json for the ADJ-LADDER.

Rung 60 opens the **transfusion medicine / apheresis** panel on the quantitative band — the arithmetic of a platelet
cell separator. During apheresis, a machine COLLECTS platelets from the donor's blood while simultaneously RETURNING
some cells to the donor. The collection RATE is the collected count divided by the collection minutes — a RATIO — and
the return RATE is the returned count divided by the return minutes — another RATIO. The NET accumulation rate is the
collection rate MINUS the return rate. Subtracting one ratio FROM another introduces a genuinely NEW arithmetic shape on
the ladder: a **difference of two ratios** — `a/b - c/d` — two independent quotients (each with its OWN denominator)
subtracted.

The setup: the separator collects `collected_count` platelets over `collect_minutes`, and returns `returned_count`
platelets over `return_minutes`. The net accumulation rate is the collection rate minus the return rate:

  NET RATE       collected_count / collect_minutes - returned_count / return_minutes   [ net platelets per minute ]
  COLLECT RATE   collected_count / collect_minutes                                     [ one ratio: collection ]
  RETURN RATE    returned_count / return_minutes                                       [ the other ratio: return ]

The **net rate** is what makes this rung distinctive — it is the ladder's first **difference of two ratios**: two
separate quotients (each with its OWN denominator) subtracted. Contrast the neighbour already on the ladder: rung-57 was
`a/b + c/d` (a SUM of two ratios); this SUBTRACTS the second ratio from the first. (The collection rate
`collected_count/collect_minutes` and the return rate `returned_count/return_minutes` ride alongside as component
readouts, so the panel teaches the whole calculation — exactly as rungs 47-59 shipped their component sums/products/
differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — including both quotients and their difference — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../58/59. This rung exercises
the engine across **two divisions folded into a subtraction** — the fact that `a/b - c/d` is NOT `(a-c)/(b+d)` made
computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/` and `-` —
**no structural constants** — so no numeric literal appears in any program, and neither the collection rate, the return
rate, nor any net-rate figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`collected_count`, `collect_minutes`, `returned_count`, `return_minutes`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  POOLED     (collected_count - returned_count) / (collect_minutes + return_minutes)   POOL the count difference over the
                                                                                       pooled minutes — a DIFFERENCE OF
                                                                                       TOTALS, not a difference of rates
                                                                                       (the classic `a/b - c/d` vs
                                                                                       `(a-c)/(b+d)` error), and
  CROSSED    collected_count / return_minutes - returned_count / collect_minutes       SWAP the denominators — divide each
                                                                                       count by the OTHER phase's minutes,

which are exactly the mistakes a student makes (pooling numerators over pooled denominators, or pairing each numerator
with the wrong denominator). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always
appear as options.

Distinctness: all four observed quantities are strictly positive and the tables are chosen so the collection rate
exceeds the return rate (the net rate is positive, a sensible net accumulation); the five family values are pairwise
distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (COLLECTED_COUNT, COLLECT_MINUTES, RETURNED_COUNT, RETURN_MINUTES) — platelet counts and timed minutes for the
# collection and the return phases, all plain positive numbers (minutes chosen to divide the counts into clean rates,
# with the collection rate exceeding the return rate). The five family values are asserted pairwise-distinct (with
# margin) below, and the net rate is asserted positive.
TABLES = [
    (300, 60, 80, 40),
    (400, 80, 90, 30),
    (360, 60, 100, 50),
    (280, 70, 60, 40),
    (420, 60, 150, 50),
    (240, 80, 60, 60),
    (600, 120, 90, 45),
]

# The option family (5 members), all built from the four observed quantities via / and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five
# always appear as the options.
FAMILY = [
    (
        "net_rate",
        "net platelet accumulation rate (the collection rate minus the return rate)",
        "collected_count / collect_minutes - returned_count / return_minutes",
    ),
    (
        "collect_rate",
        "the collection rate (collected platelets per collection minute)",
        "collected_count / collect_minutes",
    ),
    (
        "return_rate",
        "the return rate (returned platelets per return minute)",
        "returned_count / return_minutes",
    ),
    (
        "pooled",
        "the pooled count difference over the pooled minutes, not the difference of the two rates (a wrong net)",
        "(collected_count - returned_count) / (collect_minutes + return_minutes)",
    ),
    (
        "crossed",
        "each count divided by the OTHER phase's minutes (swapped denominators)",
        "collected_count / return_minutes - returned_count / collect_minutes",
    ),
]
QUERIED = ["net_rate", "collect_rate", "return_rate"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(collected_count, collect_minutes, returned_count, return_minutes):
    # Operation order mirrors the ADJ programs exactly (each quotient formed first, then folded with -; and, for the
    # pooled slip, the parenthesised count difference divides the parenthesised minute sum), so the Python option value
    # and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    collect = collected_count / collect_minutes
    ret = returned_count / return_minutes
    return {
        "net_rate": collect - ret,
        "collect_rate": collect,
        "return_rate": ret,
        "pooled": (collected_count - returned_count) / (collect_minutes + return_minutes),
        "crossed": collected_count / return_minutes - returned_count / collect_minutes,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for collected_count, collect_minutes, returned_count, return_minutes in TABLES:
        assert (
            collected_count > 0
            and collect_minutes > 0
            and returned_count > 0
            and return_minutes > 0
        ), (collected_count, collect_minutes, returned_count, return_minutes)
        fv = family_values(collected_count, collect_minutes, returned_count, return_minutes)
        # The net rate must be positive (collection rate exceeds return rate) so it reads as a sensible net accumulation.
        assert fv["net_rate"] > 0, (collected_count, collect_minutes, returned_count, return_minutes, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    collected_count,
                    collect_minutes,
                    returned_count,
                    return_minutes,
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
                collected_count,
                collect_minutes,
                returned_count,
                return_minutes,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r60rate-{idx + 1:02d}",
                "qtype": "apheresis_net_rate",
                "stem": (
                    f"A platelet apheresis run collects {num(collected_count)} platelets over {num(collect_minutes)} min "
                    f"and returns {num(returned_count)} platelets over {num(return_minutes)} min. What is the "
                    f"{name_of[key]}?"
                ),
                "program": (
                    f"observe collected_count({num(collected_count)})\n"
                    f"observe collect_minutes({num(collect_minutes)})\n"
                    f"observe returned_count({num(returned_count)})\n"
                    f"observe return_minutes({num(return_minutes)})\n"
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
            "ADJ-LADDER rung 60 — net platelet accumulation rate from four stated quantities (a NEW panel: transfusion "
            "medicine / apheresis). From a collected count and its collection minutes (their ratio is the collection "
            "rate) and a returned count and its return minutes (their ratio is the return rate), compute the net rate "
            "(collected_count/collect_minutes - returned_count/return_minutes), the collection rate "
            "(collected_count/collect_minutes), or the return rate (returned_count/return_minutes). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the "
            "arithmetic — a NEW shape, DIFFERENCE OF TWO RATIOS a/b - c/d, the first on the ladder to subtract one "
            "independent quotient from another (distinct from rung-57 sum-of-two-ratios a/b + c/d) — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only from the four "
            "observed quantities via / and - — no constant leaks, and neither the collection rate, the return rate, nor "
            "any net-rate figure ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the "
            "same four quantities, so the distractors are exactly the slips students make: POOLING the count difference "
            "over the pooled minutes ((a-c)/(b+d), a difference of totals, not a difference of rates), and SWAPPING the "
            "denominators (a/d - c/b). The core confusion tested is that a/b - c/d is not (a-c)/(b+d)."
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
