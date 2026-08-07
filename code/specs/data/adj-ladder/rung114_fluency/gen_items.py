"""Generate rung-114 (speech-language pathology / fluency) items.json for the ADJ-LADDER.

Rung 114 opens the **speech-language pathology / fluency assessment** panel on the quantitative band — the arithmetic of a
fluent-syllable index. A `first_passage` syllable count PLUS a `second_passage` syllable count, MINUS a `disfluent_count`
(the disfluent syllables), gives the fluent load, and that load is DIVIDED by a `session_span` (how many sessions the reading
is averaged over) to give the fluent-syllable index. A **three-term SUM-then-DIFFERENCE numerator, all over a divisor**
introduces a genuinely NEW arithmetic family on the ladder: `(a+b-c)/d`, i.e. `(((a + b) - c) / d)`.

This is genuinely new — the flat three-term numerators shipped so far were rung-108 `(a+b+c)/d` (all sums), rung-109
`(a-b+c)/d`, rung-110 `(a*b+c)/d`, and rung-111 `(a*b-c)/d`; NONE is the two-add-one-subtract `(a+b-c)/d`. It is the
sum-then-difference sibling of rung-108 `(a+b+c)/d` (swap the last `+` for a `-`) and distinct from rung-109 `(a-b+c)/d`
(which subtracts the MIDDLE term, not the last). The distributive pair (rung-112 `a*(b+c)/d`, rung-113 `a*(b-c)/d`) wrapped a
SUM/DIFFERENCE inside a factor; rung-114 goes back to a FLAT three-term numerator but with the previously-unshipped
add-then-subtract shape. Every earlier ratio used either a two-term numerator (rung-37 `(a+b)/(c+d)`, rung-99 `(a*b)/(c+d)`,
rung-100 `(a+b)/(c*d)`, rung-104 `(a-b)/(c*d)`, the difference-denominator trio rung-105 `(a+b)/(c-d)`, rung-106 `a*b/(c-d)`,
rung-107 `(a-b)/(c-d)`) or one of the four earlier three-term numerators (108-111) or the distributive pair (112-113).
Rung-114 moves to `(a+b-c)/d`. The operator order matters: `(a+b-c)/d` is `(((a+b)-c)/d)` (the two counts add, the disfluent
count subtracts, then the whole numerator is divided; `+` and `-` bind left-to-right and both precede `/` only via the
explicit numerator parentheses), NOT `a+b-c/d` (dropping the numerator parentheses so only the disfluent count is divided by
the divisor and then subtracted) and NOT `(a+b)/(c+d)` (regrouping so only the two counts form the numerator and the disfluent
count joins the divisor in the denominator) — the two distractors exploit exactly those confusions.

The setup: a `first_passage`, a `second_passage`, a `disfluent_count`, and a `session_span`. The total is:

  FLUENT-SYLLABLE INDEX  (first_passage + second_passage - disfluent_count) / session_span  [ a three-term numerator over a divisor ]
  FLUENT LOAD            first_passage + second_passage - disfluent_count                    [ the three-term numerator ]
  SESSION SPAN           session_span                                                        [ the divisor ]

The **fluent-syllable index** is what makes this rung distinctive — it is the ladder's first **two-add-one-subtract
three-term numerator, over a divisor**. It is a rate (fluent load per session), framed as an *index* to keep it
dimensionless-clean — the same discipline rungs 100/104/.../113 used for their ratios. (The fluent load `a+b-c` and the
session span `d` ride alongside as component readouts, so the panel teaches the whole calculation — exactly as rungs 47-113
shipped their component sums/products/differences/ratios beside the headline figure.)

Each figure is a `compute_dimensioned` program (`observe` the four quantities + `let answer = formula`); the ADJ engine carries
the arithmetic — the addition of the two passage counts, the subtraction of the disfluent count into the fluent load, then the
division of that load by the session span (the flat three-term numerator over the divisor, so (a+b-c)/d evaluates as
(((a+b)-c)/d)) — and the harness reads the scalar via the existing `compute_dimensioned` extractor. No harness/engine change,
exactly as rungs 8/16/.../112/113. This rung exercises the engine across a **two-add-one-subtract numerator, over a divisor** —
the fact that `(a+b-c)/d` is `(((a+b)-c)/d)` and NOT `a+b-c/d` and NOT `(a+b)/(c+d)` made computable. The ratio golds are
non-integer f64s; the engine's IEEE-double division matches Python's the same way rungs 99/100/104/.../113 relied on (well
within the harness's 1e-9 tolerance).

Contamination-safe by construction: every formula is built ONLY from the four observed quantities via `+`, `-` and `/` — **no
structural constants** — so no numeric literal appears in any program, and neither the fluent load, the session span, nor any
index is ever a literal (each is computed from the observed quantities). The observed quantities carry **digit-free
identifiers** (`first_passage`, `second_passage`, `disfluent_count`, `session_span`) so no numeral hides inside a variable name.

The five options are a tight family over the same four quantities: the three real readouts plus the two classic slips —

  CROSSED    first_passage + second_passage - disfluent_count / session_span  drop the numerator parentheses so only the
                                                                    disfluent count is divided by the session span and then
                                                                    subtracted (the classic `(a+b-c)/d` vs `a+b-c/d` grouping
                                                                    error), and
  SWAPPED    (first_passage + second_passage) / (disfluent_count + session_span)  regroup so only the two passage counts form
                                                                    the numerator and the disfluent count joins the session span
                                                                    in the denominator (`(a+b)/(c+d)` instead of `(a+b-c)/d`),

which are exactly the mistakes a student makes (failing to keep the whole three-term numerator over the divisor, or regrouping
which terms belong to the numerator vs the divisor). Gold rotates A-E by index. QUERIED (used as gold) = the three real
readouts; all five always appear as options.

Distinctness and positivity: this rung SUBTRACTS the disfluent count in the numerator, so positivity is guaranteed by table
construction. Each table guarantees **first_passage + second_passage > disfluent_count** (so the numerator `a+b-c` is strictly
positive, the fluent load is positive, and the index is positive) AND all quantities `>= 2` (so the crossed slip
`a+b - c/d` stays positive because `a+b > c > c/d`, and the swapped denominator `c+d >= 4`). The **session_span >= 2** keeps the
divisor away from zero, the fluent-syllable index never coincides with the session span or the fluent load, and the five family
values are pairwise distinct with a comfortable margin; and — so all three queried readouts vary across the panel — the seven
tables give distinct fluent-syllable indices, distinct fluent loads, and distinct session spans, all asserted at build time.
"""
import json

LETTERS = ["A", "B", "C", "D", "E"]

# (FIRST_PASSAGE, SECOND_PASSAGE, DISFLUENT_COUNT, SESSION_SPAN) — two passage syllable counts summed, the disfluent count
# subtracted for the fluent load, all divided by a session span, all plain positive numbers >= 2. This rung SUBTRACTS the
# disfluent count in the numerator, so every table guarantees first_passage + second_passage > disfluent_count (a+b>c) which
# keeps the numerator, the fluent load, and the index strictly positive; session_span >= 2 keeps the divisor away from zero. The
# five family values are asserted pairwise-distinct below. The seven tables give distinct fluent-syllable indices, distinct
# fluent loads, and distinct session spans so all three queried readouts vary across the panel.
TABLES = [
    (4, 3, 2, 2),
    (5, 3, 2, 3),
    (5, 4, 2, 4),
    (6, 5, 2, 5),
    (7, 5, 2, 6),
    (7, 6, 2, 7),
    (8, 7, 2, 8),
]

# The option family (5 members), all built from the four observed quantities via +, - and /. Every identifier is DIGIT-FREE.
# key -> (display name, formula-as-adj). Only the first three are *queried* (used as gold); all five always appear as the
# options.
FAMILY = [
    (
        "fluent_index",
        "fluent-syllable index (the fluent load divided by the session span)",
        "(first_passage + second_passage - disfluent_count) / session_span",
    ),
    (
        "fluent_load",
        "the fluent load (the two passage counts summed minus the disfluent count, the numerator divided by the session span)",
        "first_passage + second_passage - disfluent_count",
    ),
    (
        "session_span",
        "the session span (the divisor the fluent load is divided by)",
        "session_span",
    ),
    (
        "crossed",
        "the first passage plus the second passage minus the disfluent count divided by the session span, dropping the numerator parentheses so only the disfluent count is divided (a wrong grouping)",
        "first_passage + second_passage - disfluent_count / session_span",
    ),
    (
        "swapped",
        "the first passage plus the second passage, divided by the disfluent count plus the session span, regrouping so only the two passage counts form the numerator (a wrong pairing)",
        "(first_passage + second_passage) / (disfluent_count + session_span)",
    ),
]
QUERIED = ["fluent_index", "fluent_load", "session_span"]
ORDER = [k for k, _, _ in FAMILY]


def scalar(v):
    return {"value": v, "unit": "scalar"}


def family_values(first_passage, second_passage, disfluent_count, session_span):
    # Operation order mirrors the ADJ programs exactly (the two passage counts add, the disfluent count subtracts into the fluent
    # load, then that numerator is divided by the session span, so (a+b-c)/d evaluates as (((a+b)-c)/d)), so the Python option
    # value and the engine result are the same IEEE-double (well within the harness's 1e-9 match tolerance).
    return {
        "fluent_index": (first_passage + second_passage - disfluent_count) / session_span,
        "fluent_load": first_passage + second_passage - disfluent_count,
        "session_span": session_span,
        "crossed": first_passage + second_passage - disfluent_count / session_span,
        "swapped": (first_passage + second_passage) / (disfluent_count + session_span),
    }


def build():
    name_of = {k: nm for k, nm, _ in FAMILY}
    formula_of = {k: f for k, _, f in FAMILY}
    items = []
    idx = 0

    def num(x):
        return int(x) if float(x).is_integer() else x

    for first_passage, second_passage, disfluent_count, session_span in TABLES:
        # Every observed quantity is a plain positive number >= 2, and this rung SUBTRACTS the disfluent count in the numerator,
        # so each table guarantees first_passage + second_passage > disfluent_count (the numerator a+b-c is strictly positive)
        # which keeps every family member strictly positive; session_span >= 2 keeps the divisor away from zero.
        assert (
            first_passage >= 2
            and second_passage >= 2
            and disfluent_count >= 2
            and session_span >= 2
        ), (first_passage, second_passage, disfluent_count, session_span)
        assert first_passage + second_passage > disfluent_count, (
            first_passage, second_passage, disfluent_count, session_span,
        )
        fv = family_values(first_passage, second_passage, disfluent_count, session_span)
        for key, v in fv.items():
            assert v > 0, (key, first_passage, second_passage, disfluent_count, session_span, fv)
        # Pairwise-distinct with a comfortable margin so the harness never sees two options
        # matching the gold value within its 1e-9 tolerance.
        vals = [fv[key] for key in ORDER]
        for i in range(len(vals)):
            for j in range(i + 1, len(vals)):
                assert abs(vals[i] - vals[j]) > 1e-6, (
                    first_passage,
                    second_passage,
                    disfluent_count,
                    session_span,
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
                first_passage,
                second_passage,
                disfluent_count,
                session_span,
                opts_vals,
            )
            options = {LETTERS[i]: scalar(opts_vals[i]) for i in range(5)}
            items.append({
                "id": f"r114fluency-{idx + 1:02d}",
                "qtype": "fluent_index",
                "stem": (
                    f"A fluency assessment records a first passage of {num(first_passage)} plus a second passage of "
                    f"{num(second_passage)} minus a disfluent count of {num(disfluent_count)}, divided by a session span of "
                    f"{num(session_span)}. What is the {name_of[key]}?"
                ),
                "program": (
                    f"observe first_passage({num(first_passage)})\n"
                    f"observe second_passage({num(second_passage)})\n"
                    f"observe disfluent_count({num(disfluent_count)})\n"
                    f"observe session_span({num(session_span)})\n"
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
            "ADJ-LADDER rung 114 — fluent-syllable index from four stated quantities (a NEW panel: speech-language pathology / "
            "fluency assessment). From a first passage plus a second passage minus a disfluent count for the fluent load, all "
            "divided by a session span, compute the fluent-syllable index "
            "((first_passage+second_passage-disfluent_count)/session_span), the fluent load "
            "(first_passage+second_passage-disfluent_count), or the session span. Each item is a compute_dimensioned program "
            "(observe the four quantities, let answer = formula); the ADJ engine carries the arithmetic — a NEW family, a "
            "TWO-ADD-ONE-SUBTRACT THREE-TERM NUMERATOR, OVER A DIVISOR (a+b-c)/d (add the two passage counts, subtract the "
            "disfluent count, divide by the session span, so (a+b-c)/d = (((a+b)-c)/d); the sum-then-difference sibling of "
            "rung-108 (a+b+c)/d and distinct from rung-109 (a-b+c)/d which subtracts the MIDDLE term. The earlier three-term "
            "numerators (108 (a+b+c)/d, 109 (a-b+c)/d, 110 (a*b+c)/d, 111 (a*b-c)/d) never used two-add-one-subtract; the "
            "distributive pair (112 a*(b+c)/d, 113 a*(b-c)/d) wrapped a sum/difference inside a factor. Every earlier ratio used "
            "a TWO-term numerator: 37 (a+b)/(c+d), 99 (a*b)/(c+d), 100 (a+b)/(c*d), 104 (a-b)/(c*d), and the difference-"
            "denominator trio 105 (a+b)/(c-d), 106 a*b/(c-d), 107 (a-b)/(c-d)) — and the harness matches the scalar to the "
            "printed options. The fluent-syllable index is a rate (fluent load per session), framed as an INDEX so the "
            "dimensionless value stays honest. Contamination-safe: every figure is built only from the four observed quantities "
            "via +, - and / — no constant leaks, and neither the fluent load, the session span, nor any index ever appears as a "
            "literal (each is computed) — and the observed quantities carry digit-free identifiers so no numeral hides inside a "
            "variable name. The five options are a family over the same four quantities, so the distractors are exactly the "
            "slips students make: dropping the numerator parentheses so only the disfluent count is divided (a+b-c/d, a wrong "
            "grouping) and regrouping so only the two passage counts form the numerator ((a+b)/(c+d), a wrong pairing). The core "
            "confusion tested is that (a+b-c)/d is (((a+b)-c)/d), not a+b-c/d and not (a+b)/(c+d). This rung SUBTRACTS the "
            "disfluent count in the numerator, so positivity is guaranteed by table construction: every table has "
            "first_passage + second_passage > disfluent_count (a+b>c) and session_span >= 2 (divisor never zero), keeping every "
            "family member strictly positive."
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
