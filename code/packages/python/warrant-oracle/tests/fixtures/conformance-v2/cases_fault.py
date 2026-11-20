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
    return {"params": [list(p) for p in params], "ret": ret, "requires": CORE}


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


def fault_case(slug, summary, fault, op_name, args, result, intent, *, levels=("ir.core",), types=None):
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
    ROWS.append(row(slug, summary, fields, intent))


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
