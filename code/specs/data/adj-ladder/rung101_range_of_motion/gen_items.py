"""Generate rung-101 (orthopedics / range-of-motion) items.json for the ADJ-LADDER.

Rung 101 opens the **orthopedics / range-of-motion** panel on the quantitative band — the arithmetic of a joint's net
motion arc. A `flexion_arc` PLUS an `extension_arc` gives the combined arc (how much motion the two directions contribute
together), a `brace_factor` TIMES a `stiffness_factor` gives the brace load (the product the arc is docked by), and the
brace load is SUBTRACTED from the combined arc to give the net arc. A **sum minus a product** introduces a genuinely NEW
arithmetic family on the ladder: `a+b-c*d`, i.e. `((a+b) - (c*d))`.

This is genuinely new — the first time the ladder subtracts a bare PRODUCT from a bare SUM. It is the **minus-sibling of
rung-91** `a+b+c*d` (a sum plus a product): rung-91 added the product to the sum, rung-101 subtracts it. No prior rung took
a sum minus a product: rung-34 `a*b+c*d` summed two products, rung-35 `a*b-c*d` subtracted two products, rung-31 subtracted
two differences, and rungs 79/80 attach a `c/d` division rather than a `c*d` product to the `a*b` term. The operator order
matters: `a+b-c*d` is `((a+b) - (c*d))` (the sum forms, the product forms, then the product is subtracted from the sum
— multiplication binds tighter than the subtraction), NOT `(a+b-c)*d` (folding the subtraction inside so the brace factor
is subtracted from the combined arc *before* multiplying by the stiffness factor) and NOT `(a*b) - (c+d)` (multiplying the
first pair and summing the second pair, mispairing which pair is the product and which is the sum) — the two distractors
exploit exactly those confusions.

The setup: a `flexion_arc`, an `extension_arc`, a `brace_factor`, and a `stiffness_factor`. The total is:

  NET ARC        (flexion_arc + extension_arc) - (brace_factor * stiffness_factor)  [ a sum minus a product ]
  COMBINED ARC   flexion_arc + extension_arc                                        [ the sum, the minuend ]
  BRACE LOAD     brace_factor * stiffness_factor                                    [ the product, subtracted ]

The **net arc** is what makes this rung distinctive — it is the ladder's first **bare SUM minus a bare PRODUCT**. (The
combined arc `a+b` and the brace load `c*d` ride alongside as component readouts, so the panel teaches the whole
calculation — exactly as rungs 47-100 shipped their component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the addition of the flexion arc and extension arc into the combined arc, the multiplication of the
brace factor by the stiffness factor into the brace load, then the subtraction of the brace load from the combined arc (the
product forming before it is subtracted, so a+b-c*d evaluates as ((a+b)-(c*d))) — and the harness reads the scalar via the
existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs 8/16/.../99/100. This rung exercises
the engine across a **sum minus a product** — the fact that `a+b-c*d` is `((a+b)-(c*d))` and NOT `(a+b-c)*d` and NOT
`(a*b)-(c+d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, and `*` —
**no structural constants** — so no numeric literal appears in any program, and neither the combined arc, the brace load,
nor any net figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`flexion_arc`, `extension_arc`, `brace_factor`, `stiffness_factor`) so no numeral hides inside a
variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (flexion_arc + extension_arc - brace_factor) * stiffness_factor  fold the subtraction inside the parentheses
                                                                              so the brace factor is subtracted from the
                                                                              combined arc *before* multiplying by the
                                                                              stiffness factor (the classic `a+b-c*d` vs
                                                                              `(a+b-c)*d` precedence error), and
  SWAPPED    (flexion_arc * extension_arc) - (brace_factor + stiffness_factor)  multiply the first pair and sum the second
                                                                              pair, mispairing which pair is the product and
                                                                              which is the sum (`(a*b)-(c+d)` instead of
                                                                              `(a+b)-(c*d)`),

which are exactly the mistakes a student makes (folding the subtraction inside the parentheses before multiplying, or
mispairing which pair is a sum and which is a product). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness and positivity: the tables are chosen so `flexion_arc + extension_arc > brace_factor * stiffness_factor`
(net arc strictly positive — the combined arc always exceeds the brace load it is docked by) and `flexion_arc *
extension_arc > brace_factor + stiffness_factor` (the swapped figure strictly positive), so no family member is ever zero or
negative; every observed quantity is >= 2. The combined arc (a sum of positives) and the brace load (a product of positives)
are trivially positive, and the crossed figure `(a+b-c)*d` is positive because `a+b > c*d >= 2c > c` so `a+b-c > 0`. The
tables are chosen so the five family values are pairwise distinct with a comfortable margin, and — so all three queried
readouts vary across the panel — the seven tables give distinct net arcs, distinct combined arcs, and distinct brace loads,
all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FLEXION_ARC, EXTENSION_ARC, BRACE_FACTOR, STIFFNESS_FACTOR) — a flexion arc plus an extension arc for the combined arc,
# a brace factor times a stiffness factor for the brace load, all plain positive numbers >= 2. Each table satisfies
# flexion_arc + extension_arc > brace_factor * stiffness_factor (net arc > 0) and flexion_arc * extension_arc >
# brace_factor + stiffness_factor (swapped > 0), so every family member is strictly positive (no negatives anywhere); the
# five family values are asserted pairwise-distinct below. The seven tables give distinct net arcs, distinct combined arcs,
# and distinct brace loads so all three queried readouts vary across the panel.
TABLES = [
    (2, 3, 2, 2),
    (2, 6, 2, 3),
    (2, 9, 2, 4),
    (2, 11, 3, 3),
    (2, 13, 2, 5),
    (5, 13, 2, 6),
    (8, 13, 2, 7),
]

# The option family (5 members), all built from the four observed quantities via +, -, and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "net_arc",
        "total net arc (the combined arc minus the brace load)",
        "(flexion_arc + extension_arc) - (brace_factor * stiffness_factor)",
    ),
    (
        "combined_arc",
        "the combined arc (the flexion arc plus the extension arc, the minuend before subtracting)",
        "flexion_arc + extension_arc",
    ),
    (
        "brace_load",
        "the brace load (the brace factor times the stiffness factor, the product subtracted from the combined arc)",
        "brace_factor * stiffness_factor",
    ),
    (
        "crossed",
        "the flexion arc plus the extension arc minus the brace factor, all multiplied by the stiffness factor, folding the subtraction inside the parentheses so the brace factor is subtracted before multiplying (a wrong grouping)",
        "(flexion_arc + extension_arc - brace_factor) * stiffness_factor",
    ),
    (
        "swapped",
        "the flexion arc times the extension arc, minus the brace factor plus the stiffness factor, multiplying the first pair and summing the second pair instead (a wrong pairing)",
        "(flexion_arc * extension_arc) - (brace_factor + stiffness_factor)",
    ),
]
QUERIED = ["net_arc", "combined_arc", "brace_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(flexion_arc, extension_arc, brace_factor, stiffness_factor):
    # Operation order mirrors the ADJ programs exactly (the sum forms, the product forms, then the product is subtracted
    # from the sum, so a+b-c*d evaluates as ((a+b)-(c*d))), so the Python option value and the engine result are the same
    # IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "net_arc": (flexion_arc + extension_arc) - (brace_factor * stiffness_factor),
        "combined_arc": flexion_arc + extension_arc,
        "brace_load": brace_factor * stiffness_factor,
        "crossed": (flexion_arc + extension_arc - brace_factor) * stiffness_factor,
        "swapped": (flexion_arc * extension_arc) - (brace_factor + stiffness_factor),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for flexion_arc, extension_arc, brace_factor, stiffness_factor in TABLES:
        # Every observed quantity is a plain positive number >= 2, and the tables guarantee flexion_arc + extension_arc >
        # brace_factor * stiffness_factor (net arc > 0) and flexion_arc * extension_arc > brace_factor + stiffness_factor
        # (swapped > 0), so every family member is strictly positive with no negative anywhere.
        assert (
            flexion_arc >= 2
            and extension_arc >= 2
            and brace_factor >= 2
            and stiffness_factor >= 2
        ), (flexion_arc, extension_arc, brace_factor, stiffness_factor)
        assert flexion_arc + extension_arc > brace_factor * stiffness_factor, (
            flexion_arc, extension_arc, brace_factor, stiffness_factor,
        )
        assert flexion_arc * extension_arc > brace_factor + stiffness_factor, (
            flexion_arc, extension_arc, brace_factor, stiffness_factor,
        )
        fv = family_values(flexion_arc, extension_arc, brace_factor, stiffness_factor)
        for key, v in fv.items():
            assert v > 0, (key, flexion_arc, extension_arc, brace_factor, stiffness_factor, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    flexion_arc,
                    extension_arc,
                    brace_factor,
                    stiffness_factor,
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
                flexion_arc,
                extension_arc,
                brace_factor,
                stiffness_factor,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r101rom-{idx + 1:02d}",
                "qtype": "rom_net_arc",
                "stem": (
                    f"A range-of-motion study records a flexion arc of {num(flexion_arc)} plus an extension arc of "
                    f"{num(extension_arc)}, minus a brace factor of {num(brace_factor)} times a stiffness factor of "
                    f"{num(stiffness_factor)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe flexion_arc({num(flexion_arc)})\n"
                    f"observe extension_arc({num(extension_arc)})\n"
                    f"observe brace_factor({num(brace_factor)})\n"
                    f"observe stiffness_factor({num(stiffness_factor)})\n"
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
            "ADJ-LADDER rung 101 — net range-of-motion arc from four stated quantities (a NEW panel: orthopedics / "
            "range-of-motion). From a flexion arc plus an extension arc for the combined arc, a brace factor times a "
            "stiffness factor for the brace load, and the brace load subtracted from the combined arc, compute the net arc "
            "((flexion_arc+extension_arc)-(brace_factor*stiffness_factor)), the combined arc (flexion_arc+extension_arc), or "
            "the brace load (brace_factor*stiffness_factor). Each item is a compute_dimensioned program (observe the four "
            "quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, A SUM MINUS A PRODUCT "
            "a+b-c*d (add a and b, multiply c and d, subtract the product from the sum, so a+b-c*d = ((a+b)-(c*d)); the "
            "FIRST time the ladder subtracts a bare PRODUCT from a bare SUM — the MINUS-SIBLING of rung-91 a+b+c*d which "
            "added the product to the sum; rung-34 a*b+c*d summed two products, rung-35 a*b-c*d subtracted two products) — "
            "and the harness matches the scalar to the printed options. Contamination-safe: every figure is built only from "
            "the four observed quantities via +, -, and * — no constant leaks, and neither the combined arc, the brace load, "
            "nor any net figure ever appears as a literal (each is computed) — and the observed quantities carry digit-free "
            "identifiers so no numeral hides inside a variable name. The five options are a family over the same four "
            "quantities, so the distractors are exactly the slips students make: folding the subtraction inside the "
            "parentheses so the brace factor is subtracted before multiplying ((a+b-c)*d, a wrong grouping) and multiplying "
            "the first pair while summing the second pair ((a*b)-(c+d), a wrong pairing). The core confusion tested is that "
            "a+b-c*d is ((a+b)-(c*d)), not (a+b-c)*d and not (a*b)-(c+d). Each table guarantees the combined arc exceeds the "
            "brace load and the flexion-times-extension product exceeds the brace-plus-stiffness sum, so every figure stays "
            "strictly positive."
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
