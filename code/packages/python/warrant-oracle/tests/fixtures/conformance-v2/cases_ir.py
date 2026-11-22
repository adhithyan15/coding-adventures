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
    return {"params": [list(p) for p in params], "ret": ret, "requires": CORE}


# Stand-ins, used by many rows: bump(a) = a + 1 on U8 and add2(a, b) = a + b on U8.
INC = model([("a", "U8")], ["add", var("a"), lit("U8", 1)])
ADDM = model([("a", "U8"), ("b", "U8")], ["add", var("a"), var("b")])
BUMP = op([("a", "U8")], "U8")
ADD2 = op([("a", "U8"), ("b", "U8")], "U8")


def case(slug, summary, ir, intent, *, env=(), models=None, order=None, ops=None, subjects=None,
         types=None, levels=("ir.core",), max_ir_cost=None, role=None):
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
    ROWS.append(row(slug, summary, fields, intent))


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
