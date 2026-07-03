"""Generate rung-63 (lacrimal tear clearance) items.json for the ADJ-LADDER.

Rung 63 opens the **ophthalmology / lacrimal** panel on the quantitative band — the arithmetic of tear clearance. The
total tear production is the basal secretion PLUS the reflex secretion, and the net drainage capacity is the punctal
drainage capacity MINUS any blockage; the tear clearance index is the total production over the net drainage. Dividing
the SUM of two quantities by the DIFFERENCE of two others introduces a genuinely NEW arithmetic shape on the ladder: a
**sum over a difference** — `(a+b)/(c-d)`.

The setup: `basal_tears` of basal secretion plus `reflex_tears` of reflex secretion drain through a `drain_capacity`
reduced by a `blockage`. The tear clearance index is:

  CLEARANCE       (basal_tears + reflex_tears) / (drain_capacity - blockage)   [ production per unit net drainage ]
  TOTAL TEARS     basal_tears + reflex_tears                                   [ the numerator: total production ]
  NET DRAINAGE    drain_capacity - blockage                                    [ the denominator: net drainage ]

The **clearance** is what makes this rung distinctive — it is the ladder's first **sum over a difference**: a sum of two
quantities divided by a difference of two others. Contrast the neighbours already on the ladder: rung-59 was
`(a*b)/(c-d)` (a PRODUCT over a difference) and rung-32 was `(a-b)/(c-d)` (a DIFFERENCE over a difference); here a SUM
sits over the difference. (The total production `basal_tears+reflex_tears` and the net drainage `drain_capacity-blockage`
ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-62 shipped their
component sums/products/differences/ratios beside the headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the numerator sum, the parenthesised denominator difference, and their quotient — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../61/62. This rung exercises the engine across **a division of a sum by a difference** — the fact that
`(a+b)/(c-d)` is NOT `(a+b)/(c+d)` and NOT `(a-b)/(c-d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `/`, `+`, and `-`
— **no structural constants** — so no numeric literal appears in any program, and neither the total production, the net
drainage, nor any clearance figure is ever a literal (each is computed from the observed quantities). The observed
quantities carry **digit-free identifiers** (`basal_tears`, `reflex_tears`, `drain_capacity`, `blockage`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  POOLED     (basal_tears + reflex_tears) / (drain_capacity + blockage)   SUM the denominator instead of DIFFERENCING it
                                                                          (the classic `(a+b)/(c-d)` vs `(a+b)/(c+d)`
                                                                          error), and
  CROSSED    (basal_tears - reflex_tears) / (drain_capacity - blockage)   DIFFERENCE the numerator instead of SUMMING it
                                                                          (subtract the two productions),

which are exactly the mistakes a student makes (adding the two drainage terms, or subtracting the two productions). Gold
rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as options.

Distinctness: all four observed quantities are strictly positive, the tables are chosen so the drainage capacity exceeds
the blockage (the net drainage — and therefore the clearance — is positive) and the basal secretion exceeds the reflex
secretion (so the difference-numerator distractor stays positive); the five family values are pairwise distinct with a
comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (BASAL_TEARS, REFLEX_TEARS, DRAIN_CAPACITY, BLOCKAGE) — two tear productions and two drainage figures, all plain
# positive numbers with basal > reflex and drain_capacity > blockage. The five family values are asserted
# pairwise-distinct (with margin) below.
TABLES = [
    (60, 20, 50, 10),
    (80, 40, 60, 20),
    (90, 30, 40, 10),
    (70, 50, 80, 40),
    (100, 20, 30, 10),
    (50, 30, 70, 30),
    (120, 40, 90, 50),
]

# The option family (5 members), all built from the four observed quantities via /, +, and -. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "clearance",
        "tear clearance index (total production over the net drainage)",
        "(basal_tears + reflex_tears) / (drain_capacity - blockage)",
    ),
    (
        "total_tears",
        "the total tear production (basal plus reflex secretion)",
        "basal_tears + reflex_tears",
    ),
    (
        "net_drainage",
        "the net drainage capacity (drainage minus blockage)",
        "drain_capacity - blockage",
    ),
    (
        "pooled",
        "total production over the SUM of drainage and blockage, not their difference (a wrong net drainage)",
        "(basal_tears + reflex_tears) / (drain_capacity + blockage)",
    ),
    (
        "crossed",
        "the DIFFERENCE of the two productions over the net drainage, not their sum (a wrong total)",
        "(basal_tears - reflex_tears) / (drain_capacity - blockage)",
    ),
]
QUERIED = ["clearance", "total_tears", "net_drainage"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(basal_tears, reflex_tears, drain_capacity, blockage):
    # Operation order mirrors the ADJ programs exactly (each parenthesised sum/difference formed first, then the
    # division), so the Python option value and the engine result are the same IEEE-double (well within the harness's
    # 1e-9 match tolerance).
    return {
        "clearance": (basal_tears + reflex_tears) / (drain_capacity - blockage),
        "total_tears": basal_tears + reflex_tears,
        "net_drainage": drain_capacity - blockage,
        "pooled": (basal_tears + reflex_tears) / (drain_capacity + blockage),
        "crossed": (basal_tears - reflex_tears) / (drain_capacity - blockage),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for basal_tears, reflex_tears, drain_capacity, blockage in TABLES:
        assert (
            basal_tears > 0
            and reflex_tears > 0
            and drain_capacity > 0
            and blockage > 0
        ), (basal_tears, reflex_tears, drain_capacity, blockage)
        # Net drainage must be positive (divisor) and the basal secretion must exceed the reflex secretion
        # (so the difference-numerator distractor is positive).
        assert drain_capacity > blockage, (basal_tears, reflex_tears, drain_capacity, blockage)
        assert basal_tears > reflex_tears, (basal_tears, reflex_tears, drain_capacity, blockage)
        fv = family_values(basal_tears, reflex_tears, drain_capacity, blockage)
        for key, v in fv.items():
            assert v > 0, (key, basal_tears, reflex_tears, drain_capacity, blockage, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    basal_tears,
                    reflex_tears,
                    drain_capacity,
                    blockage,
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
                basal_tears,
                reflex_tears,
                drain_capacity,
                blockage,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r63tear-{idx + 1:02d}",
                "qtype": "lacrimal_tear_clearance",
                "stem": (
                    f"A tear study measures {num(basal_tears)} units of basal secretion and {num(reflex_tears)} of "
                    f"reflex secretion draining through a capacity of {num(drain_capacity)} reduced by a "
                    f"{num(blockage)} blockage. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe basal_tears({num(basal_tears)})\n"
                    f"observe reflex_tears({num(reflex_tears)})\n"
                    f"observe drain_capacity({num(drain_capacity)})\n"
                    f"observe blockage({num(blockage)})\n"
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
            "ADJ-LADDER rung 63 — tear clearance index from four stated quantities (a NEW panel: ophthalmology / "
            "lacrimal). From a basal and a reflex tear production (their sum is the total production) and a drainage "
            "capacity reduced by a blockage (their difference is the net drainage), compute the clearance "
            "((basal_tears+reflex_tears)/(drain_capacity-blockage)), the total production (basal_tears+reflex_tears), or "
            "the net drainage (drain_capacity-blockage). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW shape, SUM OVER A "
            "DIFFERENCE (a+b)/(c-d), the first on the ladder to divide a sum by a difference (distinct from rung-59 "
            "product-over-difference (a*b)/(c-d) and rung-32 difference-over-difference (a-b)/(c-d)) — and the harness "
            "matches the scalar to the printed options. Contamination-safe: every index is built only from the four "
            "observed quantities via /, +, and - — no constant leaks, and neither the total production, the net "
            "drainage, nor any clearance figure ever appears as a literal (each is computed) — and the observed "
            "quantities carry digit-free identifiers so no numeral hides inside a variable name. The five options are a "
            "family over the same four quantities, so the distractors are exactly the slips students make: SUMMING the "
            "denominator ((a+b)/(c+d), a wrong net drainage) and DIFFERENCING the numerator ((a-b)/(c-d), a wrong "
            "total). The core confusion tested is that (a+b)/(c-d) is not (a+b)/(c+d) and not (a-b)/(c-d)."
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
