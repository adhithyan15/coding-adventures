"""Generate rung-126 (capnography ventilation index / lone term over an ADD-A-QUOTIENT three-term denominator) items.json.

Rung 126 opens the **capnography / end-tidal ventilation** panel on the ADJ-LADDER's quantitative band, and — more importantly —
introduces the ladder's **first three-term denominator that contains a DIVISION**. A single observed `expired_volume` is DIVIDED by an
effective dead space formed from a `dead_space` with a per-cycle `shunt_reserve / breath_cycles` quotient ADDED to it, to give the
ventilation index. A **lone term over an add-a-quotient three-term denominator**, `a/(b + c/d)`, i.e. `(a / (b + (c/d)))` — the inner
quotient `c/d` binds BEFORE the `+` (operator precedence), and the whole `b + c/d` sits under the division bar (grouping).

This is genuinely new. Every three-term denominator shipped so far is built from `+`, `-`, and `*` only — b+c+d (116), b+c-d (117),
b*c+d (118), b*c-d (119), b*c*d (121), b+c*d (122), b-c*d (123), b-c-d (124), and rung-125's b-c+d — **no denominator has ever
contained a `/`**. rung-126 promotes the first quotient-bearing denominator to a headline. It is the division sibling of rung-122's
`a/(b + c*d)`: rung-122 MULTIPLIES the later two terms (b + c*d), rung-126 DIVIDES them (b + c/d). Their swapped distractors differ only
in that one operator. It also sharpens two distinct confusions at once — the precedence question "does `c/d` bind before the `+`?" and
the grouping question "is the whole `b + c/d` under the bar?".

The setup: an `expired_volume`, a `dead_space`, a `shunt_reserve`, and a `breath_cycles` count. The figures are:

  VENTILATION INDEX   expired_volume / (dead_space + shunt_reserve / breath_cycles)   [ lone term / add-a-quotient denom ]
  EFFECTIVE SPACE     dead_space + shunt_reserve / breath_cycles                      [ the add-a-quotient denominator ]
  PER CYCLE           shunt_reserve / breath_cycles                                   [ the shunt reserve spread over the breath cycles ]

The **ventilation index** is what makes this rung distinctive — it is the ladder's first **lone quantity over a denominator that
contains a division (as a headline)**. It is a rate (expired volume per unit of effective dead space), framed as an *index* to keep it
dimensionless-clean — the same discipline rungs 100/.../124/125 used for their ratios. (The effective space `b + c/d` and the per-cycle
quotient `c/d` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-125 shipped their
component sums/products/quotients beside the headline figure. The per-cycle quotient `c/d` anchors the "the shunt reserve is spread over
the breath cycles FIRST, then added to the dead space" grouping against both distractors.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries the
arithmetic — the division of the shunt reserve by the breath cycles to form the per-cycle quotient, then the addition of that quotient
to the dead space to form the whole effective space, then the division of the expired volume by that whole effective space (so
a/(b+c/d) evaluates as (a/(b+(c/d)))) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No
harness/engine change, exactly as rungs 8/16/.../124/125. This rung exercises the engine across a **lone-term-over-(add-a-quotient
three-term) ratio** — the fact that `a/(b+c/d)` is `(a/(b+(c/d)))` and NOT `(a/b)+c/d` and NOT `a/((b+c)/d)` made computable. The golds
are non-integer f64s; the engine's IEEE-double division matches Python's the same way rungs 100/.../124/125 relied on (well within the
harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+` and `/` — **no structural
constants** — so no numeric literal appears in any program, and neither the effective space, the per-cycle quotient, nor the
ventilation index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`expired_volume`, `dead_space`, `shunt_reserve`, `breath_cycles`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED     expired_volume / dead_space + shunt_reserve / breath_cycles   drop the denominator parentheses so only the dead space
                                                                  divides the expired volume, then the per-cycle quotient is added
                                                                  (the classic `a/(b+c/d)` vs `a/b+c/d` grouping error, evaluating
                                                                  `(a/b)+(c/d)`), and
  MISGROUPED  expired_volume / ((dead_space + shunt_reserve) / breath_cycles)   add the dead space and shunt reserve FIRST, then
                                                                  divide by the breath cycles (`a/((b+c)/d)` = `a*d/(b+c)`, ignoring
                                                                  the precedence that `c/d` binds before the `+` — the "was the reserve
                                                                  divided before or after adding the dead space?" slip),

which are exactly the mistakes a student makes (failing to keep the whole add-a-quotient denominator under the bar, or breaking the
`c/d`-binds-first precedence). Gold rotates A-E by index. QUERIED (used as gold) = the three real readouts; all five always appear as
options.

Distinctness and positivity: every observed quantity is `>= 2` and the denominator uses only `+` and `/` of positive quantities, so
positivity is automatic — the effective space `b + c/d`, the per-cycle quotient `c/d`, the misgrouped denominator `(b+c)/d`, and the
crossed `a/b` are all comfortably positive, so every family member is `> 0` (asserted at build time). And — so all three queried
readouts vary across the panel — the seven tables give distinct ventilation indices, distinct effective spaces, and distinct per-cycle
quotients, all asserted at build time; the five family values are pairwise distinct with a comfortable margin.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (EXPIRED_VOLUME, DEAD_SPACE, SHUNT_RESERVE, BREATH_CYCLES) — a lone expired volume divided by an effective dead space (a dead space
# with a per-cycle shunt_reserve/breath_cycles quotient added to it) for the ventilation index. The denominator uses only + and / of
# positive quantities, so positivity is automatic. The seven tables give distinct per-cycle quotients (c/d), distinct effective spaces
# (b + c/d), and distinct ventilation indices (a/(b+c/d)) so all three queried readouts vary across the panel; the five family values
# are asserted pairwise-distinct below.
TABLES = [
    (30, 4, 6, 3),   # c/d = 2.0,  eff = 6.0,  index = 5.0
    (44, 5, 5, 2),   # c/d = 2.5,  eff = 7.5,  index = 5.8666…
    (60, 6, 9, 3),   # c/d = 3.0,  eff = 9.0,  index = 6.6666…
    (26, 7, 3, 2),   # c/d = 1.5,  eff = 8.5,  index = 3.0588…
    (78, 8, 8, 2),   # c/d = 4.0,  eff = 12.0, index = 6.5
    (95, 9, 7, 2),   # c/d = 3.5,  eff = 12.5, index = 7.6
    (120, 10, 10, 2),# c/d = 5.0,  eff = 15.0, index = 8.0
]

# The option family (5 members), all built from the four observed quantities via + and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the options.
FAMILY = [
    (
        "ventilation_index",
        "ventilation index (the expired volume divided by the effective space)",
        "expired_volume / (dead_space + shunt_reserve / breath_cycles)",
    ),
    (
        "effective_space",
        "the effective space (the dead space plus the per-cycle shunt reserve, the divisor the expired volume is divided by)",
        "dead_space + shunt_reserve / breath_cycles",
    ),
    (
        "per_cycle",
        "the per-cycle reserve (the shunt reserve spread over the breath cycles, before it is added to the dead space)",
        "shunt_reserve / breath_cycles",
    ),
    (
        "crossed",
        "the expired volume divided by the dead space, plus the shunt reserve over the breath cycles, dropping the denominator parentheses so only the dead space divides (a wrong grouping)",
        "expired_volume / dead_space + shunt_reserve / breath_cycles",
    ),
    (
        "misgrouped",
        "the expired volume divided by the dead space plus the shunt reserve, all over the breath cycles, adding before dividing the reserve (a wrong grouping)",
        "expired_volume / ((dead_space + shunt_reserve) / breath_cycles)",
    ),
]
QUERIED = ["ventilation_index", "effective_space", "per_cycle"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(expired_volume, dead_space, shunt_reserve, breath_cycles):
    # Operation order mirrors the ADJ programs exactly (the shunt reserve is divided by the breath cycles to form the per-cycle
    # quotient, then that quotient is added to the dead space to form the whole effective space, then the expired volume is divided by
    # that whole effective space, so a/(b+c/d) evaluates as (a/(b+(c/d)))), so the Python option value and the engine result are the
    # same IEEE-double (well within the 1e-9 tolerance).
    return {
        "ventilation_index": expired_volume / (dead_space + shunt_reserve / breath_cycles),
        "effective_space": dead_space + shunt_reserve / breath_cycles,
        "per_cycle": shunt_reserve / breath_cycles,
        "crossed": expired_volume / dead_space + shunt_reserve / breath_cycles,
        "misgrouped": expired_volume / ((dead_space + shunt_reserve) / breath_cycles),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for expired_volume, dead_space, shunt_reserve, breath_cycles in TABLES:
        # Every observed quantity is a plain positive number >= 2. The denominator uses only + and / of positive quantities, so
        # positivity is automatic; it is still asserted per family member below.
        assert (
            expired_volume >= 2
            and dead_space >= 2
            and shunt_reserve >= 2
            and breath_cycles >= 2
        ), (expired_volume, dead_space, shunt_reserve, breath_cycles)
        fv = family_values(expired_volume, dead_space, shunt_reserve, breath_cycles)
        for key, v in fv.items():
            assert v > 0, (key, expired_volume, dead_space, shunt_reserve, breath_cycles, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    expired_volume,
                    dead_space,
                    shunt_reserve,
                    breath_cycles,
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
                expired_volume,
                dead_space,
                shunt_reserve,
                breath_cycles,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r126capno-{idx + 1:02d}",
                "qtype": "ventilation_index",
                "stem": (
                    f"A capnography trace records an expired volume of {num(expired_volume)} divided by a dead space of "
                    f"{num(dead_space)} plus a shunt reserve of {num(shunt_reserve)} over a breath cycle count of "
                    f"{num(breath_cycles)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe expired_volume({num(expired_volume)})\n"
                    f"observe dead_space({num(dead_space)})\n"
                    f"observe shunt_reserve({num(shunt_reserve)})\n"
                    f"observe breath_cycles({num(breath_cycles)})\n"
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
            "ADJ-LADDER rung 126 — capnography ventilation index from four stated quantities (a NEW panel: capnography / end-tidal "
            "ventilation, and the ladder's FIRST three-term denominator containing a division). From a lone expired volume divided by "
            "an effective space (a dead space with a per-cycle shunt_reserve/breath_cycles quotient added to it), compute the "
            "ventilation index (expired_volume/(dead_space+shunt_reserve/breath_cycles)), the effective space "
            "(dead_space+shunt_reserve/breath_cycles), or the per-cycle reserve (shunt_reserve/breath_cycles). Each item is a "
            "compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — "
            "a NEW family, a LONE TERM OVER AN ADD-A-QUOTIENT THREE-TERM DENOMINATOR a/(b+c/d) (divide the shunt reserve by the breath "
            "cycles, add that to the dead space, then divide the expired volume by that whole effective space, so a/(b+c/d) = "
            "(a/(b+(c/d))); the division sibling of rung-122's a/(b+c*d). No denominator on the ladder has ever contained a division, "
            "so this promotes the first quotient-bearing denominator to a queried gold. The precedence-and-grouping slips ride "
            "alongside as distractors. The harness matches the scalar to the printed options. The ventilation index is a rate "
            "(expired volume per unit of effective space), framed as an INDEX so the dimensionless value stays honest. "
            "Contamination-safe: every figure is built only from the four observed quantities via + and / — no constant leaks, and "
            "neither the effective space, the per-cycle quotient, nor the ventilation index ever appears as a literal (each is "
            "computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a variable name. The five "
            "options are a family over the same four quantities, so the distractors are exactly the slips students make: dropping the "
            "denominator parentheses so only the dead space divides (a/b+c/d, evaluating (a/b)+(c/d), a wrong grouping) and adding "
            "before dividing the reserve (a/((b+c)/d) = a*d/(b+c), breaking the c/d-binds-first precedence, a wrong grouping). The core "
            "confusion tested is that a/(b+c/d) is (a/(b+(c/d))), not a/b+c/d and not a/((b+c)/d). The denominator uses only + and / of "
            "positive quantities so positivity is automatic; the five family values are pairwise distinct and all three queried "
            "readouts vary across the panel, all asserted strictly positive at build time."
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
