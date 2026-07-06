"""Generate rung-88 (neurology nerve-conduction index) items.json for the ADJ-LADDER.

Rung 88 opens the **neurology / nerve-conduction** panel on the quantitative band — the arithmetic of a nerve-conduction
index. A `conduction_velocity` is multiplied by a `segment_span` AND by a `myelin_gain` — a THREE-FACTOR product — and a
`baseline_latency` is then ADDED. A three-factor product with a bare term added introduces a genuinely NEW arithmetic
shape on the ladder: a **three-factor product PLUS a term** — `a*b*c+d`, i.e. `((a*b*c) + d)`.

This is genuinely new: it COMPLETES the three-factor-product trio — rung-86 subtracted a term (`a*b*c-d`), rung-87
divided by a term (`a*b*c/d`), and rung-88 ADDS a term (`a*b*c+d`). No shipped shape ever added a bare term to a
three-factor product; every prior addition shape added to a product of at most TWO observed factors (rung-69 `a*b/c+d`,
rung-79 `a*b+c/d`, and rungs 34/37's sums of two two-factor products / two sums). The operator order matters: `a*b*c+d`
is `((a*b*c) + d)` by precedence (all three multiplies bind first, left-to-right, then the addition applies), NOT
`a*b*(c+d)` (adding `d` to `c` INSIDE the third factor) and NOT `a*b+c*d` (multiplying only the first two factors and
adding the product of the other two) — the two distractors exploit exactly those confusions.

The setup: a `conduction_velocity`, a `segment_span`, a `myelin_gain`, and a `baseline_latency`. The nerve conduction is:

  NERVE CONDUCTION  conduction_velocity * segment_span * myelin_gain + baseline_latency  [ three-factor product plus a term ]
  GROSS PRODUCT     conduction_velocity * segment_span * myelin_gain                      [ the three-factor product, before the baseline ]
  BASE PRODUCT      conduction_velocity * segment_span                                    [ the first two factors, before the myelin gain ]

The **nerve conduction** is what makes this rung distinctive — it is the ladder's first **three-factor product PLUS a
term**. (The gross product `a*b*c` and the base product `a*b` ride alongside as component readouts, so the panel teaches
the whole calculation — exactly as rungs 47-87 shipped their component sums/products/differences/ratios beside the
headline figure.)

Each index is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the multiplication of the conduction velocity by the segment span by the myelin gain (three
factors, left-to-right), then the addition of the baseline latency (the multiplies before the add) — and the harness
reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../86/87. This rung exercises the engine across **a three-factor product plus a term** — the fact that `a*b*c+d`
is `((a*b*c) + d)` and NOT `a*b*(c+d)` and NOT `a*b+c*d` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `*` and `+` — **no
structural constants** — so no numeric literal appears in any program, and neither the gross product, the base product,
nor any conduction figure is ever a literal (each is computed from the observed quantities). The observed quantities
carry **digit-free identifiers** (`conduction_velocity`, `segment_span`, `myelin_gain`, `baseline_latency`) so no numeral
hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    conduction_velocity * segment_span * (myelin_gain + baseline_latency)   add the baseline latency to the myelin
                                                                                     gain INSIDE the third factor, instead
                                                                                     of after the whole product (the classic
                                                                                     `a*b*c+d` vs `a*b*(c+d)` error), and
  SWAPPED    conduction_velocity * segment_span + myelin_gain * baseline_latency     multiply only the first TWO factors and
                                                                                     add the PRODUCT of the other two, a
                                                                                     wrong pairing (`a*b+c*d` instead of
                                                                                     `a*b*c+d`),

which are exactly the mistakes a student makes (folding the final addition into the last factor, or pairing the four
quantities into two products). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five
always appear as options.

Distinctness and positivity: the tables keep the guards — every quantity `>= 2`, `conduction_velocity*segment_span !=
baseline_latency` (so the nerve conduction never collapses onto the swapped) and `conduction_velocity*segment_span*
(myelin_gain-1) != myelin_gain*baseline_latency` (so the gross product never collapses onto the swapped) — so every
family member, including the headline nerve conduction `a*b*c+d`, is strictly positive (a sum of positive products); the
five family values are pairwise distinct with a comfortable margin, asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (CONDUCTION_VELOCITY, SEGMENT_SPAN, MYELIN_GAIN, BASELINE_LATENCY) — a conduction velocity to multiply by the segment
# span and the myelin gain (a three-factor product), and a baseline latency to add to that product, all plain positive
# numbers >= 2. The tables satisfy the guards: conduction_velocity*segment_span != baseline_latency (nerve conduction !=
# swapped) and conduction_velocity*segment_span*(myelin_gain-1) != myelin_gain*baseline_latency (gross product !=
# swapped). The five family values are asserted pairwise-distinct below.
TABLES = [
    (3, 4, 2, 3),
    (4, 3, 3, 2),
    (5, 4, 2, 5),
    (6, 2, 4, 3),
    (4, 5, 3, 2),
    (7, 3, 2, 3),
    (5, 6, 4, 3),
]

# The option family (5 members), all built from the four observed quantities via * and +. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "nerve_conduction",
        "net nerve conduction (the three-factor product plus the baseline latency)",
        "conduction_velocity * segment_span * myelin_gain + baseline_latency",
    ),
    (
        "gross_product",
        "the gross product (the conduction velocity times the segment span times the myelin gain, before the baseline)",
        "conduction_velocity * segment_span * myelin_gain",
    ),
    (
        "base_product",
        "the base product (the conduction velocity times the segment span, before the myelin gain)",
        "conduction_velocity * segment_span",
    ),
    (
        "crossed",
        "the conduction velocity times the segment span, all scaled by the myelin gain PLUS the baseline latency, with the baseline added to the myelin gain inside the third factor instead of onto the whole product (a wrong grouping)",
        "conduction_velocity * segment_span * (myelin_gain + baseline_latency)",
    ),
    (
        "swapped",
        "the conduction velocity times the segment span, PLUS the myelin gain times the baseline latency, pairing the four quantities into two products instead of one three-factor product (a wrong pairing)",
        "conduction_velocity * segment_span + myelin_gain * baseline_latency",
    ),
]
QUERIED = ["nerve_conduction", "gross_product", "base_product"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(conduction_velocity, segment_span, myelin_gain, baseline_latency):
    # Operation order mirrors the ADJ programs exactly (all three multiplies bind first, left-to-right, then the
    # addition applies, so a*b*c+d evaluates as ((a*b*c)+d)), so the Python option value and the engine result are the
    # same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "nerve_conduction": conduction_velocity * segment_span * myelin_gain + baseline_latency,
        "gross_product": conduction_velocity * segment_span * myelin_gain,
        "base_product": conduction_velocity * segment_span,
        "crossed": conduction_velocity * segment_span * (myelin_gain + baseline_latency),
        "swapped": conduction_velocity * segment_span + myelin_gain * baseline_latency,
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for conduction_velocity, segment_span, myelin_gain, baseline_latency in TABLES:
        assert (
            conduction_velocity > 0
            and segment_span > 0
            and myelin_gain > 0
            and baseline_latency > 0
        ), (conduction_velocity, segment_span, myelin_gain, baseline_latency)
        fv = family_values(conduction_velocity, segment_span, myelin_gain, baseline_latency)
        # The tables satisfy the guards, so every family member is strictly positive.
        for key, v in fv.items():
            assert v > 0, (key, conduction_velocity, segment_span, myelin_gain, baseline_latency, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    conduction_velocity,
                    segment_span,
                    myelin_gain,
                    baseline_latency,
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
                conduction_velocity,
                segment_span,
                myelin_gain,
                baseline_latency,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r88ncv-{idx + 1:02d}",
                "qtype": "nerve_conduction_index",
                "stem": (
                    f"A nerve study reads a conduction velocity of {num(conduction_velocity)} times a segment span of "
                    f"{num(segment_span)} times a myelin gain of {num(myelin_gain)}, with a baseline latency of "
                    f"{num(baseline_latency)} added on. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe conduction_velocity({num(conduction_velocity)})\n"
                    f"observe segment_span({num(segment_span)})\n"
                    f"observe myelin_gain({num(myelin_gain)})\n"
                    f"observe baseline_latency({num(baseline_latency)})\n"
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
            "ADJ-LADDER rung 88 — neurology nerve-conduction index from four stated quantities (a NEW panel: neurology / "
            "nerve-conduction). From a conduction velocity to multiply by the segment span and the myelin gain (a "
            "three-factor product) and a baseline latency to add, compute the nerve conduction "
            "(conduction_velocity*segment_span*myelin_gain+baseline_latency), the gross product "
            "(conduction_velocity*segment_span*myelin_gain), or the base product (conduction_velocity*segment_span). Each "
            "item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine "
            "carries the arithmetic — a NEW shape, THREE-FACTOR PRODUCT PLUS A TERM a*b*c+d (multiply a by b by c, add d, "
            "so a*b*c+d = ((a*b*c)+d); it completes the three-factor-product trio with rung-86 a*b*c-d and rung-87 "
            "a*b*c/d, and no prior addition shape added a bare term to a three-factor product — every earlier add, e.g. "
            "rung-69 a*b/c+d and rung-79 a*b+c/d, added to at most two observed factors) — and the harness matches the "
            "scalar to the printed options. Contamination-safe: every index is built only from the four observed "
            "quantities via * and + — no constant leaks, and neither the gross product, the base product, nor any "
            "conduction figure ever appears as a literal (each is computed) — and the observed quantities carry "
            "digit-free identifiers so no numeral hides inside a variable name. The five options are a family over the "
            "same four quantities, so the distractors are exactly the slips students make: adding the baseline latency to "
            "the myelin gain inside the third factor (a*b*(c+d), a wrong grouping) and pairing the four quantities into "
            "two products (a*b+c*d, a wrong pairing). The core confusion tested is that a*b*c+d is ((a*b*c)+d), not "
            "a*b*(c+d) and not a*b+c*d."
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
