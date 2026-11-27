"""``v2-ir``: the model-IR evaluator at ``ir.core`` (slice S5a table).

Every case is ``levels``, ``ir``, ``env``, ``types``, ``models``, ``model_order``, ``ops``,
``subjects`` (and ``max_ir_cost`` or ``role`` when not the default).  ``subjects`` names, per op, the
pure model that stands in for the subject, so a lane binds the op to that model and *counts* its
invocations; the oracle derives ``value`` from the tree with each ``subject_op`` rewritten to that
``model_call`` and ``calls`` from the unrewritten tree by the pure trace of section 4.4.

The rows are grouped: every Appendix A tag, short-circuiting and call counting, then each of the
nine error codes.  A result type is chosen so that the case does not depend on a static width of a
``Bytes`` literal or on the unknown error type of a bare ``ok`` (see the README): byte results come
from variables, and a ``Result`` result comes from an ``if`` whose other branch has a declared type.
"""

from __future__ import annotations

from kit_common import hi, lo, row

ROWS: list = []
CORE = ["ir.core"]
ERR = {"E": {"kind": "error", "cases": ["BAD", "WORSE"]}}
NAT_MAX = 2**64 - 1
INT_MAX = 2**127 - 1
# [D58] no ``lit`` carries more than 20 decimal digits, ``Int`` included, so the edge of the Int window is
# reached by arithmetic: (2^63) * (2^64 - 1) + (2^63 - 1) = 2^127 - 1, from three literals of at most 20 digits.
EDGE = ["add", ["mul", ["lit", {"type": "Int", "value": str(2**63)}], ["lit", {"type": "Int", "value": str(NAT_MAX)}]],
        ["lit", {"type": "Int", "value": str(2**63 - 1)}]]
NEG_EDGE = ["neg", EDGE]


# -- IR builders -------------------------------------------------------------------------------------------
def lit(t, v):
    return ["lit", {"type": t, "value": str(v)}]


def blit(b):
    return ["lit", b]


def var(n):
    return ["var", n]


def sub_op(name, *args):
    return ["subject_op", name, list(args)]


def call(name, *args):
    return ["model_call", name, list(args)]


def e(name, ty, value):
    return [name, {"type": ty, "value": value}]


def model(params, body):
    return {"params": [list(p) for p in params], "body": body}


def op(params, ret):
    """An op row as a corpus writer emits it: ``requires`` is computed from the signature (R9), ``ir.tuple`` when a type holds a tuple."""
    tuple_typed = any("(" in t for t in [*(t for _, t in params), ret])
    return {"params": [list(p) for p in params], "ret": ret, "requires": ["ir.core", "ir.tuple"] if tuple_typed else CORE}


# Stand-ins, used by many rows: bump(a) = a + 1 on U8 and add2(a, b) = a + b on U8.
INC = model([("a", "U8")], ["add", var("a"), lit("U8", 1)])
ADDM = model([("a", "U8"), ("b", "U8")], ["add", var("a"), var("b")])
BUMP = op([("a", "U8")], "U8")
ADD2 = op([("a", "U8"), ("b", "U8")], "U8")


def case(slug, summary, ir, intent, *, env=(), models=None, order=None, ops=None, subjects=None,
         types=None, levels=("ir.core",), max_ir_cost=None, role=None, requires=None):
    models = models or {}
    fields = {
        "levels": sorted(levels),
        "ir": ir,
        "env": [list(x) for x in env],
        "types": types or {},
        "models": models,
        "model_order": list(models) if order is None else list(order),
        "ops": ops or {},
        "subjects": subjects or {},
    }
    if max_ir_cost is not None:
        fields["max_ir_cost"] = max_ir_cost
    if role is not None:
        fields["role"] = role
    ROWS.append(row(slug, summary, fields, intent, requires))


def ok(ty, value, calls=0):
    return {"type": ty, "value": value, "calls": calls}


def err(code, klass=None):
    out = {"error": code}
    if klass:
        out["class"] = klass
    return out


# ===============================================================================================
# 1. Every tag of Appendix A, a positive case and an edge case each
# ===============================================================================================

# lit: every typed form
case("lit-bool-true", "a Bool literal is a bare JSON boolean", blit(True), ok("Bool", True))
case("lit-bool-false", "false", blit(False), ok("Bool", False))
for t in ("U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64"):
    case(f"lit-{t.lower()}-max", f"a {t} literal at the maximum", lit(t, hi(t)), ok(t, str(hi(t))))
    case(f"lit-{t.lower()}-min", f"a {t} literal at the minimum", lit(t, lo(t)), ok(t, str(lo(t))))
case("lit-nat", "a Nat literal at 2^64-1", lit("Nat", NAT_MAX), ok("Nat", str(NAT_MAX)))
case("lit-int-negative", "an Int literal is the one place Int is written", lit("Int", -3), ok("Int", "-3"))
case("lit-signed-minimum-is-neg-of-lit", "the signed minimum is neg of a lit holding -2^(w-1), and neg wraps it to itself",
     ["neg", lit("I8", -128)], ok("I8", "-128"))
case("lit-bytes-equal-var", "a Bytes literal is lowercase hex", ["eq", lit("Bytes", "0a0b"), var("b")],
     ok("Bool", True), env=[e("b", "Bytes", "0a0b")])
case("lit-bytes-empty", "the empty Bytes literal", ["eq", lit("Bytes", ""), var("b")], ok("Bool", True),
     env=[e("b", "Bytes", "")])

# var
case("var-u8", "a variable reads the environment", var("a"), ok("U8", "5"), env=[e("a", "U8", "5")])
case("var-bytes", "a Bytes variable", var("b"), ok("Bytes", "0a0b"), env=[e("b", "Bytes", "0a0b")])
case("var-result", "a Result variable", var("r"), ok("Result<U8,E>", {"err": "WORSE"}),
     env=[e("r", "Result<U8,E>", {"err": "WORSE"})], types=ERR)
case("var-shadow-free-second-binding", "two bindings are independent", ["sub", var("a"), var("b")],
     ok("U8", "2"), env=[e("a", "U8", "5"), e("b", "U8", "3")])

# subject_op
case("subject-op-one-call", "one subject call is counted and stands in as inc", sub_op("bump", var("a")),
     ok("U8", "6", 1), env=[e("a", "U8", "5")], models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
case("subject-op-wraps-in-stand-in", "the stand-in model wraps like any U8 add", sub_op("bump", var("a")),
     ok("U8", "0", 1), env=[e("a", "U8", "255")], models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
case("subject-op-nested-args", "a subject call nested in the arguments of another counts once each",
     sub_op("bump", sub_op("bump", var("a"))), ok("U8", "7", 2), env=[e("a", "U8", "5")],
     models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
case("subject-op-two-ops", "two different ops, each with its stand-in", sub_op("add2", sub_op("bump", var("a")), var("b")),
     ok("U8", "9", 2), env=[e("a", "U8", "5"), e("b", "U8", "3")], models={"inc": INC, "addm": ADDM},
     ops={"bump": BUMP, "add2": ADD2}, subjects={"bump": "inc", "add2": "addm"})
case("subject-op-nested-associativity", "add2(add2(a,b),c) == add2(a,add2(b,c)): four calls per case",
     ["eq", sub_op("add2", sub_op("add2", var("a"), var("b")), var("c")),
      sub_op("add2", var("a"), sub_op("add2", var("b"), var("c")))],
     ok("Bool", True, 4), env=[e("a", "U8", "3"), e("b", "U8", "4"), e("c", "U8", "5")],
     models={"addm": ADDM}, ops={"add2": ADD2}, subjects={"add2": "addm"})
case("subject-op-argument-of-model", "a subject call in the argument of a model call counts once",
     call("inc", sub_op("bump", var("a"))), ok("U8", "7", 1), env=[e("a", "U8", "5")],
     models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
case("subject-op-under-arithmetic", "a subject call below an add", ["add", sub_op("bump", var("a")), lit("U8", 10)],
     ok("U8", "16", 1), env=[e("a", "U8", "5")], models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
case("subject-op-stand-in-differs-from-model-call", "the stand-in's value is the subject's value for this case",
     ["eq", sub_op("bump", var("a")), call("inc", var("a"))], ok("Bool", True, 1), env=[e("a", "U8", "9")],
     models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})

# model_call
case("model-call-pure", "a pure model call is not a subject call", call("inc", var("a")), ok("U8", "6", 0),
     env=[e("a", "U8", "5")], models={"inc": INC})
case("model-call-wraps", "a model body wraps like any U8 add", call("inc", var("a")), ok("U8", "0", 0),
     env=[e("a", "U8", "255")], models={"inc": INC})
case("model-call-two-args", "a model with two parameters", call("addm", var("a"), var("b")), ok("U8", "7", 0),
     env=[e("a", "U8", "3"), e("b", "U8", "4")], models={"addm": ADDM})
case("model-call-earlier-model", "a model may call a model declared before it",
     call("twice", var("a")), ok("U8", "7", 0), env=[e("a", "U8", "5")],
     models={"inc": INC, "twice": model([("a", "U8")], call("inc", call("inc", var("a"))))}, order=["inc", "twice"])

# len
case("len-three", "len of a Bytes", ["len", var("b")], ok("Nat", "3"), env=[e("b", "Bytes", "0a0b0c")])
case("len-empty", "len of the empty Bytes", ["len", var("b")], ok("Nat", "0"), env=[e("b", "Bytes", "")])

# ok / err / is_err
case("ok-equals-ok-variable", "ok(e) is the Ok value of e", ["eq", ["ok", var("a")], var("r")], ok("Bool", True),
     env=[e("a", "U8", "5"), e("r", "Result<U8,E>", {"ok": "5"})], types=ERR)
case("ok-differs-in-payload", "Ok(5) is not Ok(6)", ["eq", ["ok", var("a")], var("r")], ok("Bool", False),
     env=[e("a", "U8", "5"), e("r", "Result<U8,E>", {"ok": "6"})], types=ERR)
case("err-equals-err-variable", "err(CASE) names a case", ["eq", ["err", "BAD"], var("r")], ok("Bool", True),
     env=[e("r", "Result<U8,E>", {"err": "BAD"})], types=ERR)
case("err-nominal-cases", "errors are nominal: BAD is not WORSE", ["eq", ["err", "BAD"], ["err", "WORSE"]],
     ok("Bool", False), types=ERR)
case("ok-is-not-err", "an Ok is never an Err", ["eq", ["ok", var("a")], ["err", "BAD"]], ok("Bool", False),
     env=[e("a", "U8", "5")], types=ERR)
case("is-err-on-err", "is_err of an Err", ["is_err", var("r")], ok("Bool", True),
     env=[e("r", "Result<U8,E>", {"err": "BAD"})], types=ERR)
case("is-err-on-ok", "is_err of an Ok", ["is_err", var("r")], ok("Bool", False),
     env=[e("r", "Result<U8,E>", {"ok": "1"})], types=ERR)
case("is-err-of-bare-ok", "is_err of a bare ok node", ["is_err", ["ok", blit(True)]], ok("Bool", False), types=ERR)
case("is-err-of-bare-err", "is_err of a bare err node", ["is_err", ["err", "BAD"]], ok("Bool", True), types=ERR)
case("result-from-if-keeps-declared-type", "an if of a Result variable and an err takes the variable's type",
     ["if", ["eq", var("a"), lit("U8", 0)], ["err", "BAD"], var("r")],
     ok("Result<U8,E>", {"ok": "4"}), env=[e("a", "U8", "1"), e("r", "Result<U8,E>", {"ok": "4"})], types=ERR)

# eq ne lt gt le ge
case("eq-bytes-equal", "Bytes equality is structural", ["eq", var("a"), var("b")], ok("Bool", True),
     env=[e("a", "Bytes", "0102"), e("b", "Bytes", "0102")])
case("eq-bytes-length-differs", "a prefix is not equal", ["eq", var("a"), var("b")], ok("Bool", False),
     env=[e("a", "Bytes", "01"), e("b", "Bytes", "0102")])
case("eq-bool", "Bool equality", ["eq", blit(True), blit(True)], ok("Bool", True))
case("eq-nat-int-widens", "Nat widens to Int in a comparison", ["eq", lit("Nat", 3), lit("Int", 3)], ok("Bool", True))
case("ne-u8", "ne on equal operands is false", ["ne", lit("U8", 7), lit("U8", 7)], ok("Bool", False))
case("ne-bytes", "ne on different Bytes", ["ne", var("a"), var("b")], ok("Bool", True),
     env=[e("a", "Bytes", "00"), e("b", "Bytes", "01")])
case("lt-i8-signed-order", "a signed compare is by value, not by bit pattern", ["lt", lit("I8", -1), lit("I8", 1)],
     ok("Bool", True))
case("lt-nat-int-negative", "a Nat is never below a negative Int", ["lt", lit("Nat", 0), lit("Int", -1)], ok("Bool", False))
case("gt-u8-unsigned-order", "255 > 1 for an unsigned type", ["gt", lit("U8", 255), lit("U8", 1)], ok("Bool", True))
case("gt-equal-is-false", "gt on equal operands", ["gt", lit("U8", 4), lit("U8", 4)], ok("Bool", False))
case("le-equal", "le on equal operands", ["le", lit("U16", 4), lit("U16", 4)], ok("Bool", True))
case("le-i64-min", "I64 minimum <= 0", ["le", lit("I64", lo("I64")), lit("I64", 0)], ok("Bool", True))
case("ge-equal", "ge on equal operands", ["ge", lit("U64", NAT_MAX), lit("U64", NAT_MAX)], ok("Bool", True))
case("ge-signed-negative", "I32 -5 >= -4 is false", ["ge", lit("I32", -5), lit("I32", -4)], ok("Bool", False))

# add sub mul
case("add-u8-wraps", "U8 255 + 1 wraps to 0 in the IR", ["add", var("a"), var("b")], ok("U8", "0"),
     env=[e("a", "U8", "255"), e("b", "U8", "1")])
case("add-u8-no-wrap", "U8 100 + 100 does not wrap", ["add", var("a"), var("b")], ok("U8", "200"),
     env=[e("a", "U8", "100"), e("b", "U8", "100")])
case("add-u8-wraps-mid", "U8 200 + 100 wraps to 44 (saturating would give 255)", ["add", var("a"), var("b")],
     ok("U8", "44"), env=[e("a", "U8", "200"), e("b", "U8", "100")])
case("add-i8-wraps", "I8 127 + 1 wraps to -128", ["add", lit("I8", 127), lit("I8", 1)], ok("I8", "-128"))
case("add-nat-is-exact", "Nat + Nat is exact and has type Int", ["add", lit("Nat", NAT_MAX), lit("Nat", 1)],
     ok("Int", str(NAT_MAX + 1)))
case("add-nat-int-widens", "Nat + Int widens to Int", ["add", lit("Nat", 5), lit("Int", -7)], ok("Int", "-2"))
case("sub-u8-wraps", "U8 0 - 1 wraps to 255", ["sub", lit("U8", 0), lit("U8", 1)], ok("U8", "255"))
case("sub-u64-wraps", "U64 0 - 1 wraps to 2^64-1", ["sub", lit("U64", 0), lit("U64", 1)], ok("U64", str(NAT_MAX)))
case("sub-nat-is-int", "Nat - Nat has type Int", ["sub", lit("Nat", 3), lit("Nat", 5)], ok("Int", "-2"))
case("sub-i16-wraps", "I16 minimum - 1 wraps", ["sub", lit("I16", lo("I16")), lit("I16", 1)], ok("I16", str(hi("I16"))))
case("mul-u8-wraps", "U8 16 * 16 wraps to 0", ["mul", lit("U8", 16), lit("U8", 16)], ok("U8", "0"))
case("mul-u64-wraps", "U64 maximum squared is 1", ["mul", lit("U64", NAT_MAX), lit("U64", NAT_MAX)], ok("U64", "1"))
case("mul-nat-below-window", "10^19 * 10^19 is exact", ["mul", lit("Nat", 10**19), lit("Nat", 10**19)], ok("Int", str(10**38)))
case("mul-i8-wraps", "I8 -128 * -1 wraps back to -128", ["mul", lit("I8", -128), lit("I8", -1)], ok("I8", "-128"))

# and or implies not if
case("and-true-true", "and", ["and", blit(True), blit(True)], ok("Bool", True))
case("and-true-false", "and with a false right operand", ["and", blit(True), blit(False)], ok("Bool", False))
case("or-false-false", "or", ["or", blit(False), blit(False)], ok("Bool", False))
case("or-false-true", "or with a true right operand", ["or", blit(False), blit(True)], ok("Bool", True))
case("implies-false-false", "false implies anything", ["implies", blit(False), blit(False)], ok("Bool", True))
case("implies-true-false", "true implies false is false", ["implies", blit(True), blit(False)], ok("Bool", False))
case("not-true", "not true", ["not", blit(True)], ok("Bool", False))
case("not-not", "double negation", ["not", ["not", blit(True)]], ok("Bool", True))
case("if-then", "the taken branch is the value", ["if", blit(True), lit("U8", 1), lit("U8", 2)], ok("U8", "1"))
case("if-else", "the else branch", ["if", blit(False), lit("U8", 1), lit("U8", 2)], ok("U8", "2"))
case("if-untaken-branch-not-evaluated", "an overflowing node in the untaken branch is never evaluated",
     ["if", blit(True), lit("Int", 1), ["mul", lit("Nat", NAT_MAX), lit("Nat", NAT_MAX)]], ok("Int", "1"))
case("and-untaken-operand-not-evaluated", "the right operand of a false and is not evaluated",
     ["and", blit(False), ["eq", ["mul", lit("Nat", NAT_MAX), lit("Nat", NAT_MAX)], lit("Int", 0)]], ok("Bool", False))

# concat take drop (vars of type Bytes: the result type is plain Bytes)
case("concat-two", "concat joins two Bytes", ["concat", var("a"), var("b")], ok("Bytes", "0102"),
     env=[e("a", "Bytes", "01"), e("b", "Bytes", "02")])
case("concat-empty-left", "concat with the empty Bytes", ["concat", var("a"), var("b")], ok("Bytes", "02"),
     env=[e("a", "Bytes", ""), e("b", "Bytes", "02")])
case("take-prefix", "take the first two bytes", ["take", var("b"), lit("Nat", 2)], ok("Bytes", "0a0b"),
     env=[e("b", "Bytes", "0a0b0c")])
case("take-zero", "take zero bytes", ["take", var("b"), lit("Nat", 0)], ok("Bytes", ""), env=[e("b", "Bytes", "0a0b0c")])
case("take-clamps-past-the-end", "take clamps to the length", ["take", var("b"), lit("Nat", 10)], ok("Bytes", "0a0b0c"),
     env=[e("b", "Bytes", "0a0b0c")])
case("take-negative-count-is-zero", "a negative count is zero (a slice from the end would be wrong)",
     ["take", var("b"), lit("Int", -1)], ok("Bytes", ""), env=[e("b", "Bytes", "0a0b0c")])
case("take-count-from-sub", "a count computed as Nat - Nat may be negative", ["take", var("b"), ["sub", lit("Nat", 0), lit("Nat", 1)]],
     ok("Bytes", ""), env=[e("b", "Bytes", "0a0b0c")])
case("drop-prefix", "drop the first byte", ["drop", var("b"), lit("Nat", 1)], ok("Bytes", "0b0c"),
     env=[e("b", "Bytes", "0a0b0c")])
case("drop-clamps-past-the-end", "drop clamps to the length", ["drop", var("b"), lit("Nat", 10)], ok("Bytes", ""),
     env=[e("b", "Bytes", "0a0b0c")])
case("drop-negative-count-is-zero", "a negative count drops nothing", ["drop", var("b"), lit("Int", -2)],
     ok("Bytes", "0a0b0c"), env=[e("b", "Bytes", "0a0b0c")])
case("take-last-byte-form", "the truncate-last shape: take(b, len(b) - 1)",
     ["take", var("b"), ["sub", ["len", var("b")], lit("Nat", 1)]], ok("Bytes", "0a0b"), env=[e("b", "Bytes", "0a0b0c")])

# neg
case("neg-i8-min", "neg wraps the signed minimum to itself", ["neg", var("a")], ok("I8", "-128"), env=[e("a", "I8", "-128")])
case("neg-i8-positive", "neg of 5", ["neg", var("a")], ok("I8", "-5"), env=[e("a", "I8", "5")])
case("neg-u8-wraps", "neg of an unsigned value wraps", ["neg", lit("U8", 1)], ok("U8", "255"))
case("neg-nat-is-int", "neg of a Nat is an Int", ["neg", lit("Nat", 5)], ok("Int", "-5"))

# ===============================================================================================
# 2. Short-circuit and the pure call trace (section 4.2 rule 2, section 4.4)
# ===============================================================================================
S = sub_op("bump", var("a"))
S6 = ["eq", S, lit("U8", 6)]  # true for a = 5 (bump(5) = 6)
A5 = ["eq", var("a"), lit("U8", 5)]  # a pure guard that is true
A0 = ["eq", var("a"), lit("U8", 0)]  # a pure guard that is false
SC = dict(env=[e("a", "U8", "5")], models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
case("short-circuit-and-untaken", "and: the untaken subject call is not counted", ["and", A0, S6], ok("Bool", False, 0), **SC)
case("short-circuit-and-taken", "and: the taken subject call is counted", ["and", A5, S6], ok("Bool", True, 1), **SC)
case("short-circuit-or-untaken", "or: a true left operand skips the subject call", ["or", A5, S6], ok("Bool", True, 0), **SC)
case("short-circuit-or-taken", "or: a false left operand runs the subject call", ["or", A0, S6], ok("Bool", True, 1), **SC)
case("short-circuit-implies-untaken", "implies: a false antecedent skips the subject call", ["implies", A0, S6],
     ok("Bool", True, 0), **SC)
case("short-circuit-implies-taken", "implies: a true antecedent runs the subject call", ["implies", A5, S6],
     ok("Bool", True, 1), **SC)
case("short-circuit-if-then-taken", "if: the taken then branch is counted", ["if", A5, S6, blit(False)], ok("Bool", True, 1), **SC)
case("short-circuit-if-else-taken", "if: the taken else branch is counted", ["if", A0, blit(False), S6], ok("Bool", True, 1), **SC)
case("short-circuit-if-then-untaken", "if: the untaken then branch is not counted", ["if", A0, S6, blit(True)],
     ok("Bool", True, 0), **SC)
case("short-circuit-if-else-untaken", "if: the untaken else branch is not counted", ["if", A5, blit(True), S6],
     ok("Bool", True, 0), **SC)
case("short-circuit-two-calls-one-branch", "only the taken branch's calls are counted",
     ["if", A5, ["eq", sub_op("add2", S, var("a")), lit("U8", 11)], ["eq", sub_op("add2", S, S), lit("U8", 0)]],
     ok("Bool", True, 2), env=[e("a", "U8", "5")], models={"inc": INC, "addm": ADDM},
     ops={"bump": BUMP, "add2": ADD2}, subjects={"bump": "inc", "add2": "addm"})
case("short-circuit-call-on-the-left-guards-nothing", "an impure left operand is fine when it guards no call",
     ["and", S6, A5], ok("Bool", True, 1), **SC)
case("short-circuit-subject-free-and", "no subject call, no count", ["and", A5, A0], ok("Bool", False, 0), **SC)

# ===============================================================================================
# 3. impure-guard: a subject call under a guard that itself contains a subject call (section 4.4)
# ===============================================================================================
case("impure-guard-if", "the condition of an if contains a subject call and a branch calls the subject",
     ["if", S6, S, lit("U8", 0)], err("impure-guard"), **SC)
case("impure-guard-and", "the left operand of and contains a subject call and so does the right",
     ["and", S6, S6], err("impure-guard"), **SC)
case("impure-guard-or", "the same for or", ["or", S6, S6], err("impure-guard"), **SC)
case("impure-guard-implies", "the same for implies", ["implies", S6, S6], err("impure-guard"), **SC)
case("impure-guard-nested-in-then", "an impure guard inside the taken branch of a pure guard",
     ["if", A5, ["and", S6, S6], blit(True)], err("impure-guard"), **SC)

# ===============================================================================================
# 4. unknown-tag and level-missing
# ===============================================================================================
case("unknown-tag-frobnicate", "a tag of no level this document defines", ["frobnicate", var("a")],
     err("unknown-tag"), env=[e("a", "U8", "1")])
case("unknown-tag-nested", "an unknown tag below a known one", ["not", ["wibble", blit(True)]], err("unknown-tag"))
case("unknown-tag-uppercase", "tags are case-sensitive: ADD is not add", ["ADD", lit("U8", 1), lit("U8", 2)],
     err("unknown-tag"))
case("unknown-tag-in-model", "an unknown tag in a model body", call("m", var("a")), err("unknown-tag"),
     env=[e("a", "U8", "1")], models={"m": model([("a", "U8")], ["frobnicate", var("a")])})
case("unknown-tag-is-not-skipped", "an unknown operand is never skipped even in an untaken branch",
     ["if", blit(True), lit("U8", 1), ["wibble", lit("U8", 2)]], err("unknown-tag"))
LM = dict(levels=("ir.core",))
case("level-missing-bit-xor", "bit_xor is an ir.fixed tag and levels lacks ir.fixed",
     ["bit_xor", var("a"), var("b")], err("level-missing"), env=[e("a", "U8", "1"), e("b", "U8", "3")])
case("level-missing-shl", "shl is an ir.fixed tag", ["shl", var("a"), lit("Nat", 1)], err("level-missing"),
     env=[e("a", "U8", "1")])
case("level-missing-popcount", "popcount is an ir.fixed tag", ["popcount", var("a")], err("level-missing"),
     env=[e("a", "U8", "1")])
case("level-missing-unwrap-or", "unwrap_or is an ir.fixed tag", ["unwrap_or", ["at", var("b"), lit("Nat", 0)], lit("U8", 0)],
     err("level-missing"), env=[e("b", "Bytes", "00")])
case("level-missing-tuple", "the tuple literal is an ir.tuple tag", ["tuple", [var("a"), var("b")]], err("level-missing"),
     env=[e("a", "U8", "1"), e("b", "U8", "2")])
case("level-missing-project", "project is an ir.tuple tag", ["project", ["tuple", [var("a"), var("b")]], 0],
     err("level-missing"), env=[e("a", "U8", "1"), e("b", "U8", "2")])
case("level-missing-in-model", "a level-missing tag in a model body", call("m", var("a")), err("level-missing"),
     env=[e("a", "U8", "1")], models={"m": model([("a", "U8")], ["rotl", var("a"), lit("Nat", 1)])})
case("level-missing-in-untaken-branch", "a tag of a missing level is refused even where it would not run",
     ["if", blit(True), lit("U8", 1), ["bit_not", lit("U8", 2)]], err("level-missing"))

# ===============================================================================================
# 5. model-order
# ===============================================================================================
A_TO_B = {"b": model([("a", "U8")], var("a")), "a": model([("a", "U8")], call("b", var("a")))}
case("model-order-forward-reference", "model a calls model b, which is declared after it",
     call("a", var("x")), err("model-order"), env=[e("x", "U8", "1")], models=A_TO_B, order=["a", "b"])
case("model-order-declared-before-is-fine", "the same models with b declared first",
     call("a", var("x")), ok("U8", "1"), env=[e("x", "U8", "1")], models=A_TO_B, order=["b", "a"])
case("model-order-self-call", "a model that calls itself", call("a", var("x")), err("model-order"),
     env=[e("x", "U8", "1")], models={"a": model([("a", "U8")], call("a", var("a")))})
case("model-order-missing-model", "a call to a model that does not exist", call("ghost", var("x")), err("model-order"),
     env=[e("x", "U8", "1")])
case("model-order-not-in-model-order", "a model in models but not in model_order cannot be called",
     call("a", var("x")), err("model-order"), env=[e("x", "U8", "1")], models={"a": model([("a", "U8")], var("a"))}, order=[])
case("model-order-caller-calls-unlisted", "a model calls one that is missing from model_order",
     call("a", var("x")), err("model-order"), env=[e("x", "U8", "1")], models=A_TO_B, order=["a"])
case("model-order-from-the-array-not-the-keys", "order is the model_order array, never the key order of models",
     call("a", var("x")), err("model-order"), env=[e("x", "U8", "1")],
     models={"a": model([("a", "U8")], var("a")), "b": model([("a", "U8")], call("a", var("a")))}, order=["b", "a"])

# ===============================================================================================
# 6. subject-forbidden
# ===============================================================================================
case("subject-forbidden-in-model", "a subject_op in a model body", call("m", var("a")), err("subject-forbidden"),
     env=[e("a", "U8", "1")], models={"inc": INC, "m": model([("a", "U8")], sub_op("bump", var("a")))},
     ops={"bump": BUMP}, subjects={"bump": "inc"})
case("subject-forbidden-in-unused-model", "a subject_op in a model that nothing calls is still refused",
     blit(True), err("subject-forbidden"), models={"inc": INC, "m": model([("a", "U8")], sub_op("bump", var("a")))},
     ops={"bump": BUMP}, subjects={"bump": "inc"})
case("subject-forbidden-role-admit", "a subject_op in an admit expression", ["eq", S, lit("U8", 6)],
     err("subject-forbidden"), role="admit", **SC)
case("subject-forbidden-role-fault", "a subject_op in a fault expression", ["eq", S, lit("U8", 6)],
     err("subject-forbidden"), role="fault", **SC)
case("role-admit-without-subject", "an admit expression with no subject call is fine", A5, ok("Bool", True, 0),
     role="admit", **SC)
case("role-fault-without-subject", "a fault expression with no subject call is fine", ["add", var("a"), lit("U8", 1)],
     ok("U8", "6", 0), role="fault", **SC)
case("role-body-explicit", "the default role spelled out allows subject calls", S6, ok("Bool", True, 1), role="body", **SC)

# ===============================================================================================
# 7. ill-typed: any other static type error
# ===============================================================================================
U = [e("a", "U8", "1"), e("c", "U16", "2"), e("b", "Bytes", "00")]
case("ill-typed-mixed-widths", "add of U8 and U16", ["add", var("a"), var("c")], err("ill-typed"), env=U)
case("ill-typed-condition-not-bool", "an if condition that is a U8", ["if", var("a"), lit("U8", 1), lit("U8", 2)],
     err("ill-typed"), env=U)
case("ill-typed-branches-differ", "if branches of different types", ["if", blit(True), lit("U8", 1), blit(False)],
     err("ill-typed"))
case("ill-typed-unbound-var", "a variable that is not in env", var("nope"), err("ill-typed"), env=U)
case("ill-typed-bool-is-not-u8", "Bool is not U8", ["eq", blit(True), lit("U8", 1)], err("ill-typed"))
case("ill-typed-u8-vs-nat", "U8 and Nat do not unify", ["eq", var("a"), lit("Nat", 1)], err("ill-typed"), env=U)
case("ill-typed-order-on-bool", "an ordering needs numbers", ["lt", blit(True), blit(False)], err("ill-typed"))
case("ill-typed-order-mixed-widths", "an ordering of a fixed width needs the same width", ["lt", var("a"), var("c")],
     err("ill-typed"), env=U)
case("ill-typed-not-of-u8", "not needs a Bool", ["not", var("a")], err("ill-typed"), env=U)
case("ill-typed-and-of-u8", "and needs Bool operands", ["and", blit(True), var("a")], err("ill-typed"), env=U)
case("ill-typed-len-of-u8", "len needs Bytes", ["len", var("a")], err("ill-typed"), env=U)
case("ill-typed-neg-of-bool", "neg needs a number", ["neg", blit(True)], err("ill-typed"))
case("ill-typed-add-of-bool", "arithmetic needs numbers", ["add", blit(True), blit(True)], err("ill-typed"))
case("ill-typed-concat-of-u8", "concat needs Bytes", ["concat", var("b"), var("a")], err("ill-typed"), env=U)
case("ill-typed-take-count-bool", "take needs a number count", ["take", var("b"), blit(True)], err("ill-typed"), env=U)
case("ill-typed-is-err-of-u8", "is_err needs a Result", ["is_err", var("a")], err("ill-typed"), env=U)
case("ill-typed-result-vs-non-result", "comparing a Result with a non-Result", ["eq", var("r"), var("a")],
     err("ill-typed"), env=[e("a", "U8", "1"), e("r", "Result<U8,E>", {"ok": "1"})], types=ERR)
case("ill-typed-model-call-too-few-args", "a model call with the wrong argument count", call("addm", var("a")),
     err("ill-typed"), env=U, models={"addm": ADDM})
case("ill-typed-model-call-too-many-args", "a model call with too many arguments", call("inc", var("a"), var("a")),
     err("ill-typed"), env=U, models={"inc": INC})
case("ill-typed-model-call-wrong-type", "a model call with an argument of the wrong type", call("inc", var("c")),
     err("ill-typed"), env=U, models={"inc": INC})
case("ill-typed-subject-op-missing-from-ops", "a subject_op naming an op that ops does not declare (R45)",
     sub_op("bump", var("a")), err("ill-typed"), env=U)
case("ill-typed-err-case-undeclared", "an err whose case no error type in types declares (R41)",
     ["is_err", ["err", "NOPE"]], err("ill-typed"), types=ERR)
case("ill-typed-err-without-an-error-type", "an err when types declares no error type at all (R41)",
     ["is_err", ["err", "BAD"]], err("ill-typed"))
case("ill-typed-subject-wrong-arity", "a subject call with the wrong argument count", sub_op("bump", var("a"), var("a")),
     err("ill-typed"), env=U, models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
case("ill-typed-subject-wrong-type", "a subject call with an argument of the wrong type", sub_op("bump", var("c")),
     err("ill-typed"), env=U, models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
case("ill-typed-in-model-body", "a type error inside a model body", call("m", var("a")), err("ill-typed"),
     env=U, models={"m": model([("a", "U8")], ["not", var("a")])})

# ===============================================================================================
# 8. depth: nesting past 64, and past 64 active model frames (section 14)
# ===============================================================================================
def nots(n):
    ir = blit(True)
    for _ in range(n):
        ir = ["not", ir]
    return ir


def chain(n):
    """m1(a) = a, mk(a) = m(k-1)(a): calling mn has n active frames."""
    models = {"m1": model([("a", "Bool")], var("a"))}
    for k in range(2, n + 1):
        models[f"m{k}"] = model([("a", "Bool")], call(f"m{k - 1}", var("a")))
    return models


case("depth-expression-64-accepted", "an expression 64 levels deep is allowed (the root is level 1)", nots(63),
     ok("Bool", False))
case("depth-expression-65-refused", "an expression 65 levels deep is refused", nots(64), err("depth"))
case("depth-expression-far-past", "an expression 200 levels deep", nots(199), err("depth"))
case("depth-model-frames-64-accepted", "64 active model_call frames are allowed", call("m64", var("a")), ok("Bool", True),
     env=[e("a", "Bool", True)], models=chain(64), order=[f"m{k}" for k in range(1, 65)])
case("depth-model-frames-65-refused", "65 active model_call frames are refused", call("m65", var("a")), err("depth"),
     env=[e("a", "Bool", True)], models=chain(65), order=[f"m{k}" for k in range(1, 66)])

# ===============================================================================================
# 9. ir-cost: the worst-path cost against max_ir_cost (section 4.2 rule 6)
# ===============================================================================================
def cost_models():
    """c1(a) = a (cost 1) and ck(a) = and(c(k-1)(a), c(k-1)(a)): costs 1, 7, 19, 43, 91, 187, 379, 763, 1531."""
    models = {"c1": model([("a", "Bool")], var("a"))}
    for k in range(2, 10):
        models[f"c{k}"] = model([("a", "Bool")], ["and", call(f"c{k - 1}", var("a")), call(f"c{k - 1}", var("a"))])
    return models


#: the cost of one call ``ck(a)`` is 1 + 1 (the argument) + the body, so 3, 9, 21, 45, 93, 189, 381, 765, 1533
CALL_COST = {k: c for k, c in zip(range(1, 10), (3, 9, 21, 45, 93, 189, 381, 765, 1533))}


def expr_of_cost(target):
    """A Bool expression over ``a`` (true in every case) whose worst-path cost is exactly ``target``.

    The root is ``or(a, X)``, so its value is ``true`` whatever ``X`` is, and its cost is
    ``1 + 1 + cost(X)`` (a short circuit is never credited).  ``X`` joins model calls and ``a`` with
    ``and`` nodes, largest call first, and a final ``not`` supplies the odd unit when one is left.
    """
    want = target - 2
    leaves = [(call(f"c{k}", var("a")), c) for k, c in sorted(CALL_COST.items(), key=lambda kv: -kv[1])]
    leaves.append((var("a"), 1))
    ir, total = None, 0
    for leaf, c in leaves:
        while (c <= want) if ir is None else (total + 1 + c <= want):
            ir, total = (leaf, c) if ir is None else (["and", ir, leaf], total + 1 + c)
    if total == want - 1:
        ir, total = ["not", ir], want
    assert total == want, (target, total)
    return ["or", var("a"), ir]


CM = dict(env=[e("a", "Bool", True)], models=cost_models(), order=[f"c{k}" for k in range(1, 10)])
case("cost-2000-accepted", "an expression of worst-path cost exactly 2,000 is accepted", expr_of_cost(2000),
     ok("Bool", True), **CM)
case("cost-2001-refused", "an expression of cost 2,001 exceeds the default", expr_of_cost(2001), err("ir-cost"), **CM)
case("cost-2001-accepted-under-4096", "the same expression under max_ir_cost 4096", expr_of_cost(2001), ok("Bool", True),
     max_ir_cost=4096, **CM)
case("cost-4096-accepted-at-limit", "an expression of cost exactly 4,096 under max_ir_cost 4096", expr_of_cost(4096),
     ok("Bool", True), max_ir_cost=4096, **CM)
case("cost-4097-refused-under-4096", "cost 4,097 exceeds max_ir_cost 4096", expr_of_cost(4097), err("ir-cost"),
     max_ir_cost=4096, **CM)
case("cost-if-takes-the-worse-branch", "an if costs its condition plus the larger branch: 1 + 1 + 1998 = 2000",
     ["if", var("a"), expr_of_cost(1998), blit(True)], ok("Bool", True), **CM)
case("cost-if-worse-branch-over", "the larger branch decides: 1 + 1 + 1999 = 2001",
     ["if", var("a"), blit(True), expr_of_cost(1999)], err("ir-cost"), **CM)
case("cost-and-is-never-credited", "a short circuit is never credited: and costs both operands",
     ["and", blit(False), expr_of_cost(1999)], err("ir-cost"), **CM)
case("cost-model-body-2000-accepted", "a model body of cost 2,000 is accepted",
     blit(True), ok("Bool", True), env=[], models={**cost_models(), "big": model([("a", "Bool")], expr_of_cost(2000))},
     order=[f"c{k}" for k in range(1, 10)] + ["big"])
case("cost-model-body-2001-refused", "a model body of cost 2,001 is refused even if nothing calls it",
     blit(True), err("ir-cost"), env=[], models={**cost_models(), "big": model([("a", "Bool")], expr_of_cost(2001))},
     order=[f"c{k}" for k in range(1, 10)] + ["big"])
case("cost-model-call-adds-the-body", "a call costs 1 + its arguments + the callee's body: 1 + 1 + 1998 = 2000",
     call("big", var("a")), ok("Bool", True), env=[e("a", "Bool", True)],
     models={**cost_models(), "big": model([("a", "Bool")], expr_of_cost(1998))},
     order=[f"c{k}" for k in range(1, 10)] + ["big"])
case("cost-model-call-over", "a call whose callee's body brings the total to 2,001",
     call("big", var("a")), err("ir-cost"), env=[e("a", "Bool", True)],
     models={**cost_models(), "big": model([("a", "Bool")], expr_of_cost(1999))},
     order=[f"c{k}" for k in range(1, 10)] + ["big"])


def doubling(n):
    """d1(a) = and(a, a); dk(a) = d(k-1)(d(k-1)(a)): the cost doubles every level."""
    models = {"d1": model([("a", "Bool")], ["and", var("a"), var("a")])}
    for k in range(2, n + 1):
        models[f"d{k}"] = model([("a", "Bool")], call(f"d{k - 1}", call(f"d{k - 1}", var("a"))))
    return models


case("cost-doubling-chain-refused", "a model chain that doubles its cost every level is refused",
     call("d12", var("a")), err("ir-cost"), env=[e("a", "Bool", True)], models=doubling(12),
     order=[f"d{k}" for k in range(1, 13)])
case("cost-doubling-chain-short-accepted", "the same chain stopped at nine levels (cost 1,533) is accepted",
     call("d9", var("a")), ok("Bool", True), env=[e("a", "Bool", True)], models=doubling(9),
     order=[f"d{k}" for k in range(1, 10)])
case("max-ir-cost-1999-refused", "max_ir_cost below 2,000 is itself an ir-cost error", blit(True), err("ir-cost"),
     max_ir_cost=1999)
case("max-ir-cost-zero-refused", "max_ir_cost 0", blit(True), err("ir-cost"), max_ir_cost=0)
case("max-ir-cost-65537-refused", "max_ir_cost above the 65,536 ceiling", blit(True), err("ir-cost"), max_ir_cost=65537)
case("max-ir-cost-65536-accepted", "max_ir_cost at the ceiling", blit(True), ok("Bool", True), max_ir_cost=65536)
case("max-ir-cost-2001-accepted", "max_ir_cost just above the default", blit(True), ok("Bool", True), max_ir_cost=2001)

# ===============================================================================================
# 10. ir-overflow and its class (section 3.2, section 4.2 rule 6)
# ===============================================================================================
OVN = ["mul", lit("Nat", NAT_MAX), lit("Nat", NAT_MAX)]
case("overflow-nat-product-corpus-class", "a Nat product leaving 2^127 in a subject-independent node is the corpus's",
     OVN, err("ir-overflow", "corpus"))
case("overflow-int-sum-corpus-class", "an Int sum leaving the window", ["add", EDGE, lit("Int", 1)],
     err("ir-overflow", "corpus"))
case("overflow-in-model-corpus-class", "an overflow inside a pure model is the corpus's too",
     call("sq", lit("Nat", NAT_MAX)), err("ir-overflow", "corpus"),
     models={"sq": model([("a", "Nat")], ["mul", var("a"), var("a")])})
case("overflow-below-subject-case-class", "the same product below a subject call is a case failure",
     ["mul", sub_op("idn", var("n")), lit("Nat", NAT_MAX)], err("ir-overflow", "case"),
     env=[e("n", "Nat", str(NAT_MAX))], models={"keep": model([("a", "Nat")], var("a"))},
     ops={"idn": op([("a", "Nat")], "Nat")}, subjects={"idn": "keep"})
case("overflow-below-subject-in-comparison", "an overflow under a comparison whose operand calls the subject",
     ["eq", ["mul", sub_op("idn", var("n")), lit("Nat", NAT_MAX)], lit("Int", 0)], err("ir-overflow", "case"),
     env=[e("n", "Nat", str(NAT_MAX))], models={"keep": model([("a", "Nat")], var("a"))},
     ops={"idn": op([("a", "Nat")], "Nat")}, subjects={"idn": "keep"})
case("overflow-pure-sibling-of-subject-corpus-class", "a pure overflowing argument next to a subject call is the corpus's",
     ["and", ["eq", OVN, lit("Int", 0)], S6], err("ir-overflow", "corpus"), **SC)
case("overflow-not-reached-below-subject", "a subject result that does not overflow gives a value",
     ["mul", sub_op("idn", var("n")), lit("Nat", 2)], ok("Int", "10", 1), env=[e("n", "Nat", "5")],
     models={"keep": model([("a", "Nat")], var("a"))}, ops={"idn": op([("a", "Nat")], "Nat")}, subjects={"idn": "keep"})


# The edge of the Int window (section 3.2), built from literals of at most 20 digits ([D58]).
case("int-window-edge-by-arithmetic", "2^63 * (2^64-1) + (2^63-1) reaches 2^127-1, the top of the window", EDGE,
     ok("Int", str(INT_MAX)))
case("int-window-edge-negated", "negating the top of the window gives the bottom, -(2^127-1)", NEG_EDGE,
     ok("Int", str(-INT_MAX)))
case("int-edge-plus-zero", "the edge plus zero stays at the edge", ["add", EDGE, lit("Int", 0)], ok("Int", str(INT_MAX)))
case("int-edge-times-minus-one", "the edge times -1 is the other edge", ["mul", EDGE, lit("Int", -1)],
     ok("Int", str(-INT_MAX)))
case("int-edge-times-zero", "the window is on the result, not the operands", ["mul", EDGE, lit("Int", 0)],
     ok("Int", "0"))
case("int-edge-comparison", "-(2^127-1) <= 2^127-1", ["le", NEG_EDGE, EDGE], ok("Bool", True))
case("int-min-minus-one-corpus-class", "-(2^127-1) minus one leaves the window", ["sub", NEG_EDGE, lit("Int", 1)],
     err("ir-overflow", "corpus"))
case("int-product-exactly-2-127", "2^63 * 2^64 is 2^127, one past the window", ["mul", lit("Int", 2**63), lit("Int", 2**64)],
     err("ir-overflow", "corpus"))
case("int-edge-neg-neg", "negating twice returns the edge", ["neg", NEG_EDGE], ok("Int", str(INT_MAX)))


def doubling_concat(n):
    """x1(a) = concat(a, a); xk(a) = x(k-1)(x(k-1)(a)): a length is multiplied by 2^(2^(k-1)) (section 4.2)."""
    models = {"x1": model([("a", "Bytes")], ["concat", var("a"), var("a")])}
    for k in range(2, n + 1):
        models[f"x{k}"] = model([("a", "Bytes")], call(f"x{k - 1}", call(f"x{k - 1}", var("a"))))
    return models


DC = dict(env=[e("a", "Bytes", "ab")], models=doubling_concat(9), order=[f"x{k}" for k in range(1, 10)])
case("overflow-doubling-concat-chain", "nine levels of doubling concat multiply a length by 2^256: refused before allocation",
     call("x9", var("a")), err("ir-overflow", "corpus"), **DC)
case("overflow-concat-exactly-65536", "x5 turns one byte into exactly 65,536 bytes, the MAX_IR_VALUE",
     ["eq", ["len", call("x5", var("a"))], lit("Nat", 65536)], ok("Bool", True), **DC)
case("overflow-concat-65537", "one byte more than MAX_IR_VALUE", ["concat", call("x5", var("a")), var("a")],
     err("ir-overflow", "corpus"), **DC)
case("overflow-concat-65537-below-subject", "a concat past the limit below a subject call is a case failure",
     ["eq", ["len", ["concat", call("x5", var("a")), sub_op("same", var("a"))]], lit("Nat", 0)],
     err("ir-overflow", "case"), env=[e("a", "Bytes", "ab")],
     models={**doubling_concat(5), "keepb": model([("a", "Bytes")], var("a"))},
     order=[f"x{k}" for k in range(1, 6)] + ["keepb"], ops={"same": op([("a", "Bytes")], "Bytes")},
     subjects={"same": "keepb"})


# ===============================================================================================
# 11. S5e (VER04r section 16.18): static widths, the overflow class under a guard, bytes literals, a count,
#     level-missing by reach and a chain nothing calls
# ===============================================================================================

# -- R70: an op with width_params ------------------------------------------------------------------------------
def wop(params, ret, widths=("N",)):
    return {"params": [list(p) for p in params], "ret": ret, "requires": CORE, "width_params": list(widths)}


EQF = wop([("a", "Bytes<N>"), ("b", "Bytes<N>")], "Bool")  # eq_fixed<N>(a: Bytes<N>, b: Bytes<N>) -> Bool
# The stand-in has the op's signature with plain Bytes in place of each named-width type (R70).
SAME = model([("a", "Bytes"), ("b", "Bytes")], ["eq", var("a"), var("b")])
EQ_ARGS = dict(env=[e("a", "Bytes<2>", "0a0b"), e("b", "Bytes<2>", "0a0b")], models={"same": SAME},
               ops={"eq_fixed": EQF}, subjects={"eq_fixed": "same"})


def eq_fixed(*tail):
    return ["subject_op", "eq_fixed", [var("a"), var("b")], *tail]


case("width-params-one-width", "a subject_op over an op with width_params carries one static width, [2]", eq_fixed([2]),
     ok("Bool", True, 1), **EQ_ARGS)
case("width-params-width-mismatch", "a static width of 3 does not accept Bytes<2> arguments", eq_fixed([3]),
     err("ill-typed"), **EQ_ARGS)
case("width-params-width-zero", "the width 0 is a width: Bytes<0> arguments", eq_fixed([0]), ok("Bool", True, 1),
     env=[e("a", "Bytes<0>", ""), e("b", "Bytes<0>", "")], models={"same": SAME}, ops={"eq_fixed": EQF},
     subjects={"eq_fixed": "same"})
case("width-params-missing-width", "an op with width_params and a subject_op with no fourth element", eq_fixed(),
     err("ill-typed"), **EQ_ARGS)
case("width-params-extra-width", "two widths for one width_param", eq_fixed([2, 2]), err("ill-typed"), **EQ_ARGS)
case("width-params-width-over-4096", "the width 4,097", eq_fixed([4097]), err("ill-typed"), **EQ_ARGS)
case("width-params-empty-widths-for-an-op-with-one", "an empty widths list for an op that has one width_param",
     eq_fixed([]), err("ill-typed"), **EQ_ARGS)
case("width-params-widths-on-an-op-without", "a widths list on an op without width_params", ["subject_op", "bump", [var("a")], [2]],
     err("ill-typed"), **SC)
case("width-params-empty-widths-on-an-op-without", "an empty widths list on an op without width_params (the generator "
     "never writes one)", ["subject_op", "bump", [var("a")], []], err("ill-typed"), **SC)

# -- R71: the class of an overflow under an impure guard -----------------------------------------------------------
OVI = ["eq", OVN, lit("Int", 0)]  # a pure Bool node that overflows whenever it runs
IF_GUARD = ["if", S6, OVI, blit(False)]  # the branch is subject-independent: the generator's trace never reaches it
AND_GUARD = ["and", S6, OVI]
case("overflow-if-impure-guard-runs-case-class", "if subject(..) then <pure overflow> else false: the stand-in makes "
     "the guarded node run, and the overflow is the case's", IF_GUARD, err("ir-overflow", "case"), **SC)
case("overflow-if-impure-guard-skips-the-node", "the same tree with a stand-in whose answer takes the else branch: a value "
     "and one call", IF_GUARD, ok("Bool", False, 1), env=[e("a", "U8", "0")], models={"inc": INC}, ops={"bump": BUMP},
     subjects={"bump": "inc"})
case("overflow-and-impure-guard-runs-case-class", "subject(..) and <pure overflow>: the right operand runs and the "
     "overflow is the case's", AND_GUARD, err("ir-overflow", "case"), **SC)
case("overflow-and-impure-guard-skips-the-node", "the same and with a false left operand: the overflow does not run",
     AND_GUARD, ok("Bool", False, 1), env=[e("a", "U8", "0")], models={"inc": INC}, ops={"bump": BUMP},
     subjects={"bump": "inc"})
case("overflow-if-pure-guard-corpus-class", "the twin with a pure guard: the generator evaluates the taken branch, so "
     "the overflow is the corpus's", ["if", A5, OVI, ["if", S6, blit(True), blit(False)]],
     err("ir-overflow", "corpus"), **SC)
BOOM = model([("a", "U8")], OVI)  # overflows whatever its argument is
case("overflow-in-model-under-a-subject-argument-case-class", "a model body that overflows, called with a subject call "
     "as an argument: the walk never evaluates the body, so the overflow is the case's", call("boom", S),
     err("ir-overflow", "case"), env=[e("a", "U8", "5")], models={"inc": INC, "boom": BOOM}, ops={"bump": BUMP},
     subjects={"bump": "inc"})
case("overflow-in-model-with-a-pure-argument-corpus-class", "the same model called with a pure argument is the corpus's",
     call("boom", var("a")), err("ir-overflow", "corpus"), env=[e("a", "U8", "5")], models={"boom": BOOM})

# -- R73: a bytes literal --------------------------------------------------------------------------------------------
BYTES2 = model([("a", "Bytes<2>")], ["eq", ["len", var("a")], lit("Nat", 2)])
case("lit-bytes-into-a-fixed-width-parameter", "a literal of two bytes passed to a Bytes<2> parameter has the static "
     "type Bytes<2>", call("two", lit("Bytes", "0a0b")), ok("Bool", True), models={"two": BYTES2})
case("lit-bytes-too-short-for-a-fixed-width-parameter", "a literal of three bytes does not flow to Bytes<2>",
     call("two", lit("Bytes", "0a0b0c")), err("ill-typed"), models={"two": BYTES2})
case("lit-written-type-bytes-n", "a lit whose written type is Bytes<2> (the written type is always Bytes)",
     ["lit", {"type": "Bytes<2>", "value": "0a0b"}], err("ill-typed"))
case("lit-object-form-bool", "a Bool literal is the bare form: the object form with type Bool",
     ["lit", {"type": "Bool", "value": True}], err("ill-typed"))

# -- R74: the count of take and drop ----------------------------------------------------------------------------------
case("ill-typed-take-count-fixed-width", "a fixed-width count: take(b, 1u8)", ["take", var("b"), lit("U8", 1)],
     err("ill-typed"), env=U)
case("ill-typed-drop-count-fixed-width", "a fixed-width count: drop(b, 1u8)", ["drop", var("b"), lit("U8", 1)],
     err("ill-typed"), env=U)

# -- R69: level-missing by what ir reaches ------------------------------------------------------------------------------
case("level-missing-env-tuple-type", "an env entry of a tuple type at levels ir.core: the type is ir.tuple, the tag is not "
     "needed", blit(True), err("level-missing"), env=[e("t", "(U8,U8)", ["1", "2"])])
case("level-missing-env-list-type", "an env entry of type List<U8> at levels ir.core: the type is the reserved ir.list "
     "(D63)", blit(True), err("level-missing"), env=[e("l", "List<U8>", ["1"])])
case("level-missing-model-not-reached-is-a-value", "a model with an ir.fixed tag that ir does not call causes no "
     "level-missing, and the case's requires is ir.core", blit(True), ok("Bool", True),
     models={"fx": model([("a", "U8")], ["bit_not", var("a")])})

# -- R77: the chain depth covers every model, called or not --------------------------------------------------------------
case("depth-model-chain-65-uncalled", "a chain of 65 models that nothing calls: the depth of m65 is 65", blit(True),
     err("depth"), models=chain(65), order=[f"m{k}" for k in range(1, 66)])
case("depth-model-chain-64-uncalled-is-a-value", "a chain of 64 models that nothing calls is accepted", blit(True),
     ok("Bool", True), models=chain(64), order=[f"m{k}" for k in range(1, 65)])


# ===============================================================================================
# S10d (VER04r section 13, R9, R76, R100, R111, R120): the vectors at ``ir.fixed`` and ``ir.tuple``
# ===============================================================================================
# Every row below states the ``requires`` it expects, and the build fails if the oracle computes another: a case whose
# ``expect`` is not ``level-missing`` has the levels of what its tree reaches (R9, [D64]), and one whose ``expect`` is
# ``level-missing`` has its ``levels`` (R24).  A lane that implements ``ir.core`` alone skips every one that names
# another level (section 11); the few that name ``ir.core`` alone say why in their summary.
FX = ["ir.core", "ir.fixed"]
TU = ["ir.core", "ir.tuple"]
FT = ["ir.core", "ir.fixed", "ir.tuple"]
MAX64 = str(NAT_MAX)


def fx(slug, summary, ir, intent, **kw):
    """A case at ``levels`` ``ir.core`` and ``ir.fixed``, whose ``requires`` is the same."""
    kw.setdefault("levels", FX)
    kw.setdefault("requires", FX)
    case(slug, summary, ir, intent, **kw)


def tu(slug, summary, ir, intent, **kw):
    """A case at ``levels`` ``ir.core`` and ``ir.tuple``, whose ``requires`` is the same."""
    kw.setdefault("levels", TU)
    kw.setdefault("requires", TU)
    case(slug, summary, ir, intent, **kw)


def unwrap(option, default):
    return ["unwrap_or", option, default]


def u8(v):
    return lit("U8", v)


def i8(v):
    return lit("I8", v)


def nat(v):
    return lit("Nat", v)


# -- bit_and bit_or bit_xor bit_not: per bit of the w-bit pattern, re-read in the type ---------------------------------
fx("bit-and-u8", "240 & 60 on U8 is 48", ["bit_and", u8(240), u8(60)], ok("U8", "48"))
fx("bit-or-u8", "240 | 60 on U8 is 252", ["bit_or", u8(240), u8(60)], ok("U8", "252"))
fx("bit-xor-u8", "240 ^ 60 on U8 is 204", ["bit_xor", u8(240), u8(60)], ok("U8", "204"))
fx("bit-and-i8-negative", "bit_and(-1, 127) on I8: the pattern ff & 7f is 127", ["bit_and", i8(-1), i8(127)], ok("I8", "127"))
fx("bit-or-i8-negative", "bit_or(-128, 1) on I8: the pattern 80 | 01 re-read in I8 is -127", ["bit_or", i8(-128), i8(1)],
   ok("I8", "-127"))
fx("bit-xor-i8-negative", "bit_xor(-128, 127) on I8: the pattern 80 ^ 7f is ff, which is -1", ["bit_xor", i8(-128), i8(127)],
   ok("I8", "-1"))
fx("bit-and-u64-clears-the-high-bit", "the shape of the ignore-high-bit fault: MAX & (2^63-1) on U64",
   ["bit_and", var("a"), lit("U64", 2**63 - 1)], ok("U64", str(2**63 - 1)), env=[e("a", "U64", MAX64)])
fx("bit-and-i64-minimum", "the I64 minimum & -1 is the minimum", ["bit_and", lit("I64", lo("I64")), lit("I64", -1)],
   ok("I64", str(lo("I64"))))
fx("bit-not-u8-zero", "bit_not(0) on U8 is 255", ["bit_not", u8(0)], ok("U8", "255"))
fx("bit-not-i8-zero", "bit_not(0) on I8 is -1 (section A.1)", ["bit_not", i8(0)], ok("I8", "-1"))
fx("bit-not-i8-minimum", "bit_not of the I8 minimum is the maximum", ["bit_not", i8(-128)], ok("I8", "127"))
fx("bit-not-u64-maximum", "bit_not of the U64 maximum is 0", ["bit_not", var("a")], ok("U64", "0"), env=[e("a", "U64", MAX64)])

# -- shl shr sar rotl rotr: the count is an exact Nat compared with w before any host shift -------------------------------
fx("shl-u8-top-bit", "shl(1, 7) on U8 is 128", ["shl", u8(1), nat(7)], ok("U8", "128"))
fx("shl-u8-count-equal-to-width", "shl(1, 8) on U8 is 0: n >= w gives 0", ["shl", u8(1), nat(8)], ok("U8", "0"))
fx("shl-u8-drops-the-top-bit", "shl(129, 1) on U8 is 2: the bit that leaves is lost", ["shl", u8(129), nat(1)], ok("U8", "2"))
fx("shl-u8-count-zero", "shl(255, 0) is 255", ["shl", u8(255), nat(0)], ok("U8", "255"))
fx("shl-i8-into-the-sign-bit", "shl(1, 7) on I8 is -128", ["shl", i8(1), nat(7)], ok("I8", "-128"))
fx("shl-u64-top-bit", "shl(1, 63) on U64 is 2^63", ["shl", lit("U64", 1), nat(63)], ok("U64", str(2**63)))
fx("shl-count-is-the-largest-nat", "a count of 2^64-1 is compared with w first: the result is 0",
   ["shl", u8(255), var("n")], ok("U8", "0"), env=[e("n", "Nat", MAX64)])
fx("shr-i8-is-logical", "shr(-1, 1) on I8 is 127: logical on the pattern, then re-read", ["shr", i8(-1), nat(1)], ok("I8", "127"))
fx("shr-u8-top-bit", "shr(255, 7) on U8 is 1", ["shr", u8(255), nat(7)], ok("U8", "1"))
fx("shr-u8-count-equal-to-width", "shr(255, 8) on U8 is 0", ["shr", u8(255), nat(8)], ok("U8", "0"))
fx("shr-i8-minimum-by-seven", "shr(-128, 7) on I8: the pattern 80 shifted right by 7 is 1", ["shr", i8(-128), nat(7)], ok("I8", "1"))
fx("shr-count-is-the-largest-nat", "shr by 2^64-1 is 0", ["shr", u8(255), var("n")], ok("U8", "0"), env=[e("n", "Nat", MAX64)])
fx("sar-i8-minimum-by-seven", "sar(-128, 7) on I8 is -1: sign fill", ["sar", i8(-128), nat(7)], ok("I8", "-1"))
fx("sar-i8-minimum-by-one", "sar(-128, 1) on I8 is -64", ["sar", i8(-128), nat(1)], ok("I8", "-64"))
fx("sar-i8-positive-past-the-width", "sar(64, 100) on I8 is 0: n >= w gives 0 for a non-negative value", ["sar", i8(64), nat(100)],
   ok("I8", "0"))
fx("sar-i8-negative-past-the-width", "sar(-1, 2^64-1) on I8 is -1: n >= w gives -1 for a negative value",
   ["sar", i8(-1), var("n")], ok("I8", "-1"), env=[e("n", "Nat", MAX64)])
fx("sar-i8-positive-by-six", "sar(127, 6) on I8 is 1", ["sar", i8(127), nat(6)], ok("I8", "1"))
fx("rotl-u8-wraps-the-top-bit", "rotl(129, 1) on U8: 81 becomes 03", ["rotl", u8(129), nat(1)], ok("U8", "3"))
fx("rotl-u8-count-reduced-mod-w", "rotl(1, 9) on U8 rotates by 9 mod 8 = 1", ["rotl", u8(1), nat(9)], ok("U8", "2"))
fx("rotl-u8-count-equal-to-width", "rotl(1, 8) on U8 is a full turn", ["rotl", u8(1), nat(8)], ok("U8", "1"))
fx("rotl-i8-minimum", "rotl(-128, 1) on I8: the pattern 80 becomes 01", ["rotl", i8(-128), nat(1)], ok("I8", "1"))
fx("rotl-count-is-the-largest-nat", "rotl by 2^64-1 reduces n mod w first: 2^64-1 mod 8 is 7, so 1 becomes 128",
   ["rotl", u8(1), var("n")], ok("U8", "128"), env=[e("n", "Nat", MAX64)])
fx("rotr-u8-low-bit-to-the-top", "rotr(1, 1) on U8 is 128", ["rotr", u8(1), nat(1)], ok("U8", "128"))
fx("rotr-u8-by-four", "rotr(129, 4) on U8: 81 becomes 18", ["rotr", u8(129), nat(4)], ok("U8", "24"))
fx("rotr-count-is-the-largest-nat", "rotr by 2^64-1 reduces n mod w first: it rotates by 7, so 1 becomes 2",
   ["rotr", u8(1), var("n")], ok("U8", "2"), env=[e("n", "Nat", MAX64)])

# -- popcount clz ctz bit: on the w-bit pattern, ``Nat`` and ``Bool`` results ---------------------------------------------
fx("popcount-u8-all-ones", "popcount(255) on U8 is 8", ["popcount", u8(255)], ok("Nat", "8"))
fx("popcount-i8-minus-one", "popcount(-1) on I8 counts the pattern ff", ["popcount", i8(-1)], ok("Nat", "8"))
fx("popcount-u8-zero", "popcount(0) is 0", ["popcount", u8(0)], ok("Nat", "0"))
fx("popcount-u64-maximum", "popcount of the U64 maximum is 64", ["popcount", var("a")], ok("Nat", "64"), env=[e("a", "U64", MAX64)])
fx("clz-u8-zero", "clz(0) on U8 is w, 8", ["clz", u8(0)], ok("Nat", "8"))
fx("clz-u8-one", "clz(1) on U8 is 7", ["clz", u8(1)], ok("Nat", "7"))
fx("clz-i8-minus-one", "clz(-1) on I8: the pattern ff has no leading zero", ["clz", i8(-1)], ok("Nat", "0"))
fx("clz-u8-top-bit", "clz(128) on U8 is 0", ["clz", u8(128)], ok("Nat", "0"))
fx("ctz-u8-zero", "ctz(0) on U8 is w, 8", ["ctz", u8(0)], ok("Nat", "8"))
fx("ctz-u8-eight", "ctz(8) on U8 is 3", ["ctz", u8(8)], ok("Nat", "3"))
fx("ctz-i8-minimum", "ctz(-128) on I8: the pattern 80 has seven trailing zeros", ["ctz", i8(-128)], ok("Nat", "7"))
fx("ctz-u64-top-bit", "ctz(2^63) on U64 is 63", ["ctz", lit("U64", 2**63)], ok("Nat", "63"))
fx("bit-u8-set", "bit(5, 0) on U8: bit 0 of 0b101 is set", ["bit", u8(5), nat(0)], ok("Bool", True))
fx("bit-u8-clear", "bit(5, 1) on U8 is false", ["bit", u8(5), nat(1)], ok("Bool", False))
fx("bit-u8-index-past-the-width", "bit(5, 8) on U8 is false for i >= w", ["bit", u8(5), nat(8)], ok("Bool", False))
fx("bit-i8-sign-bit", "bit(-1, 7) on I8 is true", ["bit", i8(-1), nat(7)], ok("Bool", True))
fx("bit-index-is-the-largest-nat", "bit(5, 2^64-1) is false", ["bit", u8(5), var("n")], ok("Bool", False), env=[e("n", "Nat", MAX64)])

# -- widen wrap try to_int to_nat max_value min_value -----------------------------------------------------------------------
fx("widen-u8-to-u32", "widen keeps the value: 255 as U32", ["widen", "U32", u8(255)], ok("U32", "255"))
fx("widen-i8-to-i16", "widen of -128 from I8 to I16", ["widen", "I16", i8(-128)], ok("I16", "-128"))
fx("widen-u8-to-i16", "the range of U8 is inside I16, so 255 widens to I16", ["widen", "I16", u8(255)], ok("I16", "255"))
fx("wrap-nat-into-i8", "wrap I8 of the Nat 200: 200 mod 256 re-read in I8 is -56", ["wrap", "I8", nat(200)], ok("I8", "-56"))
fx("wrap-int-into-u8", "wrap U8 of the Int -1 is 255", ["wrap", "U8", lit("Int", -1)], ok("U8", "255"))
fx("wrap-nat-256-into-u8", "wrap U8 of the Nat 256 is 0", ["wrap", "U8", nat(256)], ok("U8", "0"))
fx("wrap-u8-into-i8", "wrap I8 of the U8 200 is -56", ["wrap", "I8", u8(200)], ok("I8", "-56"))
fx("wrap-i8-into-u16", "wrap U16 of the I8 -1 is 65535", ["wrap", "U16", i8(-1)], ok("U16", "65535"))
fx("try-nat-out-of-range", "try U8 of 300 is None: unwrap_or gives the default", unwrap(["try", "U8", nat(300)], u8(7)), ok("U8", "7"))
fx("try-nat-in-range", "try U8 of 255 is Some(255)", unwrap(["try", "U8", nat(255)], u8(7)), ok("U8", "255"))
fx("try-int-below-range", "try I8 of -129 is None", unwrap(["try", "I8", lit("Int", -129)], i8(5)), ok("I8", "5"))
fx("try-int-at-the-minimum", "try I8 of -128 is Some(-128)", unwrap(["try", "I8", lit("Int", -128)], i8(5)), ok("I8", "-128"))
fx("to-int-i8", "to_int of an I8 is the mathematical value", ["to_int", i8(-5)], ok("Int", "-5"))
fx("to-int-u64-maximum", "to_int of the U64 maximum", ["to_int", var("a")], ok("Int", MAX64), env=[e("a", "U64", MAX64)])
fx("to-nat-u8", "to_nat of a U8", ["to_nat", u8(200)], ok("Nat", "200"))
fx("to-nat-u64-maximum", "to_nat of the U64 maximum is a Nat", ["to_nat", var("a")], ok("Nat", MAX64), env=[e("a", "U64", MAX64)])
fx("max-value-i64", "max_value I64 is not folded to a lit (section A.1), and its value is 2^63-1", ["max_value", "I64"],
   ok("I64", str(2**63 - 1)))
fx("min-value-i8", "min_value I8", ["min_value", "I8"], ok("I8", "-128"))
fx("max-value-u64", "max_value U64", ["max_value", "U64"], ok("U64", MAX64))
fx("min-value-u8", "min_value U8 is 0", ["min_value", "U8"], ok("U8", "0"))

# -- checked_add checked_sub checked_mul: ``None`` iff the exact result leaves the type (read through unwrap_or) -----------------
fx("checked-add-u8-overflows", "checked_add(200, 100) on U8 is None", unwrap(["checked_add", u8(200), u8(100)], u8(9)), ok("U8", "9"))
fx("checked-add-u8-in-range", "checked_add(1, 2) on U8 is Some(3)", unwrap(["checked_add", u8(1), u8(2)], u8(9)), ok("U8", "3"))
fx("checked-add-i8-past-the-maximum", "checked_add(127, 1) on I8 is None", unwrap(["checked_add", i8(127), i8(1)], i8(9)), ok("I8", "9"))
fx("checked-add-i8-below-the-minimum", "checked_add(-128, -1) on I8 is None", unwrap(["checked_add", i8(-128), i8(-1)], i8(9)),
   ok("I8", "9"))
fx("checked-sub-u8-below-zero", "checked_sub(1, 2) on U8 is None", unwrap(["checked_sub", u8(1), u8(2)], u8(9)), ok("U8", "9"))
fx("checked-sub-u8-in-range", "checked_sub(5, 2) on U8 is Some(3)", unwrap(["checked_sub", u8(5), u8(2)], u8(9)), ok("U8", "3"))
fx("checked-mul-u8-overflows", "checked_mul(16, 16) on U8 is None", unwrap(["checked_mul", u8(16), u8(16)], u8(9)), ok("U8", "9"))
fx("checked-mul-u8-at-the-maximum", "checked_mul(15, 17) on U8 is Some(255)", unwrap(["checked_mul", u8(15), u8(17)], u8(9)),
   ok("U8", "255"))
fx("checked-mul-i8-minimum-by-minus-one", "checked_mul(-128, -1) on I8 is None", unwrap(["checked_mul", i8(-128), i8(-1)], i8(9)),
   ok("I8", "9"))
fx("checked-mul-u64-maximum-squared", "checked_mul(MAX, MAX) on U64 is None, not an ir-overflow",
   unwrap(["checked_mul", var("a"), var("a")], lit("U64", 5)), ok("U64", "5"), env=[e("a", "U64", MAX64)])

# -- div rem (unsigned and Int/Nat, floor) and div_trunc rem_trunc (signed, toward zero) --------------------------------------
fx("div-u8", "div(7, 2) on U8 is Some(3)", unwrap(["div", u8(7), u8(2)], u8(99)), ok("U8", "3"))
fx("div-u8-by-zero", "div(7, 0) on U8 is None", unwrap(["div", u8(7), u8(0)], u8(99)), ok("U8", "99"))
fx("rem-u8", "rem(7, 2) on U8 is Some(1)", unwrap(["rem", u8(7), u8(2)], u8(99)), ok("U8", "1"))
fx("rem-u8-by-zero", "rem(7, 0) on U8 is None", unwrap(["rem", u8(7), u8(0)], u8(99)), ok("U8", "99"))
fx("div-int-floors", "div(-7, 2) on Int is -4: floor", unwrap(["div", lit("Int", -7), lit("Int", 2)], lit("Int", 0)), ok("Int", "-4"))
fx("rem-int-has-the-divisors-sign-positive", "rem(-7, 2) on Int is 1", unwrap(["rem", lit("Int", -7), lit("Int", 2)], lit("Int", 0)),
   ok("Int", "1"))
fx("rem-int-has-the-divisors-sign-negative", "rem(7, -2) on Int is -1: -2 * -4 + -1", unwrap(["rem", lit("Int", 7), lit("Int", -2)],
   lit("Int", 0)), ok("Int", "-1"))
fx("div-int-negative-divisor", "div(7, -2) on Int is -4: floor", unwrap(["div", lit("Int", 7), lit("Int", -2)], lit("Int", 0)),
   ok("Int", "-4"))
fx("rem-int-both-negative", "rem(-7, -2) on Int is -1: -2 * 3 + -1", unwrap(["rem", lit("Int", -7), lit("Int", -2)], lit("Int", 0)),
   ok("Int", "-1"))
fx("div-int-by-zero", "div(5, 0) on Int is None", unwrap(["div", lit("Int", 5), lit("Int", 0)], lit("Int", 42)), ok("Int", "42"))
fx("div-two-nats-give-an-int", "div on two Nat operands has the type Option<Int>", unwrap(["div", nat(7), nat(2)], lit("Int", 0)),
   ok("Int", "3"))
fx("rem-two-nats-give-an-int", "rem on two Nat operands has the type Option<Int>", unwrap(["rem", nat(7), nat(2)], lit("Int", 0)),
   ok("Int", "1"))
fx("div-trunc-i8-toward-zero", "div_trunc(-7, 2) on I8 is -3", unwrap(["div_trunc", i8(-7), i8(2)], i8(99)), ok("I8", "-3"))
fx("rem-trunc-i8-sign-of-the-dividend", "rem_trunc(-7, 2) on I8 is -1", unwrap(["rem_trunc", i8(-7), i8(2)], i8(99)), ok("I8", "-1"))
fx("rem-trunc-i8-negative-divisor", "rem_trunc(7, -2) on I8 is 1", unwrap(["rem_trunc", i8(7), i8(-2)], i8(99)), ok("I8", "1"))
fx("div-trunc-i8-minimum-by-minus-one", "div_trunc(-128, -1) on I8 is None: the quotient leaves the type",
   unwrap(["div_trunc", i8(-128), i8(-1)], i8(99)), ok("I8", "99"))
fx("rem-trunc-i8-minimum-by-minus-one", "rem_trunc(-128, -1) on I8 is None too", unwrap(["rem_trunc", i8(-128), i8(-1)], i8(99)),
   ok("I8", "99"))
fx("div-trunc-i8-by-zero", "div_trunc(7, 0) on I8 is None", unwrap(["div_trunc", i8(7), i8(0)], i8(99)), ok("I8", "99"))

# -- imin imax ---------------------------------------------------------------------------------------------------------------------
fx("imax-int", "imax(-3, 5) on Int is 5", ["imax", lit("Int", -3), lit("Int", 5)], ok("Int", "5"))
fx("imin-int", "imin(-3, 5) on Int is -3", ["imin", lit("Int", -3), lit("Int", 5)], ok("Int", "-3"))
fx("imin-two-nats-give-an-int", "imin of two Nat operands has the type Int", ["imin", nat(3), nat(9)], ok("Int", "3"))
fx("imax-mixed-nat-and-int", "imax(Nat 2, Int -1) is 2", ["imax", nat(2), lit("Int", -1)], ok("Int", "2"))

# -- be_bytes le_bytes from_be from_le (the width of the operand is static, section A.1) -----------------------------------------
fx("be-bytes-u16", "be_bytes(258) on U16 is 0102", ["eq", ["be_bytes", lit("U16", 258)], ["lit", {"type": "Bytes", "value": "0102"}]],
   ok("Bool", True))
fx("le-bytes-u16", "le_bytes(258) on U16 is 0201", ["eq", ["le_bytes", lit("U16", 258)], ["lit", {"type": "Bytes", "value": "0201"}]],
   ok("Bool", True))
fx("be-bytes-i16-negative", "be_bytes(-2) on I16 is the two's-complement bytes fffe",
   ["eq", ["be_bytes", lit("I16", -2)], ["lit", {"type": "Bytes", "value": "fffe"}]], ok("Bool", True))
fx("be-bytes-u8", "be_bytes of a U8 is one byte", ["eq", ["be_bytes", u8(7)], ["lit", {"type": "Bytes", "value": "07"}]], ok("Bool", True))
fx("be-bytes-u32-as-bytes", "take(be_bytes(0x01020304), 100) has the type Bytes (take gives plain Bytes)",
   ["take", ["be_bytes", lit("U32", 0x01020304)], nat(100)], ok("Bytes", "01020304"))
fx("from-be-u16", "from_be U16 of 0102 is 258", ["from_be", "U16", ["lit", {"type": "Bytes", "value": "0102"}]], ok("U16", "258"))
fx("from-le-u16", "from_le U16 of 0102 is 513", ["from_le", "U16", ["lit", {"type": "Bytes", "value": "0102"}]], ok("U16", "513"))
fx("from-be-i8-negative", "from_be I8 of ff is -1", ["from_be", "I8", ["lit", {"type": "Bytes", "value": "ff"}]], ok("I8", "-1"))
fx("from-le-concat-of-a-sized-operand", "from_le U32 of concat(t, 000000) with t: Bytes<1> has the static width 4 and is accepted",
   ["from_le", "U32", ["concat", var("t"), ["lit", {"type": "Bytes", "value": "000000"}]]], ok("U32", "1"),
   env=[e("t", "Bytes<1>", "01")])
fx("from-be-concat-of-a-sized-operand", "from_be U32 of concat(t, 000000) is 0x01000000", ["from_be", "U32",
   ["concat", var("t"), ["lit", {"type": "Bytes", "value": "000000"}]]], ok("U32", "16777216"), env=[e("t", "Bytes<1>", "01")])
fx("from-be-u64-of-a-sized-variable", "from_be U64 of a Bytes<8> variable", ["from_be", "U64", var("b")], ok("U64", "258"),
   env=[e("b", "Bytes<8>", "0000000000000102")])

# -- at and unwrap_or ---------------------------------------------------------------------------------------------------------------
fx("at-in-range", "at(b, 1) of 0a0b is Some(11)", unwrap(["at", var("b"), nat(1)], u8(9)), ok("U8", "11"), env=[e("b", "Bytes", "0a0b")])
fx("at-first", "at(b, 0) of 0a0b is Some(10)", unwrap(["at", var("b"), nat(0)], u8(9)), ok("U8", "10"), env=[e("b", "Bytes", "0a0b")])
fx("at-past-the-end", "at(b, 2) of a two-byte value is None", unwrap(["at", var("b"), nat(2)], u8(9)), ok("U8", "9"),
   env=[e("b", "Bytes", "0a0b")])
fx("at-negative-index", "at(b, -1) with an Int index is None", unwrap(["at", var("b"), lit("Int", -1)], u8(9)), ok("U8", "9"),
   env=[e("b", "Bytes", "0a0b")])
fx("at-int-index-in-range", "at(b, 1) with an Int index is Some(11)", unwrap(["at", var("b"), lit("Int", 1)], u8(9)), ok("U8", "11"),
   env=[e("b", "Bytes", "0a0b")])
fx("at-of-the-empty-bytes", "at of the empty Bytes is None", unwrap(["at", var("b"), nat(0)], u8(9)), ok("U8", "9"),
   env=[e("b", "Bytes", "")])
fx("at-of-a-sized-variable", "at of a Bytes<2> variable", unwrap(["at", var("b"), nat(1)], u8(9)), ok("U8", "11"),
   env=[e("b", "Bytes<2>", "0a0b")])
fx("unwrap-or-evaluates-both-operands", "unwrap_or has no short circuit: a subject call in the default is made and counted "
   "although the value is not used", unwrap(["at", var("b"), nat(0)], sub_op("bump", var("a"))), ok("U8", "10", 1),
   env=[e("b", "Bytes", "0a"), e("a", "U8", "5")], models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
fx("unwrap-or-default-is-used-and-counted", "the same expression when the first operand is None: the default is the stand-in's "
   "6, and the call counts once as before", unwrap(["at", var("b"), nat(0)], sub_op("bump", var("a"))), ok("U8", "6", 1),
   env=[e("b", "Bytes", ""), e("a", "U8", "5")], models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
fx("unwrap-or-in-arithmetic", "unwrap_or feeds an ordinary add", ["add", unwrap(["at", var("b"), nat(0)], u8(0)), u8(1)],
   ok("U8", "11"), env=[e("b", "Bytes", "0a")])

# -- ir.fixed: what is refused.  Each case holds one defect and an otherwise accepted tree (R25) ----------------------------------
IT = err("ill-typed")
fx("ill-typed-bit-and-mixed-widths", "bit_and of a U8 and a U16", ["bit_and", u8(1), lit("U16", 1)], IT)
fx("ill-typed-bit-xor-signed-and-unsigned", "bit_xor of an I8 and a U8", ["bit_xor", i8(1), u8(1)], IT)
fx("ill-typed-bit-not-of-a-bool", "bit_not wants a fixed-width operand", ["bit_not", blit(True)], IT)
fx("ill-typed-bit-and-of-nat", "the bit operators are fixed-width only: Nat has no w", ["bit_and", nat(1), nat(1)], IT)
fx("ill-typed-sar-on-an-unsigned-type", "sar wants a signed operand", ["sar", u8(128), nat(1)], IT)
fx("ill-typed-shl-count-is-not-a-nat", "a count that is a U8 is not a Nat", ["shl", u8(1), u8(1)], IT)
fx("ill-typed-shr-count-is-an-int", "a count that is an Int is not a Nat", ["shr", u8(1), lit("Int", 1)], IT)
fx("ill-typed-rotl-count-is-signed", "a count that is an I8 is not a Nat", ["rotl", u8(1), i8(1)], IT)
fx("ill-typed-bit-index-is-a-bool", "bit(a, i) wants a Nat index", ["bit", u8(1), blit(True)], IT)
fx("ill-typed-popcount-of-a-nat", "popcount wants a fixed-width operand", ["popcount", nat(1)], IT)
fx("ill-typed-widen-narrows", "widen U8 of a U16: the range is not inside the target", ["widen", "U8", lit("U16", 1)], IT)
fx("ill-typed-widen-signed-to-unsigned-of-the-same-width", "widen U8 of an I8: the range is not inside the target",
   ["widen", "U8", i8(1)], IT)
fx("ill-typed-widen-to-a-bool", "widen names a fixed-width type", ["widen", "Bool", u8(1)], IT)
fx("ill-typed-wrap-of-a-bool", "wrap wants a fixed-width, Int or Nat operand", ["wrap", "U8", blit(True)], IT)
fx("ill-typed-wrap-to-a-nat", "wrap names a fixed-width type", ["wrap", "Nat", u8(1)], IT)
fx("ill-typed-wrap-names-an-unknown-type", "wrap names a type that does not exist", ["wrap", "U7", nat(1)], IT)
fx("ill-typed-try-to-a-bytes", "try names a fixed-width type", unwrap(["try", "Bytes", nat(1)], nat(0)), IT)
fx("ill-typed-to-nat-of-a-signed-type", "to_nat wants an unsigned operand", ["to_nat", i8(1)], IT)
fx("ill-typed-to-int-of-a-bool", "to_int wants a fixed-width operand", ["to_int", blit(True)], IT)
fx("ill-typed-max-value-of-a-nat", "max_value names a fixed-width type", ["max_value", "Nat"], IT)
fx("ill-typed-checked-add-mixed-widths", "checked_add of a U8 and a U16", unwrap(["checked_add", u8(1), lit("U16", 1)], u8(0)), IT)
fx("ill-typed-unwrap-or-default-of-another-type", "unwrap_or with an I8 default for an Option<U8>",
   unwrap(["checked_add", u8(1), u8(1)], i8(0)), IT)
fx("ill-typed-unwrap-or-of-a-non-option", "unwrap_or wants an Option as its first operand", unwrap(u8(1), u8(0)), IT)
fx("ill-typed-div-of-signed-operands", "div on signed fixed-width operands is refused: use div_trunc", unwrap(["div", i8(7), i8(2)],
   i8(0)), IT)
fx("ill-typed-rem-of-signed-operands", "rem on signed fixed-width operands is refused: use rem_trunc",
   unwrap(["rem", i8(7), i8(2)], i8(0)), IT)
fx("ill-typed-div-trunc-of-unsigned-operands", "div_trunc is for signed operands", unwrap(["div_trunc", u8(7), u8(2)], u8(0)), IT)
fx("ill-typed-imin-of-fixed-width-operands", "imin takes Int or Nat operands", ["imin", u8(1), u8(2)], IT)
fx("ill-typed-be-bytes-of-a-nat", "be_bytes wants a fixed-width operand", ["be_bytes", nat(1)], IT)
fx("ill-typed-from-be-of-a-plain-bytes-operand", "from_be U32 of a Bytes of unknown width is refused: the static width is not 4",
   ["from_be", "U32", var("b")], IT, env=[e("b", "Bytes", "01020304")])
fx("ill-typed-from-le-of-a-plain-bytes-concat", "from_le U32 of concat(plain Bytes, 000000) has no static width",
   ["from_le", "U32", ["concat", var("b"), ["lit", {"type": "Bytes", "value": "000000"}]]], IT, env=[e("b", "Bytes", "01")])
fx("ill-typed-from-be-of-the-wrong-static-width", "from_be U16 of a three-byte literal: the width is 3, not 2",
   ["from_be", "U16", ["lit", {"type": "Bytes", "value": "010203"}]], IT)
fx("ill-typed-from-be-of-the-wrong-sized-variable", "from_be U32 of a Bytes<2> variable", ["from_be", "U32", var("b")], IT,
   env=[e("b", "Bytes<2>", "0102")])
fx("ill-typed-at-index-is-fixed-width", "at wants a Nat or an Int index", unwrap(["at", var("b"), u8(0)], u8(0)), IT,
   env=[e("b", "Bytes", "00")])
fx("ill-typed-at-of-a-u8", "at wants a Bytes operand", unwrap(["at", u8(0), nat(0)], u8(0)), IT)

# -- ir.fixed: cost and overflow ---------------------------------------------------------------------------------------------------------
def fx_doubling(n):
    """d1(a) = unwrap_or(checked_add(a, a), a); dk(a) = d(k-1)(d(k-1)(a)): ir.fixed tags are costed, and the cost doubles."""
    models = {"d1": model([("a", "U8")], unwrap(["checked_add", var("a"), var("a")], var("a")))}
    for k in range(2, n + 1):
        models[f"d{k}"] = model([("a", "U8")], call(f"d{k - 1}", call(f"d{k - 1}", var("a"))))
    return models


fx("cost-fixed-doubling-chain-refused", "a chain of unwrap_or and checked_add that doubles its cost every level is refused",
   call("d9", var("a")), err("ir-cost"), env=[e("a", "U8", "1")], models=fx_doubling(9), order=[f"d{k}" for k in range(1, 10)])
fx("cost-fixed-doubling-chain-short-accepted", "the same chain stopped at six levels is accepted (the value saturates at 128: checked_add is None from there)",
   call("d6", var("a")), ok("U8", "128"), env=[e("a", "U8", "1")], models=fx_doubling(6), order=[f"d{k}" for k in range(1, 7)])
fx("overflow-fixed-to-int-product-corpus-class", "to_int(U64 max) squared leaves the 2^127 window, in a subject-independent node",
   ["mul", ["to_int", var("a")], ["to_int", var("a")]], err("ir-overflow", "corpus"), env=[e("a", "U64", MAX64)])
fx("overflow-fixed-to-int-product-below-a-subject-case-class", "the same product with a subject call under it is a case failure",
   ["mul", ["to_int", sub_op("idw", var("a"))], ["to_int", var("a")]], err("ir-overflow", "case"), env=[e("a", "U64", MAX64)],
   models={"keep": model([("a", "U64")], var("a"))}, ops={"idw": op([("a", "U64")], "U64")}, subjects={"idw": "keep"})
fx("overflow-fixed-count-is-not-an-overflow", "a shift by the largest Nat is a value, never an overflow", ["shl", lit("U64", 1), var("n")],
   ok("U64", "0"), env=[e("n", "Nat", MAX64)])

# -- ir.fixed and ir.tuple: level-missing is decided by what ir reaches, and its requires is the case's levels (R24) --------------
case("level-missing-fixed-tag-at-the-tuple-level", "bit_xor is an ir.fixed tag: levels holds ir.tuple and lacks ir.fixed",
     ["bit_xor", u8(1), u8(3)], err("level-missing"), levels=TU, requires=TU)
case("level-missing-tuple-literal-at-the-fixed-level", "the tuple literal is an ir.tuple tag: levels holds ir.fixed and lacks ir.tuple",
     ["tuple", [u8(1), u8(2)]], err("level-missing"), levels=FX, requires=FX)
case("level-missing-project-at-the-fixed-level", "project is an ir.tuple tag",
     ["project", ["tuple", [u8(1), u8(2)]], 0], err("level-missing"), levels=FX, requires=FX)
case("level-missing-match-on-a-result-at-the-fixed-level", "match is an ir.tuple tag even on a Result",
     ["match", var("r"), [[["ok", ["bind", "x"]], var("x")], [["err_any"], u8(0)]]], err("level-missing"),
     env=[e("r", "Result<U8,E>", {"ok": "1"})], types=ERR, levels=FX, requires=FX)
case("level-missing-tuple-env-type-at-the-fixed-level", "an env entry of a tuple type needs ir.tuple although no tuple tag is used",
     blit(True), err("level-missing"), env=[e("t", "(U8,U8)", ["1", "2"])], levels=FX, requires=FX)
case("level-missing-tuple-op-signature-at-the-fixed-level", "a subject call to an op whose parameter is a pair needs ir.tuple: the tree has no tuple tag, "
     "and the stand-in has the op's signature", sub_op("first", var("t")), err("level-missing"), env=[e("t", "(U8,U8)", ["1", "2"])],
     models={"fst": model([("t", "(U8,U8)")], u8(1))}, ops={"first": op([("t", "(U8,U8)")], "U8")}, subjects={"first": "fst"},
     levels=FX, requires=FX)
case("level-missing-fixed-tag-in-a-model-at-the-tuple-level", "a model body with an ir.fixed tag, called",
     call("m", var("a")), err("level-missing"), env=[e("a", "U8", "1")], models={"m": model([("a", "U8")], ["bit_not", var("a")])},
     levels=TU, requires=TU)

# -- R100: a stand-in counts for requires and never causes level-missing -------------------------------------------------------------
fx("stand-in-body-needs-ir-fixed", "ir holds ir.core tags only; the stand-in of the subject uses bit_xor, so the case needs ir.fixed "
   "(R100): a value", sub_op("flip", var("a")), ok("U8", "4", 1), env=[e("a", "U8", "5")],
   models={"fl": model([("a", "U8")], ["bit_xor", var("a"), u8(1)])}, ops={"flip": op([("a", "U8")], "U8")}, subjects={"flip": "fl"})
fx("stand-in-calls-a-model-that-needs-ir-fixed", "the stand-in is pure core and calls a model whose body uses shl: the closure of the "
   "stand-in counts (R100)", sub_op("dbl", var("a")), ok("U8", "10", 1), env=[e("a", "U8", "5")],
   models={"shift": model([("a", "U8")], ["shl", var("a"), nat(1)]), "dbl": model([("a", "U8")], call("shift", var("a")))},
   ops={"dbl": op([("a", "U8")], "U8")}, subjects={"dbl": "dbl"})
# -- R111 (d): a model may have an Int parameter ([D75]); an op may not ----------------------------------------------------------------
case("int-model-parameter-compared-with-an-int-literal", "a model with an Int parameter, called with an Int literal: accepted ([D75], R111)",
     call("is_five", lit("Int", 5)), ok("Bool", True), models={"is_five": model([("a", "Int")], ["eq", var("a"), lit("Int", 5)])},
     requires=CORE)
case("int-model-parameter-negative-argument", "an Int parameter takes a negative Int literal", call("is_five", lit("Int", -5)),
     ok("Bool", False), models={"is_five": model([("a", "Int")], ["eq", var("a"), lit("Int", 5)])}, requires=CORE)
case("int-model-parameter-with-a-nat-operand", "an Int parameter compared inside the model with a Nat literal widens as in a comparison",
     call("m", lit("Int", 3)), ok("Bool", True),
     models={"m": model([("a", "Int")], ["eq", var("a"), lit("Nat", 3)])}, requires=CORE)
case("ill-typed-op-int-parameter-never-called", "an Int in an op row is refused whether or not ir calls the op (section 2.1, R111 (c), R122)",
     blit(True), IT, ops={"o": op([("a", "Int")], "U8")}, requires=CORE)
case("ill-typed-op-int-result-never-called", "an Int result of an op row is refused too", blit(True), IT,
     ops={"o": op([("a", "U8")], "Int")}, requires=CORE)

# ===============================================================================================
# S10d: ir.tuple, at ``levels`` ``ir.core`` and ``ir.tuple`` (Appendix A.1, R9, R76)
# ===============================================================================================
def tup(*items):
    return ["tuple", list(items)]


def proj(e_, i):
    return ["project", e_, i]


def mt(scrutinee, *arms):
    return ["match", scrutinee, [list(a) for a in arms]]


def ok_(pat):
    return ["ok", pat]


def bind(n):
    return ["bind", n]


WILD = ["wild"]
ANY = ["err_any"]
R_U8 = "Result<U8,E>"
R_PAIR = "Result<(U8,U8),E>"

# -- the tuple literal: 2 to 8 operands, nesting, every kind of component ----------------------------------------------------------
tu("tuple-literal-pair", "a literal pair of a U8 and a Bool", tup(u8(1), blit(True)), ok("(U8,Bool)", ["1", True]))
tu("tuple-literal-arity-3", "a triple", tup(blit(True), blit(False), blit(True)), ok("(Bool,Bool,Bool)", [True, False, True]))
tu("tuple-literal-arity-4", "four components", tup(u8(1), u8(2), u8(3), u8(4)), ok("(U8,U8,U8,U8)", ["1", "2", "3", "4"]))
tu("tuple-literal-arity-5", "five components", tup(*[nat(k) for k in range(5)]), ok("(Nat,Nat,Nat,Nat,Nat)", [str(k) for k in range(5)]))
tu("tuple-literal-arity-6", "six components", tup(*[i8(-k) for k in range(6)]), ok("(I8,I8,I8,I8,I8,I8)", [str(-k) for k in range(6)]))
tu("tuple-literal-arity-7", "seven components", tup(*[blit(k % 2 == 0) for k in range(7)]),
   ok("(Bool,Bool,Bool,Bool,Bool,Bool,Bool)", [k % 2 == 0 for k in range(7)]))
tu("tuple-literal-arity-8", "eight components, the most a tuple may have", tup(*[u8(k) for k in range(8)]),
   ok("(U8,U8,U8,U8,U8,U8,U8,U8)", [str(k) for k in range(8)]))
tu("tuple-literal-nested", "a tuple whose first component is a tuple", tup(tup(u8(1), u8(2)), blit(True)),
   ok("((U8,U8),Bool)", [["1", "2"], True]))
tu("tuple-literal-nested-last", "a tuple whose last component is a tuple", tup(blit(True), tup(u8(1), u8(2))),
   ok("(Bool,(U8,U8))", [True, ["1", "2"]]))
tu("tuple-literal-mixed-components", "a Nat, a signed integer, a Bool and a Bytes variable", tup(nat(5), i8(-1), blit(False), var("b")),
   ok("(Nat,I8,Bool,Bytes)", ["5", "-1", False, "0a0b"]), env=[e("b", "Bytes", "0a0b")])
tu("tuple-literal-with-a-result-component", "a component of type Result", tup(var("r"), i8(-1)),
   ok("(Result<U8,E>,I8)", [{"err": "BAD"}, "-1"]), env=[e("r", R_U8, {"err": "BAD"})], types=ERR)
tu("tuple-literal-of-computed-components", "the components are expressions", tup(["add", var("a"), u8(1)], ["eq", var("a"), u8(5)]),
   ok("(U8,Bool)", ["6", True]), env=[e("a", "U8", "5")])
tu("tuple-literal-nat-component-from-arithmetic", "Nat + Nat has the type Int, and the tuple says so",
   tup(["add", nat(1), nat(2)], u8(0)), ok("(Int,U8)", ["3", "0"]))

# -- project -------------------------------------------------------------------------------------------------------------------------
tu("project-first", "project of a literal pair, index 0", proj(tup(u8(7), blit(True)), 0), ok("U8", "7"))
tu("project-second", "project of a literal pair, index 1", proj(tup(u8(7), blit(True)), 1), ok("Bool", True))
tu("project-of-a-variable", "project of an env tuple", proj(var("t"), 1), ok("Bool", True), env=[e("t", "(U8,Bool)", ["7", True])])
tu("project-last-of-eight", "index 7 of an eight-component tuple", proj(var("t"), 7), ok("Bool", True),
   env=[e("t", "(Bool,Bool,Bool,Bool,Bool,Bool,Bool,Bool)", [False] * 7 + [True])])
tu("project-chain", "project of a project: t.0.1", proj(proj(var("n"), 0), 1), ok("U8", "2"),
   env=[e("n", "((U8,U8),Bool)", [["1", "2"], True])])
tu("project-returns-a-tuple", "a component that is itself a tuple", proj(var("n"), 0), ok("(U8,U8)", ["1", "2"]),
   env=[e("n", "((U8,U8),Bool)", [["1", "2"], True])])
tu("project-of-a-model-result", "project of a call that returns a tuple", proj(call("mk", var("a")), 1), ok("Bool", False),
   env=[e("a", "U8", "5")], models={"mk": model([("a", "U8")], tup(var("a"), ["eq", var("a"), u8(0)]))})
tu("project-of-a-subject-result", "a subject call that returns a tuple is counted once", proj(sub_op("mk", var("a")), 0),
   ok("U8", "6", 1), env=[e("a", "U8", "5")], models={"mkm": model([("a", "U8")], tup(["add", var("a"), u8(1)], blit(True)))},
   ops={"mk": op([("a", "U8")], "(U8,Bool)")}, subjects={"mk": "mkm"})
tu("project-of-an-if", "project of an if whose branches are tuples", proj(["if", var("c"), tup(u8(1), u8(2)), tup(u8(3), u8(4))], 1),
   ok("U8", "4"), env=[e("c", "Bool", False)])

# -- structural equality of tuples ---------------------------------------------------------------------------------------------------
tu("eq-tuples-equal", "tuples are equal component by component", ["eq", var("a"), var("b")], ok("Bool", True),
   env=[e("a", "(U8,Bool)", ["1", True]), e("b", "(U8,Bool)", ["1", True])])
tu("eq-tuples-differ-in-the-last-component", "a tuple that differs in one component is not equal", ["eq", var("a"), var("b")],
   ok("Bool", False), env=[e("a", "(U8,Bool)", ["1", True]), e("b", "(U8,Bool)", ["1", False])])
tu("ne-tuples-differ", "ne on tuples that differ", ["ne", var("a"), var("b")], ok("Bool", True),
   env=[e("a", "(U8,U8)", ["1", "2"]), e("b", "(U8,U8)", ["2", "1"])])
tu("eq-nested-tuples", "equality is structural at every depth", ["eq", var("a"), tup(tup(u8(1), u8(2)), blit(True))], ok("Bool", True),
   env=[e("a", "((U8,U8),Bool)", [["1", "2"], True])])
tu("eq-tuples-with-a-result-component", "a Result component compares as a Result", ["eq", var("a"), var("b")], ok("Bool", False),
   env=[e("a", "(Result<U8,E>,Bool)", [{"ok": "1"}, True]), e("b", "(Result<U8,E>,Bool)", [{"err": "BAD"}, True])], types=ERR)
tu("eq-tuple-of-a-literal", "a variable against a literal tuple", ["eq", var("a"), tup(u8(1), blit(True))], ok("Bool", True),
   env=[e("a", "(U8,Bool)", ["1", True])])
tu("if-selects-a-tuple", "an if whose branches are tuples", ["if", var("c"), tup(u8(1), u8(2)), tup(u8(3), u8(4))], ok("(U8,U8)", ["3", "4"]),
   env=[e("c", "Bool", False)])

# -- tuples in models and at the op boundary -------------------------------------------------------------------------------------
SWAP_M = model([("t", "(U8,U8)")], tup(proj(var("t"), 1), proj(var("t"), 0)))
tu("model-with-a-tuple-parameter-and-result", "a model that swaps the components of a pair", call("swap", var("t")),
   ok("(U8,U8)", ["2", "1"]), env=[e("t", "(U8,U8)", ["1", "2"])], models={"swap": SWAP_M})
tu("model-calls-a-model-with-tuples", "a model chain over tuples", call("twice", var("t")), ok("(U8,U8)", ["1", "2"]),
   env=[e("t", "(U8,U8)", ["1", "2"])], models={"swap": SWAP_M, "twice": model([("t", "(U8,U8)")], call("swap", call("swap", var("t"))))},
   order=["swap", "twice"])
tu("subject-op-with-a-tuple-parameter", "an op whose parameter is a pair; the stand-in projects", sub_op("first", var("t")),
   ok("U8", "1", 1), env=[e("t", "(U8,U8)", ["1", "2"])], models={"fst": model([("t", "(U8,U8)")], proj(var("t"), 0))},
   ops={"first": op([("t", "(U8,U8)")], "U8")}, subjects={"first": "fst"})
tu("subject-op-with-a-tuple-result", "the same op returns a pair and the case compares it", ["eq", sub_op("mk", var("a")), tup(var("a"), blit(True))],
   ok("Bool", True, 1), env=[e("a", "U8", "5")], models={"mkm": model([("a", "U8")], tup(var("a"), blit(True)))},
   ops={"mk": op([("a", "U8")], "(U8,Bool)")}, subjects={"mk": "mkm"})


def deep_pair(k):
    """``(((U8,U8),U8),..,U8)`` with ``k`` nested tuples, and its value: the type text depth is ``k`` (section 2.2 rule 1)."""
    text, value = "U8", "1"
    for _ in range(k):
        text, value = f"({text},U8)", [value, "2"]
    return text, value


_T16, _V16 = deep_pair(16)
_T17, _V17 = deep_pair(17)
tu("tuple-type-sixteen-deep-is-accepted", "an env type with 16 nested tuples is within section 2.2 rule 1: project reads the last component",
   proj(var("t"), 1), ok("U8", "2"), env=[e("t", _T16, _V16)])

# -- match on a Result with every pattern of Appendix A.1 (exactly one arm body is evaluated, rule 2) ----------------------------------
ROK, RERR = {"ok": "7"}, {"err": "BAD"}
MATCH_OK_BIND = lambda: mt(var("r"), (ok_(bind("x")), var("x")), (ANY, u8(0)))  # noqa: E731
tu("match-ok-bind-on-ok", "ok(bind x) takes the payload", MATCH_OK_BIND(), ok("U8", "7"), env=[e("r", R_U8, ROK)], types=ERR)
tu("match-ok-bind-on-err", "err_any is the other arm", MATCH_OK_BIND(), ok("U8", "0"), env=[e("r", R_U8, RERR)], types=ERR)


def err_cases():
    return mt(var("r"), (["err", "BAD"], nat(1)), (["err", "WORSE"], nat(2)), (ok_(WILD), nat(0)))


tu("match-err-case-bad", "err(BAD) names one case", err_cases(), ok("Nat", "1"), env=[e("r", R_U8, RERR)], types=ERR)
tu("match-err-case-worse", "err(WORSE) names the other", err_cases(), ok("Nat", "2"), env=[e("r", R_U8, {"err": "WORSE"})], types=ERR)
tu("match-ok-wild-after-err-cases", "ok(_) matches an Ok whatever its payload", err_cases(), ok("Nat", "0"),
   env=[e("r", R_U8, ROK)], types=ERR)
tu("match-first-matching-arm-wins", "err(BAD) and err_any both match an Err(BAD): the first arm is the one taken",
   mt(var("r"), (["err", "BAD"], nat(1)), (ANY, nat(2)), (ok_(WILD), nat(0))), ok("Nat", "1"), env=[e("r", R_U8, RERR)], types=ERR)
tu("match-err-any-after-a-case", "an Err(WORSE) falls through the err(BAD) arm to err_any",
   mt(var("r"), (["err", "BAD"], nat(1)), (ANY, nat(2)), (ok_(WILD), nat(0))), ok("Nat", "2"),
   env=[e("r", R_U8, {"err": "WORSE"})], types=ERR)
tu("match-bind-the-whole-result", "a bare bind as the last arm matches whatever is left, here an Err, and names the whole value",
   mt(var("r"), (ok_(WILD), blit(True)), (bind("w"), ["eq", var("w"), var("r")])), ok("Bool", True), env=[e("r", R_U8, RERR)], types=ERR)
tu("match-wild-arm", "a wild arm matches whatever is left", mt(var("r"), (ok_(WILD), nat(1)), (WILD, nat(0))), ok("Nat", "0"),
   env=[e("r", R_U8, RERR)], types=ERR)
tu("match-ok-tuple-pattern", "ok(tuple(bind a, bind b)) takes both components", mt(var("r"),
   (ok_(["tuple", [bind("a"), bind("b")]]), ["add", var("a"), var("b")]), (ANY, u8(0))), ok("U8", "7"),
   env=[e("r", R_PAIR, {"ok": ["3", "4"]})], types=ERR)
tu("match-ok-tuple-pattern-with-a-wild-component", "a wild component binds nothing", mt(var("r"),
   (ok_(["tuple", [WILD, bind("b")]]), var("b")), (ANY, u8(0))), ok("U8", "4"), env=[e("r", R_PAIR, {"ok": ["3", "4"]})], types=ERR)
tu("match-ok-nested-tuple-pattern", "a tuple pattern inside a tuple pattern", mt(var("r"),
   (ok_(["tuple", [["tuple", [bind("a"), WILD]], bind("c")]]), ["add", var("a"), var("c")]), (ANY, u8(0))), ok("U8", "4"),
   env=[e("r", "Result<((U8,U8),U8),E>", {"ok": [["1", "2"], "3"]})], types=ERR)
NESTED = lambda: mt(var("r"), (ok_(ok_(WILD)), nat(0)), (ok_(ANY), nat(1)), (ANY, nat(2)))  # noqa: E731
RR = "Result<Result<U8,E>,E>"
tu("match-nested-result-ok-ok", "ok(ok(_)) looks through two Results", NESTED(), ok("Nat", "0"), env=[e("r", RR, {"ok": {"ok": "1"}})], types=ERR)
tu("match-nested-result-ok-err", "ok(err_any) is an Ok that holds an Err", NESTED(), ok("Nat", "1"),
   env=[e("r", RR, {"ok": {"err": "BAD"}})], types=ERR)
tu("match-nested-result-err", "err_any is the outer Err", NESTED(), ok("Nat", "2"), env=[e("r", RR, {"err": "WORSE"})], types=ERR)
tu("match-arm-returns-a-tuple", "the arms may return tuples: the components are swapped", mt(var("r"),
   (ok_(["tuple", [bind("a"), bind("b")]]), tup(var("b"), var("a"))), (ANY, tup(u8(0), u8(0)))), ok("(U8,U8)", ["4", "3"]),
   env=[e("r", R_PAIR, {"ok": ["3", "4"]})], types=ERR)
tu("match-arm-returns-a-result-ok", "an arm that rebuilds an Ok and an arm that returns the scrutinee: the type is the variable's",
   mt(var("r"), (ok_(bind("x")), ["ok", ["add", var("x"), u8(1)]]), (ANY, var("r"))), ok(R_U8, {"ok": "8"}),
   env=[e("r", R_U8, ROK)], types=ERR)
tu("match-arm-returns-a-result-err", "the same match on an Err returns it unchanged", mt(var("r"),
   (ok_(bind("x")), ["ok", ["add", var("x"), u8(1)]]), (ANY, var("r"))), ok(R_U8, RERR), env=[e("r", R_U8, RERR)], types=ERR)
tu("match-arm-arithmetic-wraps", "an arm is an ordinary expression: 255 + 1 wraps on a U8",
   mt(var("r"), (ok_(bind("x")), ["add", var("x"), u8(1)]), (ANY, u8(0))), ok("U8", "0"), env=[e("r", R_U8, {"ok": "255"})], types=ERR)
tu("match-on-a-model-result", "the scrutinee is a call that returns a Result", mt(call("k", var("a")),
   (ok_(bind("x")), var("x")), (ANY, u8(0))), ok("U8", "9"), env=[e("a", "Bool", True)],
   models={"k": model([("a", "Bool")], ["if", var("a"), ["ok", u8(9)], ["err", "BAD"]])}, types=ERR)
tu("match-in-a-model-body", "a match in a model body", call("m", var("r")), ok("U8", "7"), env=[e("r", R_U8, ROK)], types=ERR,
   models={"m": model([("r", R_U8)], mt(var("r"), (ok_(bind("x")), var("x")), (ANY, u8(0))))})
tu("match-inner-match-in-an-arm", "a match inside an arm, over a second Result", mt(var("r"),
   (ok_(bind("x")), mt(var("s"), (ok_(bind("y")), ["add", var("x"), var("y")]), (ANY, u8(0)))), (ANY, u8(0))), ok("U8", "10"),
   env=[e("r", R_U8, ROK), e("s", R_U8, {"ok": "3"})], types=ERR)
STAND = dict(models={"inc": INC}, ops={"bump": BUMP}, subjects={"bump": "inc"})
tu("match-only-the-taken-arm-counts-ok", "a subject call in the err arm is not made when the Ok arm is taken: calls 0",
   mt(var("r"), (ok_(bind("x")), var("x")), (ANY, sub_op("bump", u8(5)))), ok("U8", "7", 0), env=[e("r", R_U8, ROK)], types=ERR, **STAND)
tu("match-only-the-taken-arm-counts-err", "the same match on an Err makes the one call: calls 1, the stand-in's 6",
   mt(var("r"), (ok_(bind("x")), var("x")), (ANY, sub_op("bump", u8(5)))), ok("U8", "6", 1), env=[e("r", R_U8, RERR)], types=ERR, **STAND)
IDR = model([("r", R_U8)], var("r"))
tu("match-subject-call-in-the-scrutinee-is-counted", "a subject call that returns the Result being matched is counted once; the value of its "
   "stand-in decides the arm", mt(sub_op("res", var("r")), (ok_(bind("x")), var("x")), (ANY, u8(0))), ok("U8", "7", 1),
   env=[e("r", R_U8, ROK)], types=ERR, models={"idr": IDR}, ops={"res": op([("r", R_U8)], R_U8)}, subjects={"res": "idr"})

# -- ir.tuple: what is refused.  A ``match`` that cannot be compiled is ill-typed in the IR (A.1: code ``ir``) ------------------------
tu("ill-typed-tuple-of-one-operand", "a tuple literal has 2 to 8 operands: one is refused", tup(u8(1)), IT)
tu("ill-typed-tuple-of-nine-operands", "a tuple literal of nine operands", tup(*[u8(k) for k in range(9)]), IT)
tu("ill-typed-project-past-the-arity", "project index 2 of a pair", proj(tup(u8(1), u8(2)), 2), IT)
tu("ill-typed-project-of-a-non-tuple", "project of a U8", proj(u8(1), 0), IT)
tu("ill-typed-eq-of-tuples-of-different-types", "a (U8,U8) is never equal to a (U8,Bool)", ["eq", tup(u8(1), u8(2)), tup(u8(1), blit(True))], IT)
# Not here, on purpose: a match that can fall through, an arm no value can reach, the same bind twice in a pattern and a bind that
# shadows a name are corpus errors of Appendix A.1 (the oracle's code ``corpus``, the generator's ``ir``), and the closed set of
# ``v2-ir`` has no code for them: ``ill-typed`` is "any other static type error" and A.1 says these are not typing errors.  They stay
# oracle tests (``tests/test_tuple_lowering.py`` and ``tests/test_tuple_eval.py``) until the owner gives them a code.
tu("ill-typed-match-arms-of-different-types", "one arm is a U8 and the other a Bool", mt(var("r"), (ok_(WILD), u8(1)), (ANY, blit(True))), IT,
   env=[e("r", R_U8, ROK)], types=ERR)
tu("ill-typed-match-tuple-pattern-of-the-wrong-arity", "a one-component pattern for a pair",
   mt(var("r"), (ok_(["tuple", [bind("a")]]), var("a")), (ANY, u8(0))), IT, env=[e("r", R_PAIR, {"ok": ["1", "2"]})], types=ERR)
tu("ill-typed-match-tuple-pattern-on-a-scalar-payload", "ok(tuple(..)) on a Result<U8,E>",
   mt(var("r"), (ok_(["tuple", [bind("a"), bind("b")]]), var("a")), (ANY, u8(0))), IT, env=[e("r", R_U8, ROK)], types=ERR)
# -- R76: the level of match is type-directed.  A match on a Result needs ir.tuple; any other scrutinee needs ir.data -----------------
tu("level-missing-match-on-a-bare-tuple-needs-ir-data", "the same match JSON on a bare tuple is ir.data, which levels lacks: "
   "level-missing (R76); its requires is the levels",
   mt(var("t"), (["tuple", [bind("a"), bind("b")]], ["add", var("a"), var("b")])), err("level-missing"),
   env=[e("t", "(U8,U8)", ["1", "2"])], requires=TU)
tu("level-missing-match-on-a-bool-needs-ir-data", "a match on a Bool is ir.data too: a bare bind arm does not change that",
   mt(var("c"), (bind("x"), var("x"))), err("level-missing"), env=[e("c", "Bool", True)], requires=TU)
tu("level-missing-match-on-a-model-tuple-needs-ir-data", "a tuple scrutinee that comes from a call is a bare tuple as well",
   mt(call("mk", var("a")), (["tuple", [bind("x"), WILD]], var("x"))), err("level-missing"), env=[e("a", "U8", "1")],
   models={"mk": model([("a", "U8")], tup(var("a"), blit(True)))}, requires=TU)
case("level-missing-match-on-a-result-at-the-core-level", "at levels ir.core alone a match is refused, whatever its scrutinee: match is "
     "an ir.tuple tag.  This case names ir.core only, so a lane that implements ir.core runs it",
     mt(var("r"), (ok_(bind("x")), var("x")), (ANY, u8(0))), err("level-missing"), env=[e("r", R_U8, ROK)], types=ERR, requires=CORE)


# ===============================================================================================
# S10d: R100 (a stand-in nothing reaches) and R120 (an env type the reader refuses), after the Python lane's 16.23 follow-up
# ===============================================================================================
fx("stand-in-of-an-op-nothing-reaches-counts-for-nothing", "a stand-in bound to an op that ir does not call is never run: its ir.fixed "
   "tag adds no level (R100)", blit(True), ok("Bool", True), levels=("ir.core",), requires=CORE,
   models={"fl": model([("a", "U8")], ["bit_not", var("a")])}, ops={"flip": op([("a", "U8")], "U8")}, subjects={"flip": "fl"})
# -- R120 (d): an env type the reader refuses is ill-typed ---------------------------------------------------------------------------------
case("ill-typed-env-entry-of-type-int", "an env entry of type Int: Int is IR-internal and never a value type (R120)", blit(True), IT,
     env=[e("x", "Int", "1")], requires=CORE)
case("ill-typed-env-entry-of-an-undeclared-type", "an env entry whose type types does not declare (R120)", blit(True), IT,
     env=[e("x", "Frob", "1")], requires=CORE)
case("ill-typed-env-entry-result-with-a-record-as-error", "Result<U8,Q> with Q a record: the error type of a Result is an error type "
     "(R117, R120)", blit(True), IT, env=[e("x", "Result<U8,Q>", {"ok": "1"})],
     types={"Q": {"kind": "record", "fields": [["f", "U8"]]}}, requires=CORE)

tu("ill-typed-tuple-type-seventeen-deep-env", "an env type with 17 nested tuples is deeper than 16 and refused (R120)", blit(True),
   IT, env=[e("t", _T17, _V17)], requires=CORE)


# ===============================================================================================
# S10d: ir.fixed at 16, 32 and 64 bits (the review's N4).  Expected values are written from the bit pattern as text, a different
# route from the oracle's integer shifts: ``bits(v, w)`` is the w-bit two's-complement pattern of v, and a result is read back with
# ``from_bits``.  Each case holds one defect or none.
# ===============================================================================================
def bits(v, w):
    return format(v % (1 << w), f"0{w}b")


def from_bits(text, signed):
    w = len(text)
    v = int(text, 2)
    return v - (1 << w) if signed and text[0] == "1" else v


for _t in ("U16", "U32", "U64", "I16", "I32", "I64"):
    _w, _signed = int(_t[1:]), _t[0] == "I"
    _k = _t.lower()
    _lo, _hi = lo(_t), hi(_t)
    _top = 1 << (_w - 1)  # the pattern 1000..0, as a value of the type
    _topv = from_bits(bits(_top, _w), _signed)
    # bit_or, bit_xor, bit_not on the edge patterns
    fx(f"bit-or-{_k}-minimum-and-one", f"bit_or(the pattern 10..0, 1) on {_t}", ["bit_or", lit(_t, _topv), lit(_t, 1)],
       ok(_t, str(from_bits("1" + "0" * (_w - 2) + "1", _signed))))
    fx(f"bit-xor-{_k}-maximum-and-top", f"bit_xor(all ones, the pattern 10..0) on {_t}",
       ["bit_xor", lit(_t, from_bits("1" * _w, _signed)), lit(_t, _topv)], ok(_t, str(from_bits("0" + "1" * (_w - 1), _signed))))
    fx(f"bit-not-{_k}-zero", f"bit_not(0) on {_t}", ["bit_not", lit(_t, 0)], ok(_t, str(from_bits("1" * _w, _signed))))
    fx(f"bit-not-{_k}-top-pattern", f"bit_not of the pattern 10..0 on {_t}", ["bit_not", lit(_t, _topv)],
       ok(_t, str(from_bits("0" + "1" * (_w - 1), _signed))))
    # shifts: by w-1, by w (n >= w gives 0), by one
    fx(f"shl-{_k}-by-width-minus-one", f"shl(1, {_w - 1}) on {_t} is the pattern 10..0", ["shl", lit(_t, 1), nat(_w - 1)],
       ok(_t, str(_topv)))
    fx(f"shl-{_k}-by-the-width", f"shl(1, {_w}) on {_t}: n >= w gives 0", ["shl", lit(_t, 1), nat(_w)], ok(_t, "0"))
    fx(f"shl-{_k}-drops-the-top-bit", f"shl(all ones, 1) on {_t}: the pattern 1..10", ["shl", lit(_t, from_bits("1" * _w, _signed)), nat(1)],
       ok(_t, str(from_bits("1" * (_w - 1) + "0", _signed))))
    fx(f"shr-{_k}-top-pattern-by-width-minus-one", f"shr(the pattern 10..0, {_w - 1}) on {_t} is 1: logical", ["shr", lit(_t, _topv), nat(_w - 1)],
       ok(_t, "1"))
    fx(f"shr-{_k}-all-ones-by-one", f"shr(all ones, 1) on {_t}: the pattern 01..1", ["shr", lit(_t, from_bits("1" * _w, _signed)), nat(1)],
       ok(_t, str(from_bits("0" + "1" * (_w - 1), _signed))))
    fx(f"shr-{_k}-by-the-width", f"shr(all ones, {_w}) on {_t}: n >= w gives 0", ["shr", lit(_t, from_bits("1" * _w, _signed)), nat(_w)],
       ok(_t, "0"))
    if _signed:
        fx(f"sar-{_k}-minimum-by-width-minus-one", f"sar({_lo}, {_w - 1}) on {_t} is -1: sign fill", ["sar", lit(_t, _lo), nat(_w - 1)],
           ok(_t, "-1"))
        fx(f"sar-{_k}-maximum-by-width-minus-one", f"sar(max, {_w - 1}) on {_t} is 0", ["sar", lit(_t, _hi), nat(_w - 1)], ok(_t, "0"))
        fx(f"sar-{_k}-minimum-by-one", f"sar(min, 1) on {_t}: the pattern 110..0", ["sar", lit(_t, _lo), nat(1)],
           ok(_t, str(from_bits("11" + "0" * (_w - 2), True))))
        fx(f"sar-{_k}-negative-by-the-width", f"sar(-1, {_w}) on {_t}: n >= w gives -1 for a negative value", ["sar", lit(_t, -1), nat(_w)],
           ok(_t, "-1"))
        fx(f"sar-{_k}-positive-by-the-width", f"sar(max, {_w}) on {_t}: n >= w gives 0 for a non-negative value", ["sar", lit(_t, _hi), nat(_w)],
           ok(_t, "0"))
    # rotations: by w-1, by w (a full turn), by w+1 (reduced mod w)
    fx(f"rotl-{_k}-one-by-width-minus-one", f"rotl(1, {_w - 1}) on {_t}", ["rotl", lit(_t, 1), nat(_w - 1)], ok(_t, str(_topv)))
    fx(f"rotl-{_k}-by-the-width", f"rotl(1, {_w}) on {_t} is a full turn", ["rotl", lit(_t, 1), nat(_w)], ok(_t, "1"))
    fx(f"rotl-{_k}-by-width-plus-one", f"rotl(1, {_w + 1}) on {_t}: the count reduces mod w to 1", ["rotl", lit(_t, 1), nat(_w + 1)], ok(_t, "2"))
    fx(f"rotl-{_k}-top-pattern-by-one", f"rotl(the pattern 10..0, 1) on {_t} is 1", ["rotl", lit(_t, _topv), nat(1)], ok(_t, "1"))
    fx(f"rotr-{_k}-one-by-one", f"rotr(1, 1) on {_t} is the pattern 10..0", ["rotr", lit(_t, 1), nat(1)], ok(_t, str(_topv)))
    fx(f"rotr-{_k}-by-the-width", f"rotr(1, {_w}) on {_t} is a full turn", ["rotr", lit(_t, 1), nat(_w)], ok(_t, "1"))
    fx(f"rotr-{_k}-top-pattern-by-width-minus-one", f"rotr(the pattern 10..0, {_w - 1}) on {_t} is 1 (a left turn by one)", ["rotr", lit(_t, _topv), nat(_w - 1)],
       ok(_t, "1"))
    # popcount clz ctz bit
    fx(f"popcount-{_k}-all-ones", f"popcount of the all-ones pattern on {_t} is {_w}", ["popcount", lit(_t, from_bits("1" * _w, _signed))],
       ok("Nat", str(_w)))
    fx(f"popcount-{_k}-top-pattern", f"popcount of the pattern 10..0 on {_t} is 1", ["popcount", lit(_t, _topv)], ok("Nat", "1"))
    fx(f"clz-{_k}-zero", f"clz(0) on {_t} is {_w}", ["clz", lit(_t, 0)], ok("Nat", str(_w)))
    fx(f"clz-{_k}-one", f"clz(1) on {_t} is {_w - 1}", ["clz", lit(_t, 1)], ok("Nat", str(_w - 1)))
    fx(f"clz-{_k}-top-pattern", f"clz of the pattern 10..0 on {_t} is 0", ["clz", lit(_t, _topv)], ok("Nat", "0"))
    fx(f"ctz-{_k}-zero", f"ctz(0) on {_t} is {_w}", ["ctz", lit(_t, 0)], ok("Nat", str(_w)))
    fx(f"ctz-{_k}-top-pattern", f"ctz of the pattern 10..0 on {_t} is {_w - 1}", ["ctz", lit(_t, _topv)], ok("Nat", str(_w - 1)))
    fx(f"ctz-{_k}-one", f"ctz(1) on {_t} is 0", ["ctz", lit(_t, 1)], ok("Nat", "0"))
    fx(f"bit-{_k}-top-bit-set", f"bit(the pattern 10..0, {_w - 1}) on {_t} is true", ["bit", lit(_t, _topv), nat(_w - 1)], ok("Bool", True))
    fx(f"bit-{_k}-index-equal-to-the-width", f"bit(all ones, {_w}) on {_t} is false: i >= w", ["bit", lit(_t, from_bits("1" * _w, _signed)), nat(_w)],
       ok("Bool", False))
    fx(f"bit-{_k}-low-bit-of-one", f"bit(1, 0) on {_t} is true", ["bit", lit(_t, 1), nat(0)], ok("Bool", True))
    # div_trunc and rem_trunc (signed only)
    if _signed:
        fx(f"div-trunc-{_k}-toward-zero", f"div_trunc(-7, 2) on {_t} is -3", unwrap(["div_trunc", lit(_t, -7), lit(_t, 2)], lit(_t, 99)), ok(_t, "-3"))
        fx(f"rem-trunc-{_k}-sign-of-the-dividend", f"rem_trunc(-7, 2) on {_t} is -1", unwrap(["rem_trunc", lit(_t, -7), lit(_t, 2)], lit(_t, 99)),
           ok(_t, "-1"))
        fx(f"div-trunc-{_k}-minimum-by-minus-one", f"div_trunc(min, -1) on {_t} is None: the quotient leaves the type",
           unwrap(["div_trunc", lit(_t, _lo), lit(_t, -1)], lit(_t, 99)), ok(_t, "99"))
        fx(f"rem-trunc-{_k}-minimum-by-minus-one", f"rem_trunc(min, -1) on {_t} is None too", unwrap(["rem_trunc", lit(_t, _lo), lit(_t, -1)], lit(_t, 99)),
           ok(_t, "99"))
        fx(f"div-trunc-{_k}-maximum-by-minus-one", f"div_trunc(max, -1) on {_t} is -max", unwrap(["div_trunc", lit(_t, _hi), lit(_t, -1)], lit(_t, 99)),
           ok(_t, str(-_hi)))
        fx(f"div-trunc-{_k}-minimum-by-three", f"div_trunc(min, 3) on {_t} truncates toward zero", unwrap(["div_trunc", lit(_t, _lo), lit(_t, 3)], lit(_t, 99)),
           ok(_t, str(-(-_lo // 3))))
        fx(f"rem-trunc-{_k}-minimum-by-three", f"rem_trunc(min, 3) on {_t} has the dividend's sign", unwrap(["rem_trunc", lit(_t, _lo), lit(_t, 3)], lit(_t, 99)),
           ok(_t, str(-(-_lo % 3))))
        fx(f"div-trunc-{_k}-by-zero", f"div_trunc(5, 0) on {_t} is None", unwrap(["div_trunc", lit(_t, 5), lit(_t, 0)], lit(_t, 99)), ok(_t, "99"))


# ===============================================================================================
# S10d: the cost of a match and the class of an overflow in a match arm (the review's N5; section 4.2 rule 6, R71)
# ===============================================================================================
# The cost of a match is 1 + cost(scrutinee) + the **maximum** over the arms, never the sum.  The scrutinee ``r`` is a variable (cost 1),
# the arms are ``expr_of_cost`` expressions over the Bool variable ``a``.
MCM = dict(env=[e("a", "Bool", True), e("r", R_U8, ROK)], models=cost_models(), order=[f"c{k}" for k in range(1, 10)], types=ERR)


def mcost(arm1, arm2):
    return mt(var("r"), (ok_(WILD), arm1), (ANY, arm2))


tu("cost-match-two-arms-of-1023-are-accepted", "two arms of cost 1,023 cost 1 + 1 + 1,023 = 1,025: the maximum, not the sum (which would be 2,048)",
   mcost(expr_of_cost(1023), expr_of_cost(1023)), ok("Bool", True), **MCM)
tu("cost-match-the-larger-arm-at-1998-is-accepted", "1 + 1 + 1,998 = 2,000 is the largest accepted cost", mcost(expr_of_cost(1998), blit(True)),
   ok("Bool", True), **MCM)
tu("cost-match-the-larger-arm-at-1999-is-refused", "1 + 1 + 1,999 = 2,001: the second arm decides", mcost(blit(True), expr_of_cost(1999)),
   err("ir-cost"), **MCM)
OVI = ["mul", lit("Nat", NAT_MAX), lit("Nat", NAT_MAX)]
SUBJ_R = sub_op("res", var("r"))
OVM = dict(env=[e("r", R_U8, ROK)], types=ERR, models={"idr": IDR}, ops={"res": op([("r", R_U8)], R_U8)}, subjects={"res": "idr"})
tu("overflow-in-the-taken-match-arm-pure-scrutinee-corpus-class", "a pure overflow in the arm that is taken, under a scrutinee with no subject call: the "
   "generator evaluates it, so the class is corpus", mt(var("r"), (ok_(WILD), ["add", OVI, lit("Int", 0)]), (ANY, lit("Int", 0))),
   err("ir-overflow", "corpus"), env=[e("r", R_U8, ROK)], types=ERR)
tu("overflow-in-the-taken-match-arm-subject-scrutinee-case-class", "the same arm under a scrutinee that calls the subject: the arm is under a guard "
   "with a subject call, so the class is case (R71)", mt(SUBJ_R, (ok_(WILD), ["add", OVI, lit("Int", 0)]), (ANY, lit("Int", 0))),
   err("ir-overflow", "case"), **OVM)
tu("match-arm-overflow-not-taken-is-a-value", "the overflowing arm is not taken: a value (the static pass finds no overflow, section 4.2 rule 6)",
   mt(var("r"), (ok_(WILD), lit("Int", 0)), (ANY, ["add", OVI, lit("Int", 0)])), ok("Int", "0"), env=[e("r", R_U8, ROK)], types=ERR)
