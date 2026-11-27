"""``v2-fault``: the pre / post / replace wrapper of section 8.1 (slice S5a table).

A case carries one fault row, the section 6 row of the op the fault names (``fault.op`` is the name;
the row itself carries none), the ``types`` and ``levels``, the call's ``args`` in parameter order and
the ``result`` the call returned.  The portable faults are **instantiated by the oracle's own templates**
(``instantiate_fault`` calls ``faults.fault_row``), so the kit pins the same JSON a real corpus carries;
the three hand-written rows are the ones no template writes: a ``pre`` fault whose rewrites would
disagree if applied one after another, a ``replace`` that ignores ``result``, and a row with a
``subject_op`` in it.  There is no ``models`` field: an S5 fault expression calls no model.

The expectation is ``{"args": [..]}`` for ``pre`` (every argument in parameter order after the rewrite,
unchanged ones included), ``{"result": V}`` for ``post`` and ``replace``, or an error: ``level-missing``
(the row needs a level that ``levels`` lacks: a lane records such a control ``not-live``) or
``subject-forbidden`` (a fault expression contains a ``subject_op``).
"""

from __future__ import annotations

from kit_common import row

from coding_adventures_warrant_oracle.conformance import instantiate_fault

ROWS: list = []
CORE = ["ir.core"]
ERR = {"E": {"kind": "error", "cases": ["BAD", "WORSE"]}}


def op(params, ret):
    """An op row as a corpus writer emits it: ``requires`` is computed from the signature (R9), ``ir.tuple`` when a type holds a tuple."""
    tuple_typed = any("(" in t for t in [*(t for _, t in params), ret])
    return {"params": [list(p) for p in params], "ret": ret, "requires": ["ir.core", "ir.tuple"] if tuple_typed else CORE}


OPS = {
    "cmp2": op([("a", "Bytes"), ("b", "Bytes")], "Bool"),
    "mul8": op([("a", "U8"), ("b", "U8")], "U8"),
    "swap_op": op([("a", "U8"), ("b", "U8")], "U8"),
    "count_op": op([("a", "Bytes")], "Nat"),
    "echo": op([("a", "Bytes")], "Bytes"),
    "pick": op([("a", "Bytes"), ("b", "Bytes"), ("choice", "Bool")], "Result<Bytes,E>"),
    "wide": op([("a", "U64")], "U64"),
}


def v(a):
    return ["var", a]


def fault_case(slug, summary, fault, op_name, args, result, intent, *, levels=("ir.core",), types=None, requires=None):
    if isinstance(fault, str):
        fault = instantiate_fault(fault, op_name, OPS[op_name], types or {})
    fields = {
        "fault": fault,
        "op": OPS[op_name],
        "types": types or {},
        "levels": sorted(levels),
        "args": args,
        "result": result,
    }
    ROWS.append(row(slug, summary, fields, intent, requires))


def post(name, op_name, args, result, want, slug, summary, **kw):
    fault_case(slug, summary, name, op_name, args, result, {"result": want}, **kw)


# -- invert ------------------------------------------------------------------------------------------------
post("invert", "cmp2", ["0a", "0a"], True, False, "invert-true-result", "invert flips a true result")
post("invert", "cmp2", ["0a", "0b"], False, True, "invert-false-result", "invert flips a false result")

# -- off-by-one: result + 1 wraps (section 3), T is the result type, Nat included ---------------------------------
post("off-by-one", "mul8", ["2", "3"], "255", "0", "off-by-one-u8-wraps", "255 + 1 wraps to 0 on a U8 result")
post("off-by-one", "mul8", ["2", "3"], "6", "7", "off-by-one-u8", "6 + 1 on a U8 result")
post("off-by-one", "mul8", ["2", "3"], "0", "1", "off-by-one-u8-from-zero", "0 + 1 on a U8 result")
post("off-by-one", "count_op", ["0a0b"], "5", "6", "off-by-one-nat", "5 + 1 on a Nat result is exact")
post("off-by-one", "count_op", ["0a0b"], "0", "1", "off-by-one-nat-from-zero", "0 + 1 on a Nat result")
post("off-by-one", "count_op", ["0a0b"], str(2**64 - 2), str(2**64 - 1), "off-by-one-nat-below-max",
     "the largest Nat is reachable: 2^64-2 + 1")

# -- truncate-last and drop-last-result-byte -----------------------------------------------------------------------
post("truncate-last", "echo", ["0a"], "0a0b0c", "0a0b", "truncate-last-three-bytes", "the last byte of a non-empty result is cut")
post("truncate-last", "echo", ["0a"], "0a", "", "truncate-last-one-byte", "a one-byte result becomes empty")
post("truncate-last", "echo", ["0a"], "", "", "truncate-last-empty", "an empty result is left alone (the length guard)")
post("drop-last-result-byte", "echo", ["0a"], "0a0b0c", "0a0b", "drop-last-three-bytes", "the core form drops the last byte")
post("drop-last-result-byte", "echo", ["0a"], "ff", "", "drop-last-one-byte", "a one-byte result becomes empty")
post("drop-last-result-byte", "echo", ["0a"], "", "00", "drop-last-empty-gives-00", "the empty result becomes the byte 00")

# -- missing-error: a replace fault (the call is still made, the result is not trusted) ---------------------------
fault_case("missing-error-on-err", "an Err result becomes Ok(first Bytes argument)", "missing-error", "pick",
           ["0a", "0b0c", True], {"err": "BAD"}, {"result": {"ok": "0a"}}, types=ERR)
fault_case("missing-error-on-ok", "an Ok result is returned unchanged", "missing-error", "pick",
           ["0a", "0b0c", True], {"ok": "0102"}, {"result": {"ok": "0102"}}, types=ERR)
fault_case("missing-error-empty-first-argument", "the first Bytes argument may be empty", "missing-error", "pick",
           ["", "0b", False], {"err": "WORSE"}, {"result": {"ok": ""}}, types=ERR)

# -- swap-choice: a pre fault on one parameter ------------------------------------------------------------------------
fault_case("swap-choice-true", "the choice flag is negated before the call", "swap-choice", "pick",
           ["0a", "0b", True], {"ok": "0a"}, {"args": ["0a", "0b", False]}, types=ERR)
fault_case("swap-choice-false", "false becomes true; the Bytes arguments are reported unchanged", "swap-choice", "pick",
           ["0a", "0b", False], {"ok": "0b"}, {"args": ["0a", "0b", True]}, types=ERR)

# -- ignore-last-byte: a pre fault on each Bytes parameter ---------------------------------------------------------------
fault_case("ignore-last-byte-both-arguments", "both Bytes parameters are rewritten", "ignore-last-byte", "cmp2",
           ["0a0b", "0c0d"], True, {"args": ["0a00", "0c00"]})
fault_case("ignore-last-byte-empty-argument", "an empty argument stays empty (the length guard); a one-byte one becomes 00",
           "ignore-last-byte", "cmp2", ["", "0c"], True, {"args": ["", "00"]})
fault_case("ignore-last-byte-one-parameter", "an op with one Bytes parameter", "ignore-last-byte", "echo",
           ["aabbcc"], "aabbcc", {"args": ["aabb00"]})

# -- hand-written rows -----------------------------------------------------------------------------------------------------
SWAP = {"fault": "swap-operands", "op": "swap_op", "kind": "pre", "requires": CORE,
        "rewrites": [{"param": "a", "expr": v("b")}, {"param": "b", "expr": v("a")}]}
fault_case("pre-swaps-simultaneously", "a and b are exchanged: every rewrite reads the arguments as passed in",
           SWAP, "swap_op", ["3", "4"], "7", {"args": ["4", "3"]})
SHIFT = {"fault": "shift-operands", "op": "swap_op", "kind": "pre", "requires": CORE,
         "rewrites": [{"param": "a", "expr": v("b")},
                      {"param": "b", "expr": ["add", v("a"), ["lit", {"type": "U8", "value": "1"}]]}]}
fault_case("pre-rewrites-read-the-original-arguments", "b := a + 1 reads the passed-in a, not the rewritten one",
           SHIFT, "swap_op", ["3", "4"], "7", {"args": ["4", "4"]})
ONE = {"fault": "rewrite-first", "op": "swap_op", "kind": "pre", "requires": CORE,
       "rewrites": [{"param": "a", "expr": ["add", v("a"), v("b")]}]}
fault_case("pre-unchanged-arguments-are-reported", "an argument with no rewrite is reported as passed in", ONE, "swap_op",
           ["3", "4"], "7", {"args": ["7", "4"]})
CONST = {"fault": "always-true", "op": "cmp2", "kind": "replace", "requires": CORE, "expr": ["lit", True]}
fault_case("replace-ignores-the-result", "a replace fault supplies the result without trusting the call", CONST, "cmp2",
           ["0a", "0b"], False, {"result": True})
FIRST = {"fault": "return-argument", "op": "echo", "kind": "replace", "requires": CORE, "expr": v("a")}
fault_case("replace-returns-an-argument", "replace with a parameter", FIRST, "echo", ["0a"], "ff", {"result": "0a"})
APPEND = {"fault": "append-argument", "op": "echo", "kind": "post", "requires": CORE,
          "expr": ["concat", v("result"), v("a")]}
fault_case("post-reads-parameters-and-result", "a post fault sees the parameters and result", APPEND, "echo", ["0a"], "0b",
           {"result": "0b0a"})

# -- level-missing: the row needs a level that levels lacks ------------------------------------------------------------------------
fault_case("level-missing-xor-lsb", "xor-lsb is an ir.fixed template and levels lacks ir.fixed", "xor-lsb", "mul8",
           ["2", "3"], "6", {"error": "level-missing"})
fault_case("level-missing-ignore-high-bit", "ignore-high-bit (a pre fault) needs ir.fixed", "ignore-high-bit", "wide",
           ["1"], "1", {"error": "level-missing"})
fault_case("level-missing-flip-result-bit", "flip-result-bit on Bytes needs ir.fixed", "flip-result-bit", "echo",
           ["0a"], "0b", {"error": "level-missing"})
fault_case("level-missing-ok-form-needs-tuple", "the Ok form of drop-last-result-byte needs ir.tuple for its match",
           "drop-last-result-byte", "pick", ["0a", "0b", True], {"ok": "0a"}, {"error": "level-missing"}, types=ERR)
FIXED_ROW = {"fault": "bit-and-one", "op": "swap_op", "kind": "post", "requires": ["ir.core", "ir.fixed"],
             "expr": ["bit_and", v("result"), ["lit", {"type": "U8", "value": "1"}]]}
fault_case("level-missing-hand-written-fixed-row", "a hand-written row that names ir.fixed", FIXED_ROW, "swap_op",
           ["1", "2"], "3", {"error": "level-missing"})

# -- subject-forbidden: a fault expression contains a subject_op -------------------------------------------------------------------------
CALLS = {"fault": "call-subject", "op": "mul8", "kind": "post", "requires": CORE,
         "expr": ["subject_op", "mul8", [v("a"), v("b")]]}
fault_case("subject-forbidden-post", "a post expression that calls the subject", CALLS, "mul8", ["2", "3"], "6",
           {"error": "subject-forbidden"})
CALLS_PRE = {"fault": "call-subject-pre", "op": "mul8", "kind": "pre", "requires": CORE,
             "rewrites": [{"param": "a", "expr": ["subject_op", "mul8", [v("a"), v("b")]]}]}
fault_case("subject-forbidden-pre", "a pre rewrite that calls the subject", CALLS_PRE, "mul8", ["2", "3"], "6",
           {"error": "subject-forbidden"})
CALLS_REPLACE = {"fault": "call-subject-replace", "op": "cmp2", "kind": "replace", "requires": CORE,
                 "expr": ["eq", ["subject_op", "cmp2", [v("a"), v("b")]], ["lit", True]]}
fault_case("subject-forbidden-nested-in-replace", "a replace expression with a subject call below an eq",
           CALLS_REPLACE, "cmp2", ["0a", "0b"], False, {"error": "subject-forbidden"})

# -- S5e (VER04r section 16.18 R70, R94): an op with width_params, and a fault that changes a length ---------------------------
# A fault row has no call site and no widths: it is typed with each named width unstated (a parameter or result of type
# Bytes<N> is plain Bytes), args and result are decoded as Bytes values, and each rewrite needs a *common type* (R51
# unify, not assignability) with the parameter or result type it replaces.
OPS["eq_fixed"] = {"params": [["a", "Bytes<N>"], ["b", "Bytes<N>"]], "ret": "Bool", "requires": CORE, "width_params": ["N"]}
OPS["echo_fixed"] = {"params": [["a", "Bytes<N>"]], "ret": "Bytes<N>", "requires": CORE, "width_params": ["N"]}
OPS["pair2"] = op([("a", "Bytes")], "Bytes<2>")

fault_case("width-params-ignore-last-byte", "the pre fault of eq_fixed<N>: both Bytes<N> parameters are plain Bytes here",
           "ignore-last-byte", "eq_fixed", ["0a0b", "0c0d"], True, {"args": ["0a00", "0c00"]})
fault_case("width-params-invert", "a post fault on an op with width_params leaves the result type alone", "invert",
           "eq_fixed", ["0a0b", "0a0b"], True, {"result": False})
fault_case("width-params-result-length-changes", "drop-last-result-byte on a Bytes<N> result gives plain Bytes: a fault may "
           "change a length by design (R94)", "drop-last-result-byte", "echo_fixed", ["0a0b"], "0a0b", {"result": "0a"})
fault_case("width-params-truncate-last", "truncate-last on a Bytes<N> result", "truncate-last", "echo_fixed", ["0a0b"],
           "0a0b", {"result": "0a"})
TAKE2 = {"fault": "take-two", "op": "pair2", "kind": "post", "requires": CORE,
         "expr": ["take", v("result"), ["lit", {"type": "Nat", "value": "2"}]]}
fault_case("common-type-not-flow-take-of-a-fixed-result", "take(result, 2) of a Bytes<2> result is plain Bytes: it unifies with "
           "Bytes<2> (R51) and does not flow to it; the row is accepted and the result is unchanged (R94)", TAKE2, "pair2",
           ["0a"], "0a0b", {"result": "0a0b"})


# ===============================================================================================
# S10d (VER04r section 13, R67, R96, R98, R110, R119 to R121): the faults at ``ir.fixed`` and ``ir.tuple``
# ===============================================================================================
# Each case states the ``requires`` it expects: for an accepted case it is the row's own levels united with ``ir.core`` (the
# row's ``requires`` is checked against what its tags and the op's types use); for ``level-missing`` and ``unknown-tag`` it is
# the case's ``levels`` (R24, R119 (e)).  A lane that implements ``ir.core`` alone skips the ones that name ``ir.fixed`` or
# ``ir.tuple`` and runs the rest.
FX = ["ir.core", "ir.fixed"]
TU = ["ir.core", "ir.tuple"]
FT = ["ir.core", "ir.fixed", "ir.tuple"]
OPS["neg8"] = op([("a", "I8")], "I8")
OPS["res_u8"] = op([("a", "U8")], "Result<U8,E>")
OPS["dec"] = op([("a", "Bytes")], "Result<(Bytes,Nat),E>")
OPS["dec_u8"] = op([("a", "Bytes")], "Result<(Bytes,U8),E>")
OPS["dec_i8"] = op([("a", "Bytes")], "Result<(Bytes,I8),E>")
OPS["pair_n"] = op([("a", "Bytes")], "(Bytes,Nat)")
OPS["pair_u8"] = op([("a", "Bytes")], "(Bytes,U8)")
OPS["swap_pair"] = op([("a", "U8")], "(U8,U8)")
OPS["swap_res"] = op([("a", "U8")], "Result<(U8,U8),E>")
OPS["tcmp"] = op([("t", "(U8,Bool)")], "Bool")
OPS["tswap"] = op([("t", "(U8,U8)")], "U8")
OPS["mk_res"] = op([("a", "U8")], "Result<(Bytes,Bool),E>")
OPS["mk_res_bytes2"] = op([("a", "Bytes<2>"), ("flag", "Bool")], "Result<(Bytes,Bool),E>")
OPS["mk_res_bytes"] = op([("a", "Bytes")], "Result<(Bytes,Bool),E>")
OPS["rec2"] = op([("a", "Bytes")], "(Bytes<2>,Bool)")
OPS["tres"] = op([("a", "U8")], "(Result<U8,E>,Bool)")
Z = ["lit", {"type": "Nat", "value": "0"}]
CASEWRAP = dict(types=ERR)

# -- ir.fixed templates, accepted at levels ir.core and ir.fixed --------------------------------------------------------------------
fault_case("xor-lsb-u8", "xor-lsb on a U8 result: bit_xor(result, 1), 6 becomes 7", "xor-lsb", "mul8", ["2", "3"], "6", {"result": "7"},
           levels=FX, requires=FX)
fault_case("xor-lsb-u8-odd-to-even", "255 becomes 254", "xor-lsb", "mul8", ["2", "3"], "255", {"result": "254"}, levels=FX, requires=FX)
fault_case("xor-lsb-i8-negative", "-1 (the pattern ff) becomes -2 (fe)", "xor-lsb", "neg8", ["1"], "-1", {"result": "-2"}, levels=FX,
           requires=FX)
fault_case("xor-lsb-u64", "xor-lsb on a U64 result: the maximum becomes the maximum minus one", "xor-lsb", "wide", ["1"],
           str(2**64 - 1), {"result": str(2**64 - 2)}, levels=FX, requires=FX)
fault_case("ignore-high-bit-clears-bit-63", "ignore-high-bit, a pre fault on each U64 parameter: MAX & (2^63-1)", "ignore-high-bit", "wide",
           [str(2**64 - 1)], "1", {"args": [str(2**63 - 1)]}, levels=FX, requires=FX)
fault_case("ignore-high-bit-low-value-unchanged", "a value without bit 63 is reported unchanged", "ignore-high-bit", "wide", ["5"], "1",
           {"args": ["5"]}, levels=FX, requires=FX)
fault_case("ignore-high-bit-exactly-bit-63", "2^63 becomes 0", "ignore-high-bit", "wide", [str(2**63)], "1", {"args": ["0"]}, levels=FX,
           requires=FX)
fault_case("flip-result-bit-u8", "flip-result-bit on a fixed-width result is the xor-lsb form", "flip-result-bit", "mul8", ["2", "3"], "6",
           {"result": "7"}, levels=FX, requires=FX)
fault_case("flip-result-bit-bytes", "flip-result-bit on Bytes: the low bit of the first byte flips and the length stays", "flip-result-bit",
           "echo", ["0a"], "0a0b", {"result": "0b0b"}, levels=FX, requires=FX)
fault_case("flip-result-bit-bytes-ff", "ff becomes fe", "flip-result-bit", "echo", ["0a"], "ff", {"result": "fe"}, levels=FX, requires=FX)
fault_case("flip-result-bit-empty-bytes", "the empty result becomes the byte 01", "flip-result-bit", "echo", ["0a"], "", {"result": "01"},
           levels=FX, requires=FX)
fault_case("flip-result-bit-sized-bytes", "flip-result-bit on a Bytes<N> result: one template, typed plain Bytes", "flip-result-bit",
           "echo_fixed", ["0a0b"], "0a0b", {"result": "0b0b"}, levels=FX, requires=FX)

# -- the Ok form: a match on the Result, so ir.tuple joins ----------------------------------------------------------------------------
fault_case("flip-result-bit-ok-bytes", "flip-result-bit under an Ok flips the payload (ir.fixed and ir.tuple)", "flip-result-bit", "pick",
           ["0a", "0b", True], {"ok": "0a0b"}, {"result": {"ok": "0b0b"}}, levels=FT, types=ERR, requires=FT)
fault_case("flip-result-bit-ok-empty-payload", "an empty Ok payload becomes 01", "flip-result-bit", "pick", ["0a", "0b", True], {"ok": ""},
           {"result": {"ok": "01"}}, levels=FT, types=ERR, requires=FT)
fault_case("flip-result-bit-err-is-unchanged", "an Err is returned as it is (the err_any arm)", "flip-result-bit", "pick",
           ["0a", "0b", True], {"err": "BAD"}, {"result": {"err": "BAD"}}, levels=FT, types=ERR, requires=FT)
fault_case("flip-result-bit-ok-u8", "flip-result-bit on Result<U8,E>: the payload's low bit", "flip-result-bit", "res_u8", ["1"],
           {"ok": "6"}, {"result": {"ok": "7"}}, levels=FT, types=ERR, requires=FT)
fault_case("flip-result-bit-ok-u8-err-is-unchanged", "an Err of a Result<U8,E>", "flip-result-bit", "res_u8", ["1"], {"err": "WORSE"},
           {"result": {"err": "WORSE"}}, levels=FT, types=ERR, requires=FT)
fault_case("drop-last-result-byte-ok-form", "the Ok form of drop-last-result-byte (core and ir.tuple): the last payload byte goes",
           "drop-last-result-byte", "pick", ["0a", "0b", True], {"ok": "0a0b0c"}, {"result": {"ok": "0a0b"}}, levels=TU, types=ERR,
           requires=TU)
fault_case("drop-last-result-byte-ok-empty", "an empty Ok payload becomes the byte 00", "drop-last-result-byte", "pick",
           ["0a", "0b", True], {"ok": ""}, {"result": {"ok": "00"}}, levels=TU, types=ERR, requires=TU)
fault_case("drop-last-result-byte-ok-err-unchanged", "an Err stays", "drop-last-result-byte", "pick", ["0a", "0b", True], {"err": "BAD"},
           {"result": {"err": "BAD"}}, levels=TU, types=ERR, requires=TU)

# -- ir.tuple composites: ok-as-err, drop-consumed, swap-components ---------------------------------------------------------------------
fault_case("ok-as-err-first-case", "ok-as-err reports a success as the first error case of E", "ok-as-err", "swap_res", ["1"],
           {"ok": ["1", "2"]}, {"result": {"err": "BAD"}}, levels=TU, types=ERR, requires=TU)
fault_case("ok-as-err-leaves-an-err", "an Err is unchanged", "ok-as-err", "swap_res", ["1"], {"err": "WORSE"}, {"result": {"err": "WORSE"}},
           levels=TU, types=ERR, requires=TU)
fault_case("drop-consumed-in-an-ok", "drop-consumed zeroes the second component of the tuple inside an Ok", "drop-consumed", "dec", ["0a"],
           {"ok": ["0a", "5"]}, {"result": {"ok": ["0a", "0"]}}, levels=TU, types=ERR, requires=TU)
fault_case("drop-consumed-leaves-an-err", "an Err is unchanged", "drop-consumed", "dec", ["0a"], {"err": "BAD"}, {"result": {"err": "BAD"}},
           levels=TU, types=ERR, requires=TU)
fault_case("drop-consumed-bare-tuple", "drop-consumed on a bare tuple result", "drop-consumed", "pair_n", ["0a"], ["0a", "9"],
           {"result": ["0a", "0"]}, levels=TU, requires=TU)
fault_case("drop-consumed-fixed-width-component", "a U8 second component is zeroed with a U8 literal", "drop-consumed", "pair_u8", ["0a"],
           ["0a", "200"], {"result": ["0a", "0"]}, levels=TU, requires=TU)
fault_case("swap-components-bare-pair", "swap-components exchanges the components of a homogeneous pair", "swap-components", "swap_pair",
           ["1"], ["1", "2"], {"result": ["2", "1"]}, levels=TU, requires=TU)
fault_case("swap-components-in-an-ok", "swap-components under an Ok", "swap-components", "swap_res", ["1"], {"ok": ["1", "2"]},
           {"result": {"ok": ["2", "1"]}}, levels=TU, types=ERR, requires=TU)

# -- off-by-one-consumed (R96): ir.fixed and ir.tuple, in all four combinations of a bare tuple or an Ok and a Nat or a fixed-width component -
fault_case("off-by-one-consumed-in-an-ok-even-nat", "an even Nat second component goes up: Result<(Bytes,Nat),E> accepted (R96): 4 becomes 5",
           "off-by-one-consumed", "dec", ["0a"], {"ok": ["0a", "4"]}, {"result": {"ok": ["0a", "5"]}}, levels=FT, types=ERR, requires=FT)
fault_case("off-by-one-consumed-in-an-ok-odd-nat", "an odd Nat goes down: 5 becomes 4", "off-by-one-consumed", "dec", ["0a"],
           {"ok": ["0a", "5"]}, {"result": {"ok": ["0a", "4"]}}, levels=FT, types=ERR, requires=FT)
fault_case("off-by-one-consumed-zero-goes-to-one", "0 is even and goes to 1", "off-by-one-consumed", "dec", ["0a"], {"ok": ["0a", "0"]},
           {"result": {"ok": ["0a", "1"]}}, levels=FT, types=ERR, requires=FT)
fault_case("off-by-one-consumed-largest-nat-goes-down", "the largest Nat is odd and goes down, so the value never leaves its type",
           "off-by-one-consumed", "dec", ["0a"], {"ok": ["0a", str(2**64 - 1)]}, {"result": {"ok": ["0a", str(2**64 - 2)]}}, levels=FT,
           types=ERR, requires=FT)
fault_case("off-by-one-consumed-leaves-an-err", "an Err is unchanged", "off-by-one-consumed", "dec", ["0a"], {"err": "WORSE"},
           {"result": {"err": "WORSE"}}, levels=FT, types=ERR, requires=FT)
fault_case("off-by-one-consumed-bare-tuple", "a bare (Bytes,Nat) result: the same rule on the tuple itself", "off-by-one-consumed", "pair_n",
           ["0a"], ["0a", "7"], {"result": ["0a", "6"]}, levels=FT, requires=FT)
fault_case("off-by-one-consumed-u8-component-even", "a fixed-width component is xor 1: 254 becomes 255", "off-by-one-consumed", "pair_u8",
           ["0a"], ["0a", "254"], {"result": ["0a", "255"]}, levels=FT, requires=FT)
fault_case("off-by-one-consumed-u8-component-odd-maximum", "255 is odd and becomes 254", "off-by-one-consumed", "pair_u8", ["0a"],
           ["0a", "255"], {"result": ["0a", "254"]}, levels=FT, requires=FT)
fault_case("off-by-one-consumed-signed-component-minimum", "the signed minimum -128 is even and goes up to -127", "off-by-one-consumed",
           "dec_i8", ["0a"], {"ok": ["0a", "-128"]}, {"result": {"ok": ["0a", "-127"]}}, levels=FT, types=ERR, requires=FT)
fault_case("off-by-one-consumed-signed-component-minus-one", "-1 is odd (the pattern ff) and goes down to -2", "off-by-one-consumed",
           "dec_i8", ["0a"], {"ok": ["0a", "-1"]}, {"result": {"ok": ["0a", "-2"]}}, levels=FT, types=ERR, requires=FT)

# -- missing-error with a tuple payload (R67): the plain Bytes component has no parameter of its type -------------------------------------
fault_case("missing-error-tuple-payload-no-bytes-parameter", "Result<(Bytes,Bool),E> and no parameter of type Bytes or Bool: the payload is "
           "the tuple of two zero literals, an empty Bytes (a plain Bytes, R67) and false", "missing-error", "mk_res", ["1"],
           {"err": "BAD"}, {"result": {"ok": ["", False]}}, levels=TU, types=ERR, requires=TU)
fault_case("missing-error-tuple-payload-an-ok-is-unchanged", "the same fault over an Ok result returns it unchanged", "missing-error", "mk_res",
           ["1"], {"ok": ["0a", True]}, {"result": {"ok": ["0a", True]}}, levels=TU, types=ERR, requires=TU)
fault_case("missing-error-tuple-payload-sized-bytes-parameter-is-not-a-match", "a Bytes<2> parameter is not a parameter of exactly the type "
           "Bytes: the component is the zero literal, and the Bool component is the parameter flag", "missing-error", "mk_res_bytes2",
           ["0a0b", True], {"err": "WORSE"}, {"result": {"ok": ["", True]}}, levels=TU, types=ERR, requires=TU)
fault_case("missing-error-tuple-payload-bytes-parameter", "a parameter of type Bytes is the Bytes component", "missing-error", "mk_res_bytes",
           ["0a0b"], {"err": "BAD"}, {"result": {"ok": ["0a0b", False]}}, levels=TU, types=ERR, requires=TU)

# -- a tuple parameter and a tuple result, with hand-written rows ------------------------------------------------------------------------------
fault_case("invert-on-an-op-with-a-tuple-parameter", "invert over an op whose parameter is a tuple: the row needs ir.core and ir.tuple",
           "invert", "tcmp", [["1", True]], True, {"result": False}, levels=TU, requires=TU)
SWAP_T = {"fault": "swap-pair-argument", "op": "tswap", "kind": "pre", "requires": TU,
          "rewrites": [{"param": "t", "expr": ["tuple", [["project", v("t"), 1], ["project", v("t"), 0]]]}]}
fault_case("pre-rewrites-a-tuple-parameter", "a pre fault whose rewrite builds a tuple from the parameter's components: the argument is "
           "reported as a tuple", SWAP_T, "tswap", [["1", "2"]], "3", {"args": [["2", "1"]]}, levels=TU, requires=TU)
POST_T = {"fault": "negate-flag", "op": "tcmp", "kind": "replace", "requires": TU, "expr": ["not", ["project", v("t"), 1]]}
fault_case("replace-reads-a-component-of-a-tuple-argument", "a replace expression that projects a tuple argument", POST_T, "tcmp",
           [["1", True]], True, {"result": False}, levels=TU, requires=TU)
FIXED_POST = {"fault": "mask-low-nibble", "op": "mul8", "kind": "post", "requires": FX,
              "expr": ["bit_and", v("result"), ["lit", {"type": "U8", "value": "15"}]]}
fault_case("post-with-an-ir-fixed-tag", "a hand-written post fault with bit_and on a U8 result: 255 & 15", FIXED_POST, "mul8", ["2", "3"],
           "255", {"result": "15"}, levels=FX, requires=FX)

# -- level-missing: decided from the row, requires is the case's levels (R24) -----------------------------------------------------------------
fault_case("level-missing-flip-result-bit-ok-form-without-tuple", "the Ok form of flip-result-bit needs ir.fixed and ir.tuple: levels has "
           "ir.fixed only", "flip-result-bit", "pick", ["0a", "0b", True], {"ok": "0a"}, {"error": "level-missing"}, levels=FX, types=ERR,
           requires=FX)
fault_case("level-missing-off-by-one-consumed-without-fixed", "off-by-one-consumed needs ir.fixed too: levels has ir.tuple only",
           "off-by-one-consumed", "dec", ["0a"], {"ok": ["0a", "4"]}, {"error": "level-missing"}, levels=TU, types=ERR, requires=TU)
fault_case("level-missing-off-by-one-consumed-without-tuple", "off-by-one-consumed with ir.fixed only", "off-by-one-consumed", "dec", ["0a"],
           {"ok": ["0a", "4"]}, {"error": "level-missing"}, levels=FX, types=ERR, requires=FX)
fault_case("level-missing-drop-consumed-without-tuple", "drop-consumed is an ir.tuple template", "drop-consumed", "pair_n", ["0a"],
           ["0a", "9"], {"error": "level-missing"}, levels=FX, requires=FX)
fault_case("level-missing-ok-as-err-without-tuple", "ok-as-err is an ir.tuple template", "ok-as-err", "swap_res", ["1"], {"ok": ["1", "2"]},
           {"error": "level-missing"}, levels=FX, types=ERR, requires=FX)
fault_case("level-missing-swap-components-without-tuple", "swap-components is an ir.tuple template", "swap-components", "swap_pair", ["1"],
           ["1", "2"], {"error": "level-missing"}, levels=FX, requires=FX)
fault_case("level-missing-invert-on-a-tuple-parameter", "invert is a core template, but the op's parameter is a tuple type, which is ir.tuple "
           "(R69 b): levels has ir.fixed only", "invert", "tcmp", [["1", True]], True, {"error": "level-missing"}, levels=FX, requires=FX)
fault_case("level-missing-missing-error-tuple-payload-at-core", "the tuple payload of missing-error is ir.tuple: at levels ir.core the case is "
           "level-missing, and its requires is ir.core alone, so a lane that implements ir.core runs it", "missing-error", "mk_res", ["1"],
           {"err": "BAD"}, {"error": "level-missing"}, levels=CORE, types=ERR, requires=CORE)

# -- ill-typed ([D74], R110, R96 to R98): a row with no common type, or ill-typed as a tree ------------------------------------------------
R67_LIT = {"fault": "one-byte-literal", "op": "rec2", "kind": "post", "requires": TU,
           "expr": ["tuple", [["lit", {"type": "Bytes", "value": "00"}], ["lit", True]]]}
fault_case("ill-typed-r67-one-byte-literal-in-a-bytes-2-component", "R96's second vector: a one-byte literal is a Bytes<1>, and in a tuple "
           "component it has no common type with the declared Bytes<2> (a tuple is invariant)", R67_LIT, "rec2", ["0a"], ["0a0b", True],
           {"error": "ill-typed"}, levels=TU, requires=TU)
R98_IF = {"fault": "if-with-a-wrong-branch", "op": "res_u8", "kind": "post", "requires": CORE,
          "expr": ["if", ["is_err", v("result")], v("result"), ["ok", ["lit", True]]]}
fault_case("ill-typed-r98-if-branch-ok-true-against-a-u8-result", "R98: an if whose other branch is ok(true), against a Result<U8,E> "
           "result: each branch is compared, so the Bool payload has no common type with U8", R98_IF, "res_u8", ["1"], {"ok": "6"},
           {"error": "ill-typed"}, types=ERR, requires=CORE)
R98_MATCH = {"fault": "match-with-a-wrong-arm", "op": "res_u8", "kind": "post", "requires": TU,
             "expr": ["match", v("result"), [[["ok", ["bind", "p"]], ["ok", v("p")]], [["err_any"], ["ok", ["lit", True]]]]]}
fault_case("ill-typed-r98-wrong-second-match-arm", "R98: the second arm of a match returns ok(true) where a Result<U8,E> is declared",
           R98_MATCH, "res_u8", ["1"], {"ok": "6"}, {"error": "ill-typed"}, levels=TU, types=ERR, requires=TU)
R98_COMPONENT = {"fault": "tuple-component-if", "op": "tres", "kind": "post", "requires": TU,
                 "expr": ["tuple", [["if", ["is_err", ["project", v("result"), 0]], ["project", v("result"), 0], ["ok", ["lit", True]]],
                                    ["project", v("result"), 1]]]}
fault_case("ill-typed-r98-tuple-component-if", "R98: an if in a tuple component whose other branch is ok(true), against a declared "
           "Result<U8,E> component", R98_COMPONENT, "tres", ["1"], [{"ok": "6"}, True], {"error": "ill-typed"}, levels=TU, types=ERR,
           requires=TU)
fault_case("ill-typed-not-of-a-u8-result", "a fault expression that is ill-typed as a tree: not of a U8 (R110)",
           {"fault": "not-of-u8", "op": "mul8", "kind": "post", "requires": CORE, "expr": ["not", v("result")]}, "mul8", ["2", "3"], "6",
           {"error": "ill-typed"}, requires=CORE)
fault_case("ill-typed-bool-for-a-u8-result", "no common type: a Bool expression against a U8 result (R94)",
           {"fault": "bool-for-u8", "op": "mul8", "kind": "post", "requires": CORE, "expr": ["lit", True]}, "mul8", ["2", "3"], "6",
           {"error": "ill-typed"}, requires=CORE)
fault_case("ill-typed-bytes-for-a-u8-result", "no common type: a Bytes expression against a U8 result (R94)",
           {"fault": "bytes-for-u8", "op": "mul8", "kind": "post", "requires": CORE, "expr": ["lit", {"type": "Bytes", "value": "00"}]}, "mul8",
           ["2", "3"], "6", {"error": "ill-typed"}, requires=CORE)
fault_case("ill-typed-bool-for-a-bytes-parameter", "no common type: a pre rewrite of a Bytes parameter with a Bool (R94)",
           {"fault": "bool-for-bytes", "op": "echo", "kind": "pre", "requires": CORE, "rewrites": [{"param": "a", "expr": ["lit", True]}]},
           "echo", ["0a"], "0a", {"error": "ill-typed"}, requires=CORE)
fault_case("ill-typed-tuple-for-a-scalar-result", "a tuple expression against a U8 result", {"fault": "tuple-for-u8", "op": "mul8", "kind": "post",
           "requires": TU, "expr": ["tuple", [v("a"), v("b")]]}, "mul8", ["2", "3"], "6", {"error": "ill-typed"}, levels=TU, requires=TU)

# -- R121 (c): the op row is read with the table: an Int, an error type or an undeclared type in it is ill-typed -----------------------------
BAD_OP_POST = {"fault": "echo-back", "op": "o", "kind": "post", "requires": CORE, "expr": v("result")}


def op_row_case(slug, summary, params, ret, types=None):
    ROWS.append(row(slug, summary, {"fault": BAD_OP_POST, "op": {"params": params, "ret": ret, "requires": CORE}, "types": types or {},
                                    "levels": CORE, "args": ["1"] * len(params), "result": "1"}, {"error": "ill-typed"}, CORE))


op_row_case("ill-typed-op-row-int-parameter", "an op parameter of type Int: Int is never a value type (R121)", [["a", "Int"]], "U8")
op_row_case("ill-typed-op-row-int-result", "an op result of type Int", [["a", "U8"]], "Int")
op_row_case("ill-typed-op-row-error-type-as-a-parameter", "an error type outside the second argument of a Result", [["a", "E"]], "U8", ERR)
op_row_case("ill-typed-op-row-undeclared-type", "a parameter type that types does not declare", [["a", "Frob"]], "U8")

# -- R119 (e): the three static codes of v2-fault: unknown-tag, depth and model-order ------------------------------------------------------
def post_row(name, expr):
    return {"fault": name, "op": "echo", "kind": "post", "requires": CORE, "expr": expr}


def nest_concat(n):
    expr = v("a")
    for _ in range(n):
        expr = ["concat", expr, v("a")]
    return expr


fault_case("unknown-tag-in-a-post-expression", "a tag of no level this document defines in a fault expression: unknown-tag, and its requires "
           "is the case's levels (R119 e)", post_row("bogus-tag", ["bogus", v("a")]), "echo", ["0a"], "0a", {"error": "unknown-tag"}, requires=CORE)
fault_case("unknown-tag-in-a-pre-rewrite", "the same tag in the expression of a pre rewrite", {"fault": "bogus-tag-pre", "op": "echo", "kind": "pre",
           "requires": CORE, "rewrites": [{"param": "a", "expr": ["bogus", v("a")]}]}, "echo", ["0a"], "0a", {"error": "unknown-tag"}, requires=CORE)
fault_case("depth-65-nodes-is-refused", "64 nested concats over a var is an expression 65 deep: depth (section 14)", post_row("deep", nest_concat(64)),
           "echo", ["0a"], "0a", {"error": "depth"}, requires=CORE)
fault_case("depth-64-nodes-is-accepted", "63 nested concats over a var is 64 deep and accepted: the root is level 1; the result is the argument "
           "64 times", post_row("deep-ok", nest_concat(63)), "echo", ["0a"], "0b", {"result": "0a" * 64}, requires=CORE)
fault_case("model-order-a-model-call-in-a-fault-expression", "a v2-fault case has no models, so every model_call names a model missing from "
           "models and model_order (R119 a)", post_row("calls-a-model", ["model_call", "m", [v("a")]]), "echo", ["0a"], "0a",
           {"error": "model-order"}, requires=CORE)
