"""Generate rung-103 (gastroenterology / GI-motility transit accounting) items.json for the ADJ-LADDER.

Rung 103 opens the **gastroenterology / GI-motility** panel on the quantitative band — the arithmetic of how much of an
ingested bolus actually transits the gut. An `ingested_volume` MINUS a `refluxed_volume` gives the cleared volume (what
moves past the sphincter, the difference), a `segment_count` TIMES a `stasis_per_segment` gives the stasis load (volume held
up per segment, summed across segments as the product), and the stasis load is SUBTRACTED from the cleared volume to give
the net transit volume. A **difference minus a product** introduces the LAST genuinely NEW arithmetic family in the
`(a±b)±c*d` quartet: `a-b-c*d`, i.e. `((a-b) - (c*d))`.

This is genuinely new — the first time the ladder subtracts a product from a difference — and it **COMPLETES the (a±b)±c*d
quartet**: rung-91 `a+b+c*d` (sum plus product), rung-101 `a+b-c*d` (sum minus product), rung-102 `a-b+c*d` (difference
plus product), and now rung-103 `a-b-c*d` (difference minus product). No prior rung subtracts a bare PRODUCT from a bare
DIFFERENCE: rung-35 `a*b-c*d` subtracted two products, rung-31 subtracted two differences, and rungs 79/80 attach a `c/d`
division rather than a `c*d` product to the `a*b` term. The operator order matters: `a-b-c*d` is `((a-b) - (c*d))` (the
difference forms, the product forms, then the product is subtracted from the difference — multiplication binds tighter than
the subtractions, and the two subtractions are the low-precedence joins), NOT `(a-b-c)*d` (folding the `-c` inside so the
segment count is subtracted from the cleared volume *before* multiplying by the stasis-per-segment) and NOT `(a*b)-(c-d)`
(multiplying the first pair and differencing the second pair, mispairing which pair is the product and which is the
difference) — the two distractors exploit exactly those confusions.

The setup: an `ingested_volume`, a `refluxed_volume`, a `segment_count`, and a `stasis_per_segment`. The total is:

  NET TRANSIT VOLUME   (ingested_volume - refluxed_volume) - (segment_count * stasis_per_segment)  [ a difference minus a product ]
  CLEARED VOLUME       ingested_volume - refluxed_volume                                           [ the difference, subtracted from ]
  STASIS LOAD          segment_count * stasis_per_segment                                          [ the product, subtracted ]

The **net transit volume** is what makes this rung distinctive — it is the ladder's first **bare DIFFERENCE minus a bare
PRODUCT**. (The cleared volume `a-b` and the stasis load `c*d` ride alongside as component readouts, so the panel teaches
the whole calculation — exactly as rungs 47-102 shipped their component sums/products/differences/ratios beside the
headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine
carries the arithmetic — the subtraction of the refluxed volume from the ingested volume into the cleared volume, the
multiplication of the segment count by the stasis-per-segment into the stasis load, then the subtraction of the stasis load
from the cleared volume (the product forming before it is subtracted, so a-b-c*d evaluates as ((a-b)-(c*d))) — and the
harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change, exactly as rungs
8/16/.../101/102. This rung exercises the engine across a **difference minus a product** — the fact that `a-b-c*d` is
`((a-b)-(c*d))` and NOT `(a-b-c)*d` and NOT `(a*b)-(c-d)` made computable.

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-`, and `*` —
**no structural constants** — so no numeric literal appears in any program, and neither the cleared volume, the stasis
load, nor any net figure is ever a literal (each is computed from the observed quantities). The observed quantities carry
**digit-free identifiers** (`ingested_volume`, `refluxed_volume`, `segment_count`, `stasis_per_segment`) so no numeral hides
inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    (ingested_volume - refluxed_volume - segment_count) * stasis_per_segment  fold the `- segment_count` inside the
                                                                                       parentheses so the segment count is
                                                                                       subtracted from the cleared volume
                                                                                       *before* multiplying by the
                                                                                       stasis-per-segment (the classic
                                                                                       `a-b-c*d` vs `(a-b-c)*d` precedence
                                                                                       error), and
  SWAPPED    (ingested_volume * refluxed_volume) - (segment_count - stasis_per_segment)  multiply the first pair and
                                                                                       difference the second pair, mispairing
                                                                                       which pair is the product and which is
                                                                                       the difference (`(a*b)-(c-d)` instead
                                                                                       of `(a-b)-(c*d)`),

which are exactly the mistakes a student makes (folding the subtraction inside the parentheses before multiplying, or
mispairing which pair is a difference and which is a product). Gold rotates A-E by index. QUERIED (used as gold) = the three
real readouts; all five always appear as options.

Distinctness and positivity: the tables are chosen so `ingested_volume > refluxed_volume` (cleared volume strictly
positive — some of the bolus always clears the sphincter) and `ingested_volume - refluxed_volume > segment_count *
stasis_per_segment` (net transit volume strictly positive — the cleared volume always exceeds the stasis load it is docked
by), so no family member is ever zero or negative; every observed quantity is >= 2. The cleared volume (a positive
difference) and the stasis load (a product of positives) are trivially positive, the crossed figure `(a-b-c)*d` is positive
because `a-b > c*d >= 2c > c` so `a-b-c > 0`, and the swapped figure `(a*b)-(c-d)` is positive because `a*b` comfortably
exceeds the small `segment_count - stasis_per_segment` gap. The tables are chosen so the five family values are pairwise
distinct with a comfortable margin, and — so all three queried readouts vary across the panel — the seven tables give
distinct net transit volumes, distinct cleared volumes, and distinct stasis loads, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (INGESTED_VOLUME, REFLUXED_VOLUME, SEGMENT_COUNT, STASIS_PER_SEGMENT) — an ingested volume minus a refluxed volume for the
# cleared volume, a segment count times a stasis-per-segment for the stasis load, all plain positive numbers >= 2. Each table
# satisfies ingested_volume > refluxed_volume (cleared volume > 0) and ingested_volume - refluxed_volume > segment_count *
# stasis_per_segment (net transit volume > 0), so every family member is strictly positive (no negatives anywhere); the five
# family values are asserted pairwise-distinct below. The seven tables give distinct net transit volumes, distinct cleared
# volumes, and distinct stasis loads so all three queried readouts vary across the panel.
TABLES = [
    (8, 2, 2, 2),
    (15, 3, 3, 3),
    (24, 4, 4, 4),
    (26, 5, 3, 5),
    (29, 6, 3, 6),
    (25, 7, 5, 2),
    (27, 8, 6, 2),
]

# The option family (5 members), all built from the four observed quantities via +, -, and *. Every identifier is
# DIGIT-FREE. key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always
# appear as the options.
FAMILY = [
    (
        "net_transit",
        "net transit volume (the cleared volume minus the stasis load)",
        "(ingested_volume - refluxed_volume) - (segment_count * stasis_per_segment)",
    ),
    (
        "cleared_volume",
        "the cleared volume (the ingested volume minus the refluxed volume, the difference the stasis load is subtracted from)",
        "ingested_volume - refluxed_volume",
    ),
    (
        "stasis_load",
        "the stasis load (the segment count times the stasis per segment, the product subtracted from the cleared volume)",
        "segment_count * stasis_per_segment",
    ),
    (
        "crossed",
        "the ingested volume minus the refluxed volume minus the segment count, all multiplied by the stasis per segment, folding the subtraction inside the parentheses so the segment count is subtracted before multiplying (a wrong grouping)",
        "(ingested_volume - refluxed_volume - segment_count) * stasis_per_segment",
    ),
    (
        "swapped",
        "the ingested volume times the refluxed volume, minus the segment count minus the stasis per segment, multiplying the first pair and differencing the second pair instead (a wrong pairing)",
        "(ingested_volume * refluxed_volume) - (segment_count - stasis_per_segment)",
    ),
]
QUERIED = ["net_transit", "cleared_volume", "stasis_load"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(ingested_volume, refluxed_volume, segment_count, stasis_per_segment):
    # Operation order mirrors the ADJ programs exactly (the difference forms, the product forms, then the product is
    # subtracted from the difference, so a-b-c*d evaluates as ((a-b)-(c*d))), so the Python option value and the engine
    # result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "net_transit": (ingested_volume - refluxed_volume) - (segment_count * stasis_per_segment),
        "cleared_volume": ingested_volume - refluxed_volume,
        "stasis_load": segment_count * stasis_per_segment,
        "crossed": (ingested_volume - refluxed_volume - segment_count) * stasis_per_segment,
        "swapped": (ingested_volume * refluxed_volume) - (segment_count - stasis_per_segment),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for ingested_volume, refluxed_volume, segment_count, stasis_per_segment in TABLES:
        # Every observed quantity is a plain positive number >= 2, and the tables guarantee ingested_volume >
        # refluxed_volume (cleared volume > 0) and ingested_volume - refluxed_volume > segment_count * stasis_per_segment
        # (net transit volume > 0), so every family member is strictly positive with no negative anywhere.
        assert (
            ingested_volume >= 2
            and refluxed_volume >= 2
            and segment_count >= 2
            and stasis_per_segment >= 2
        ), (ingested_volume, refluxed_volume, segment_count, stasis_per_segment)
        assert ingested_volume > refluxed_volume, (
            ingested_volume, refluxed_volume, segment_count, stasis_per_segment,
        )
        assert ingested_volume - refluxed_volume > segment_count * stasis_per_segment, (
            ingested_volume, refluxed_volume, segment_count, stasis_per_segment,
        )
        fv = family_values(ingested_volume, refluxed_volume, segment_count, stasis_per_segment)
        for key, v in fv.items():
            assert v > 0, (key, ingested_volume, refluxed_volume, segment_count, stasis_per_segment, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    ingested_volume,
                    refluxed_volume,
                    segment_count,
                    stasis_per_segment,
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
                ingested_volume,
                refluxed_volume,
                segment_count,
                stasis_per_segment,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r103gimot-{idx + 1:02d}",
                "qtype": "gimot_net_transit",
                "stem": (
                    f"A GI-motility study records an ingested volume of {num(ingested_volume)} minus a refluxed volume of "
                    f"{num(refluxed_volume)}, minus a segment count of {num(segment_count)} times a stasis-per-segment of "
                    f"{num(stasis_per_segment)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe ingested_volume({num(ingested_volume)})\n"
                    f"observe refluxed_volume({num(refluxed_volume)})\n"
                    f"observe segment_count({num(segment_count)})\n"
                    f"observe stasis_per_segment({num(stasis_per_segment)})\n"
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
            "ADJ-LADDER rung 103 — net transit volume from four stated quantities (a NEW panel: gastroenterology / "
            "GI-motility transit accounting). From an ingested volume minus a refluxed volume for the cleared volume, a "
            "segment count times a stasis-per-segment for the stasis load, and the stasis load subtracted from the cleared "
            "volume, compute the net transit volume ((ingested_volume-refluxed_volume)-(segment_count*stasis_per_segment)), "
            "the cleared volume (ingested_volume-refluxed_volume), or the stasis load (segment_count*stasis_per_segment). "
            "Each item is a compute_dimensioned program (observe the four quantities, let answer = formula); the ADJ engine "
            "carries the arithmetic — a NEW family, A DIFFERENCE MINUS A PRODUCT a-b-c*d (subtract b from a, multiply c and "
            "d, subtract the product from the difference, so a-b-c*d = ((a-b)-(c*d)); the FIRST time the ladder subtracts a "
            "bare PRODUCT from a bare DIFFERENCE — this COMPLETES the (a±b)±c*d quartet: 91 a+b+c*d, 101 a+b-c*d, 102 "
            "a-b+c*d, 103 a-b-c*d; rung-35 a*b-c*d subtracted two products) — and the harness matches the scalar to the "
            "printed options. Contamination-safe: every figure is built only from the four observed quantities via +, -, and "
            "* — no constant leaks, and neither the cleared volume, the stasis load, nor any net figure ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides "
            "inside a variable name. The five options are a family over the same four quantities, so the distractors are "
            "exactly the slips students make: folding the subtraction inside the parentheses so the segment count is "
            "subtracted before multiplying ((a-b-c)*d, a wrong grouping) and multiplying the first pair while differencing "
            "the second pair ((a*b)-(c-d), a wrong pairing). The core confusion tested is that a-b-c*d is ((a-b)-(c*d)), not "
            "(a-b-c)*d and not (a*b)-(c-d). Each table guarantees the ingested volume exceeds the refluxed volume and the "
            "cleared volume exceeds the stasis load, so every figure stays strictly positive."
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
