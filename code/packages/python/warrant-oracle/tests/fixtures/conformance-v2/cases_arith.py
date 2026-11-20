"""``v2-arith``: one evaluator node over ``lit`` operands (slice S5a table).

``type`` is the type of **both** operands.  The intents are written with plain Python integer
arithmetic and the ``wrap`` helper (``value mod 2**w``, re-read two's complement), which is the
rule of section 3.2 stated in one line; the oracle evaluates the same node through its typed IR
evaluator.  The ten slugs of section 3.4 are present, in the family below or spelled out:
``u8-add-wrap``, ``u8-sub-wrap``, ``u8-mul-wrap``, ``u64-add-wrap``, ``u64-mul-wrap``,
``i8-neg-min``, ``i64-sub-wrap``, ``nat-sub-is-int``, ``nat-bound-generation-error`` and
``v0-u8-wrap-guard``.
"""

from __future__ import annotations

from kit_common import FIXED, hi, lo, row, wrap

ROWS: list = []
NAT_MAX = 2**64 - 1


def case(slug, summary, ty, op, lhs, rhs, intent, document=None):
    fields = {"type": ty, "op": op, "lhs": str(lhs)}
    if rhs is not None:
        fields["rhs"] = str(rhs)
    if document is not None:
        fields["document"] = document
    ROWS.append(row(slug, summary, fields, intent))


def value(ty, v):
    return {"type": ty, "value": str(v)}


def truth(b):
    return {"type": "Bool", "value": b}


# -- the eight fixed-width types: every operator wraps modulo 2**w (section 3.2) ---------------------
for t in FIXED:
    k = t.lower()
    signed = FIXED[t][1]
    big, small = hi(t), lo(t)
    # add
    case(f"{k}-add-wrap", f"{t} {big} + 1 wraps", t, "add", big, 1, value(t, wrap(big + 1, t)))
    case(f"{k}-add-exact", f"{t} 1 + 2 does not wrap", t, "add", 1, 2, value(t, 3))
    # sub
    a, b = (small, 1) if signed else (0, 1)
    case(f"{k}-sub-wrap", f"{t} {a} - 1 wraps", t, "sub", a, b, value(t, wrap(a - b, t)))
    case(f"{k}-sub-exact", f"{t} 5 - 3 does not wrap", t, "sub", 5, 3, value(t, 2))
    # mul
    if t == "U8":
        a, b = 16, 16
    elif signed:
        a, b = small, -1
    else:
        a, b = big, big
    case(f"{k}-mul-wrap", f"{t} {a} * {b} wraps", t, "mul", a, b, value(t, wrap(a * b, t)))
    c, d = (-3, 5) if signed else (3, 5)
    case(f"{k}-mul-exact", f"{t} {c} * {d} does not wrap", t, "mul", c, d, value(t, c * d))
    # neg: unary minus wraps too, so the signed minimum is its own negation
    case(f"{k}-neg-zero", f"{t} -0 is 0", t, "neg", 0, None, value(t, 0))
    case(f"{k}-neg-one", f"{t} -1 is read back in {t}", t, "neg", 1, None, value(t, wrap(-1, t)))
    case(f"{k}-neg-min", f"{t} negating the minimum {small}", t, "neg", small, None, value(t, wrap(-small, t)))
    case(f"{k}-neg-max", f"{t} negating the maximum {big}", t, "neg", big, None, value(t, wrap(-big, t)))

# -- the v0 guard keeps its id with a lane-observable meaning (D45) ------------------------------------
case("v0-u8-wrap-guard", "255u8 + 1u8 = 0u8 in a corpus compiled from a v0 document: a lane wraps as usual",
     "U8", "add", 255, 1, value("U8", 0), document="v0")

# -- Nat is exact; Nat - Nat is an Int (section 3.2) ---------------------------------------------------------
case("nat-add-exact", "Nat 5 + 7 is exact and has type Int", "Nat", "add", 5, 7, value("Int", 12))
case("nat-add-large", "two Nat maxima add exactly", "Nat", "add", NAT_MAX, NAT_MAX, value("Int", 2 * NAT_MAX))
case("nat-mul-small", "Nat 6 * 7", "Nat", "mul", 6, 7, value("Int", 42))
case("nat-mul-below-window", "10^19 * 10^19 = 10^38 is inside the window", "Nat", "mul", 10**19, 10**19,
     value("Int", 10**38))
case("nat-sub-is-int", "Nat 3 - 5 = -2 has type Int", "Nat", "sub", 3, 5, value("Int", -2))
case("nat-sub-zero", "Nat 5 - 5 = 0 is still an Int", "Nat", "sub", 5, 5, value("Int", 0))
case("nat-sub-positive", "Nat 9 - 4 = 5 is an Int", "Nat", "sub", 9, 4, value("Int", 5))
case("nat-sub-large-negative", "Nat 0 - (2^64-1)", "Nat", "sub", 0, NAT_MAX, value("Int", -NAT_MAX))
case("nat-neg", "negating a Nat gives an Int", "Nat", "neg", 5, None, value("Int", -5))
case("nat-neg-zero", "negating Nat 0", "Nat", "neg", 0, None, value("Int", 0))
case("nat-bound-generation-error", "(2^64-1) * (2^64-1) leaves the 2^127 window", "Nat", "mul", NAT_MAX, NAT_MAX,
     {"error": "ir-overflow"})
case("nat-mul-max-by-two", "(2^64-1) * 2 stays inside the window", "Nat", "mul", NAT_MAX, 2, value("Int", 2 * NAT_MAX))

# -- Int (model-internal, written as a lit type) --------------------------------------------------------------
case("int-add-negative", "Int -3 + -4", "Int", "add", -3, -4, value("Int", -7))
case("int-sub-negative", "Int -3 - -4", "Int", "sub", -3, -4, value("Int", 1))
case("int-mul-negative", "Int -6 * 7", "Int", "mul", -6, 7, value("Int", -42))
case("int-neg-negative", "negating Int -5", "Int", "neg", -5, None, value("Int", 5))
case("int-mul-overflow", "(10^20-1)^2 is about 10^40, past the window", "Int", "mul", 10**20 - 1, 10**20 - 1,
     {"error": "ir-overflow"})
case("int-mul-below-window", "10^19 * 10^19 as Int", "Int", "mul", 10**19, 10**19, value("Int", 10**38))

# -- comparisons order by mathematical value (section 3.2) ---------------------------------------------------------
case("i8-lt-minus-one-one", "I8 -1 < 1 (a bit-pattern compare would say 255 < 1)", "I8", "lt", -1, 1, truth(True))
case("i8-gt-one-minus-one", "I8 1 > -1", "I8", "gt", 1, -1, truth(True))
case("i8-le-minus-one-one", "I8 -1 <= 1", "I8", "le", -1, 1, truth(True))
case("i8-ge-minus-one-one", "I8 -1 >= 1 is false", "I8", "ge", -1, 1, truth(False))
case("i8-lt-min-max", "I8 -128 < 127", "I8", "lt", -128, 127, truth(True))
case("i8-gt-min-max", "I8 -128 > 127 is false", "I8", "gt", -128, 127, truth(False))
case("i8-eq-min-min", "I8 -128 == -128", "I8", "eq", -128, -128, truth(True))
case("i8-ne-min-max", "I8 -128 != 127", "I8", "ne", -128, 127, truth(True))
case("i16-lt-min-zero", "I16 minimum < 0", "I16", "lt", lo("I16"), 0, truth(True))
case("i32-gt-zero-min", "I32 0 > minimum", "I32", "gt", 0, lo("I32"), truth(True))
case("i64-lt-min-max", "I64 minimum < maximum", "I64", "lt", lo("I64"), hi("I64"), truth(True))
case("i64-ge-max-min", "I64 maximum >= minimum", "I64", "ge", hi("I64"), lo("I64"), truth(True))
case("u8-lt-255-1", "U8 255 < 1 is false", "U8", "lt", 255, 1, truth(False))
case("u8-gt-255-1", "U8 255 > 1", "U8", "gt", 255, 1, truth(True))
case("u8-eq-same", "U8 7 == 7", "U8", "eq", 7, 7, truth(True))
case("u8-ne-same", "U8 7 != 7 is false", "U8", "ne", 7, 7, truth(False))
case("u8-le-equal", "U8 7 <= 7", "U8", "le", 7, 7, truth(True))
case("u8-ge-equal", "U8 7 >= 7", "U8", "ge", 7, 7, truth(True))
case("u16-lt-small-large", "U16 2 < 65535", "U16", "lt", 2, 65535, truth(True))
case("u32-gt-max-zero", "U32 maximum > 0", "U32", "gt", hi("U32"), 0, truth(True))
case("u64-lt-max-zero", "U64 maximum < 0 is false", "U64", "lt", hi("U64"), 0, truth(False))
case("u64-gt-max-zero", "U64 maximum > 0", "U64", "gt", hi("U64"), 0, truth(True))
case("u64-eq-max-max", "U64 maximum == maximum", "U64", "eq", hi("U64"), hi("U64"), truth(True))
case("nat-lt-small", "Nat 2 < 3", "Nat", "lt", 2, 3, truth(True))
case("nat-ge-max-zero", "Nat maximum >= 0", "Nat", "ge", NAT_MAX, 0, truth(True))
case("nat-eq-max", "Nat maximum == maximum", "Nat", "eq", NAT_MAX, NAT_MAX, truth(True))
case("nat-ne-zero-one", "Nat 0 != 1", "Nat", "ne", 0, 1, truth(True))
case("int-lt-negative", "Int -5 < 3", "Int", "lt", -5, 3, truth(True))
case("int-gt-negative", "Int -5 > -6", "Int", "gt", -5, -6, truth(True))
case("int-eq-negative", "Int -1 == -1", "Int", "eq", -1, -1, truth(True))
