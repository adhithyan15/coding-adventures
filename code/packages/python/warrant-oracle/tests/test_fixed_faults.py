"""Slice S10c-1: the ``ir.fixed`` fault templates and the fault-evaluation gate (VER04r 8.2, D27, D29).

The templates (``xor-lsb``, ``ignore-high-bit``, ``flip-result-bit``, ``drop-last-result-byte`` and
the shipped ``truncate-last``) are instantiated as data and written exactly as section 8.2 prints
them; since this slice the evaluator can run them.  Since the atomic flip (S10c, D29) the shipped gate
``IMPLEMENTED`` is open to ``ir.fixed`` and ``ir.tuple``; the closed gate ``("ir.core",)`` stays
reachable through the library parameter, and a control whose fault needs ``ir.fixed`` then records
``deferred`` witnesses.  The tests check both gates and the witness search against a brute-force
search.
"""

from __future__ import annotations

import pytest
from conftest import SPECS, adopter_bytes

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.corpus_v2 import derive
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.faults import IMPLEMENTED, REGISTRY, fault_row
from coding_adventures_warrant_oracle.ir_eval import Program
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parity_dumps import GATE_OPEN
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.suites import SuitePlan

GEN = {"name": "test", "version": "0"}
TYPES = ["U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64"]

DOC = """warrant f v1
  error E = BAD | WORSE
  op fx8(a: U8) -> U8
  op fx64(a: I64) -> I64
  op raw(d: Bytes) -> Bytes
  op rawn(d: Bytes<3>) -> Bytes<3>
  op res_b(d: Bytes) -> Result<Bytes,E>
  op res_n(d: Bytes<2>) -> Result<Bytes<2>,E>
  op res_u(a: U8) -> Result<U8,E>
  op eq64(a: U64, b: U64) -> Bool
  op flag(a: Bool) -> Bool
  claim c_fx8:
    forall a: U8. fx8(a) == a
  claim c_fx64:
    forall a: I64. fx64(a) == a
  claim c_raw:
    forall d: Bytes. raw(d) == d
  claim c_rawn:
    forall d: Bytes<3>. rawn(d) == d
  claim c_res_b:
    forall d: Bytes. res_b(d) == (if len(d) == 2 then Ok(d) else Err(E.BAD))
  claim c_res_n:
    forall d: Bytes<2>. res_n(d) == Ok(d)
  claim c_res_u:
    forall a: U8. res_u(a) == Ok(a)
  claim c_eq64:
    forall a: U64, b: U64. eq64(a, b) == (a == b)
  claim c_flag:
    forall a: Bool. flag(a) == a
"""


def spec():
    return lower_document(parse_source(DOC.encode()))


def lit(ty: str, value: object) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


R = ["var", "result"]
N0, N1 = lit("Nat", 0), lit("Nat", 1)
U0, U1 = lit("U8", 0), lit("U8", 1)
B01, B00 = lit("Bytes", "01"), lit("Bytes", "00")
GT = ["gt", ["len", R], N0]
FLIP_BYTES = [
    "if",
    GT,
    ["concat", ["be_bytes", ["bit_xor", ["unwrap_or", ["at", R, N0], U0], U1]], ["drop", R, N1]],
    B01,
]
DROP_BYTES = ["if", GT, ["take", R, ["sub", ["len", R], N1]], B00]


def with_payload(expr: object) -> object:
    """``expr`` with ``["var","ok_payload"]`` in place of ``["var","result"]`` throughout."""
    if isinstance(expr, list):
        if expr == ["var", "result"]:
            return ["var", "ok_payload"]
        return [with_payload(x) for x in expr]
    return expr


def ok_form(expr: list) -> list:
    return [
        "match",
        R,
        [[["ok", ["bind", "ok_payload"]], ["ok", with_payload(expr)]], [["err_any"], R]],
    ]


# -- the JSON of section 8.2, byte for byte ------------------------------------------------------------


@pytest.mark.parametrize("ty", TYPES)
def test_xor_lsb_and_flip_on_a_fixed_width_result_are_the_bit_xor_form(ty):
    doc = f"warrant f v1\n  op f(a: {ty}) -> {ty}\n  claim c:\n    forall a: {ty}. f(a) == a\n"
    ops = lower_document(parse_source(doc.encode())).ops
    expected = ["bit_xor", R, lit(ty, 1)]
    for name in ("xor-lsb", "flip-result-bit"):
        row = fault_row(ops["f"], name)
        assert row == {"fault": name, "op": "f", "kind": "post", "requires": ["ir.fixed"], "expr": expected}


def test_ignore_high_bit_rewrites_every_u64_parameter_with_the_ct_compare_shape():
    ops = lower_document(
        parse_source(b"warrant f v1\n  op f(a: U64, n: Nat, b: U64) -> Bool\n  claim c:\n    forall a: U64, n: Nat, b: U64. f(a, n, b) == true\n")
    ).ops
    mask = lit("U64", (1 << 63) - 1)
    assert fault_row(ops["f"], "ignore-high-bit") == {
        "fault": "ignore-high-bit",
        "op": "f",
        "kind": "pre",
        "requires": ["ir.fixed"],
        "rewrites": [
            {"param": "a", "expr": ["bit_and", ["var", "a"], mask]},
            {"param": "b", "expr": ["bit_and", ["var", "b"], mask]},
        ],
    }
    # the mask is 2**63 - 1, which is the U64 literal 9223372036854775807 of the committed ct-compare row
    assert mask == ["lit", {"type": "U64", "value": "9223372036854775807"}]


def test_flip_and_drop_on_bytes_and_bytes_n_are_one_template_each():
    ops = spec().ops
    for op in ("raw", "rawn"):  # Bytes and Bytes<N>: the same JSON, the if wrapper whatever N
        assert fault_row(ops[op], "flip-result-bit")["expr"] == FLIP_BYTES
        assert fault_row(ops[op], "flip-result-bit")["requires"] == ["ir.fixed"]
        assert fault_row(ops[op], "drop-last-result-byte")["expr"] == DROP_BYTES
        assert fault_row(ops[op], "drop-last-result-byte")["requires"] == ["ir.core"]
        assert fault_row(ops[op], "truncate-last")["expr"] == [
            "if",
            GT,
            ["take", R, ["sub", ["len", R], N1]],
            R,
        ]


def test_the_ok_forms_wrap_the_row_in_a_match_and_union_the_levels():
    ops, errors = spec().ops, spec().errors
    flip = fault_row(ops["res_b"], "flip-result-bit", errors)
    assert flip["expr"] == ok_form(FLIP_BYTES) and flip["requires"] == ["ir.fixed", "ir.tuple"]
    drop = fault_row(ops["res_n"], "drop-last-result-byte", errors)
    assert drop["expr"] == ok_form(DROP_BYTES) and drop["requires"] == ["ir.core", "ir.tuple"]
    fixed = fault_row(ops["res_u"], "flip-result-bit", errors)
    assert fixed["expr"] == ok_form(["bit_xor", R, U1]) and fixed["requires"] == ["ir.fixed", "ir.tuple"]


@pytest.mark.parametrize(
    "op,fault",
    [
        ("fx8", "drop-last-result-byte"),  # a fixed-width result is not a template
        ("fx8", "truncate-last"),
        ("raw", "xor-lsb"),  # xor-lsb needs a fixed-width result
        ("flag", "flip-result-bit"),
        ("flag", "xor-lsb"),
        ("eq64", "xor-lsb"),
        ("fx8", "ignore-high-bit"),  # no U64 parameter
        ("res_u", "drop-last-result-byte"),  # Result<fixed> has no drop template
    ],
)
def test_a_template_the_signature_cannot_take_is_a_fault_error_never_skipped(op, fault):
    s = spec()
    with pytest.raises(OracleError) as err:
        fault_row(s.ops[op], fault, s.errors)
    assert err.value.code == "generation" and err.value.message.startswith("fault:")


def test_an_ok_payload_parameter_collides_with_the_ok_form_binder():
    ops = lower_document(
        parse_source(
            b"warrant f v1\n  error E = A\n  op f(ok_payload: Bytes) -> Result<Bytes,E>\n"
            b"  claim c:\n    forall ok_payload: Bytes. f(ok_payload) == Ok(ok_payload)\n"
        )
    ).ops
    with pytest.raises(OracleError):
        fault_row(ops["f"], "flip-result-bit")
    assert fault_row(ops["f"], "missing-error")["kind"] == "replace"  # no ok_payload binder there


def test_the_registry_has_the_four_fixed_templates_and_the_older_names():
    assert {"xor-lsb", "ignore-high-bit", "flip-result-bit", "drop-last-result-byte", "truncate-last"} <= set(
        REGISTRY
    )


# -- what the templates compute -----------------------------------------------------------------------------


def evaluate(op: str, fault: str, result: object, **params: object) -> object:
    s = spec()
    row = fault_row(s.ops[op], fault, s.errors)
    prog = Program({}, s.ops, errors=s.errors)
    tenv = {p: t for p, t in s.ops[op].params}
    tenv["result"] = s.ops[op].ret
    return prog.compile(row["expr"], tenv, False).fn({**params, "result": result})


@pytest.mark.parametrize("ty", TYPES)
def test_xor_lsb_flips_the_low_bit_of_every_value_of_the_type(ty):
    bits = int(ty[1:])
    lo, hi = (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if ty[0] == "I" else (0, (1 << bits) - 1)
    doc = f"warrant f v1\n  op f(a: {ty}) -> {ty}\n  claim c:\n    forall a: {ty}. f(a) == a\n"
    s = lower_document(parse_source(doc.encode()))
    row = fault_row(s.ops["f"], "xor-lsb")
    fn = Program({}, s.ops).compile(row["expr"], {"a": A.Ty(ty), "result": A.Ty(ty)}, False).fn
    for v in sorted({lo, lo + 1, -1 if lo < 0 else 2, 0, 1, hi - 1, hi}):
        got = fn({"a": v, "result": v})
        assert got == v ^ 1 and lo <= got <= hi  # for a signed value, Python's ^ on the pattern agrees


def test_flip_result_bit_on_bytes_flips_the_low_bit_of_the_first_byte_and_makes_one_of_nothing():
    assert evaluate("raw", "flip-result-bit", b"") == b"\x01"
    assert evaluate("raw", "flip-result-bit", b"\x00") == b"\x01"
    assert evaluate("raw", "flip-result-bit", b"\x05\xff") == b"\x04\xff"
    assert evaluate("raw", "flip-result-bit", b"\xff\x00\x10") == b"\xfe\x00\x10"
    assert evaluate("rawn", "flip-result-bit", b"\x02\x03\x04") == b"\x03\x03\x04"
    # inside Ok: the payload is flipped, an Err is left alone, and the payload of a Bytes<N> is dynamic
    assert evaluate("res_n", "flip-result-bit", ("ok", b"\x01\x02")) == ("ok", b"\x00\x02")
    assert evaluate("res_b", "flip-result-bit", ("ok", b"")) == ("ok", b"\x01")
    assert evaluate("res_b", "flip-result-bit", ("err", "BAD")) == ("err", "BAD")
    assert evaluate("res_u", "flip-result-bit", ("ok", 4)) == ("ok", 5)
    assert evaluate("res_u", "flip-result-bit", ("err", "WORSE")) == ("err", "WORSE")


def test_drop_last_result_byte_shortens_by_one_and_makes_a_zero_byte_of_nothing():
    assert evaluate("raw", "drop-last-result-byte", b"") == b"\x00"
    assert evaluate("raw", "drop-last-result-byte", b"\x07") == b""
    assert evaluate("rawn", "drop-last-result-byte", b"\x01\x02\x03") == b"\x01\x02"
    assert evaluate("res_n", "drop-last-result-byte", ("ok", b"\x01\x02")) == ("ok", b"\x01")
    assert evaluate("res_b", "drop-last-result-byte", ("ok", b"")) == ("ok", b"\x00")
    assert evaluate("res_b", "drop-last-result-byte", ("err", "BAD")) == ("err", "BAD")


def test_ignore_high_bit_clears_the_top_bit_of_each_u64_parameter():
    s = spec()
    row = fault_row(s.ops["eq64"], "ignore-high-bit")
    tenv = {"a": A.Ty("U64"), "b": A.Ty("U64")}
    prog = Program({}, s.ops)
    fns = {r["param"]: prog.compile(r["expr"], tenv, False).fn for r in row["rewrites"]}
    for v in (0, 1, (1 << 63) - 1, 1 << 63, (1 << 64) - 1):
        assert fns["a"]({"a": v, "b": 0}) == v & ((1 << 63) - 1) == fns["b"]({"a": 0, "b": v})


# -- the gate -----------------------------------------------------------------------------------------------


def controls(implemented: tuple, faults: list[tuple[str, str]]) -> dict:
    profile = "".join(f"fault {op} {fault}\n" for op, fault in faults)
    out = derive(DOC.encode(), profile.encode(), spec_ref="f.warrant", generator=GEN, implemented=implemented)
    return {c["id"].rsplit("/", 2)[-2] + "/" + c["fault"]: c for c in out.corpus["controls"]}


FIXED_FAULTS = [
    ("fx8", "xor-lsb"),
    ("fx8", "flip-result-bit"),
    ("fx64", "flip-result-bit"),
    ("raw", "flip-result-bit"),
    ("rawn", "flip-result-bit"),
    ("res_b", "flip-result-bit"),
    ("res_n", "drop-last-result-byte"),
    ("res_u", "flip-result-bit"),
    ("eq64", "ignore-high-bit"),
    ("raw", "drop-last-result-byte"),
    ("raw", "truncate-last"),
]


def test_the_shipped_gate_is_open_and_the_closed_gate_defers_every_ir_fixed_fault():
    assert IMPLEMENTED == ("ir.core", "ir.fixed", "ir.tuple", "ir.data")  # ir.data opened in S11c-1
    shipped = controls(("ir.core",), FIXED_FAULTS)
    for key, control in shipped.items():
        level = next((lv for lv in control["requires"] if lv != "ir.core"), None)
        assert control["blind_claims"] == []
        (witness,) = control["slim_witness"]
        if level is None:
            assert "chunk" in witness, key  # drop-last-result-byte and truncate-last on Bytes: core
        else:
            # R12: the first level, in sorted order, the gate does not implement
            assert witness == {"claim": witness["claim"], "deferred": level}, key
            assert level == sorted(lv for lv in control["requires"] if lv != "ir.core")[0]


def test_the_open_gate_runs_the_witness_search_and_every_fault_is_seen():
    shipped = controls(("ir.core",), FIXED_FAULTS)
    opened = controls(GATE_OPEN, FIXED_FAULTS)
    assert set(shipped) == set(opened)
    for key, control in opened.items():
        assert control["blind_claims"] == [], key
        assert all("chunk" in w and w["chunk"] == 0 for w in control["slim_witness"]), key
        # the control row itself never depends on the gate
        assert control["requires"] == shipped[key]["requires"]
        assert control["required_claims"] == shipped[key]["required_claims"]


def test_a_single_library_parameter_opens_the_gate_and_nothing_else_does():
    # no command line or environment switch exists: ``derive`` and ``derive_controls`` take the tuple
    import inspect

    from coding_adventures_warrant_oracle import cli, faults

    assert "implemented" in inspect.signature(derive).parameters
    assert "implemented" in inspect.signature(faults.derive_controls).parameters
    assert "implemented" not in inspect.getsource(cli)
    assert faults.IMPLEMENTED == ("ir.core", "ir.fixed", "ir.tuple", "ir.data")


# -- the witness search against brute force ------------------------------------------------------------------


def brute_force_ordinal(plan: SuitePlan, faulted) -> int | None:
    """The lowest admitted product ordinal at which ``faulted(env)`` differs from the claim's pure side."""
    for ordinal, env in plan.iter_admitted():
        if repr(faulted(env)) != repr(plan.expected(env)):
            return ordinal
    return None


def plan_of(source: bytes, claim: str):
    s = lower_document(parse_source(source))
    prog = build_program(s)
    suite = next(x for x in s.suites if x.claim == claim)
    return s, prog, SuitePlan(s, prog, suite)


def test_the_ct_compare_eq_u64_witness_is_a_equals_zero_b_equals_two_to_the_63():
    """VER04r 13: the S10c witness of ``eq_u64/ignore-high-bit`` is chunk 0, ``a = 0``, ``b = 2**63``."""
    source = adopter_bytes("ct-compare")
    profile = (SPECS / "fixtures" / "ct-compare-warrant-v2" / "corpus.profile").read_bytes()
    shipped = derive(source, profile, spec_ref="ct-compare.warrant", generator=GEN, implemented=("ir.core",))
    opened = derive(source, profile, spec_ref="ct-compare.warrant", generator=GEN, implemented=GATE_OPEN)
    cid = "warrant/ct_compare/negctl/exhaustive/eq_u64/ignore-high-bit"
    claim = "warrant/ct_compare/exhaustive/eq_u64_agrees"
    row = next(c for c in shipped.corpus["controls"] if c["id"] == cid)
    assert row["slim_witness"] == [{"claim": claim, "deferred": "ir.fixed"}]  # the closed gate (pre-flip fixture)
    row = next(c for c in opened.corpus["controls"] if c["id"] == cid)
    assert row["slim_witness"] == [{"claim": claim, "chunk": 0}]
    assert row["requires"] == ["ir.fixed"] and row["required_claims"] == [claim]
    # the ordinal, by brute force over the claim's own domain
    s, prog, plan = plan_of(source, "eq_u64_agrees")
    mask = (1 << 63) - 1

    def faulted(env):
        return (env["a"] & mask) == (env["b"] & mask)

    ordinal = brute_force_ordinal(plan, faulted)
    assert ordinal == 311  # a = 0 (index 0), b = 2**63 (index 311 of the 377 values)
    a_index, b_index = divmod(ordinal, 377)
    assert plan.binders[0].one(a_index) == 0 and plan.binders[1].one(b_index) == 1 << 63
    # nothing else of the ct-compare document moves under the open gate
    for c1, c2 in zip(shipped.corpus["controls"], opened.corpus["controls"]):
        if c1["id"] != cid:
            assert c1 == c2


@pytest.mark.parametrize(
    "op,fault,claim",
    [
        ("fx8", "xor-lsb", "c_fx8"),
        ("fx64", "flip-result-bit", "c_fx64"),
        ("raw", "flip-result-bit", "c_raw"),
        ("rawn", "flip-result-bit", "c_rawn"),
        ("raw", "drop-last-result-byte", "c_raw"),
        ("res_b", "flip-result-bit", "c_res_b"),
        ("res_n", "drop-last-result-byte", "c_res_n"),
        ("res_u", "flip-result-bit", "c_res_u"),
    ],
)
def test_a_post_fault_witness_is_the_lowest_ordinal_a_brute_force_search_finds(op, fault, claim):
    s, prog, plan = plan_of(DOC.encode(), claim)
    row = fault_row(s.ops[op], fault, s.errors)
    tenv = {p: t for p, t in s.ops[op].params}
    tenv["result"] = s.ops[op].ret
    fn = Program({}, s.ops, errors=s.errors).compile(row["expr"], tenv, False).fn

    def faulted(env):
        return fn({**env, "result": plan.expected(env)})

    ordinal = brute_force_ordinal(plan, faulted)
    assert ordinal is not None
    out = derive(
        DOC.encode(), f"chunk_size 1024\nfault {op} {fault}\n".encode(), spec_ref="f.warrant", generator=GEN,
        implemented=GATE_OPEN,
    )
    (control,) = out.corpus["controls"]
    witness = next(w for w in control["slim_witness"] if w["claim"].endswith("/" + claim))
    assert witness["chunk"] == ordinal // 1024


def test_a_fault_no_case_can_see_is_blind_under_the_open_gate_and_needs_accept_blind():
    # the pure side does not read ``a``, so rewriting it changes nothing: every candidate is blind
    doc = "warrant f v1\n  op f(a: U64) -> U64\n  claim c:\n    forall a: U64. f(a) == 0u64\n"
    with pytest.raises(OracleError) as err:
        derive(doc.encode(), b"fault f ignore-high-bit\n", spec_ref="f.warrant", generator=GEN,
               implemented=GATE_OPEN)
    assert "blind" in err.value.message
    out = derive(
        doc.encode(),
        b"fault f ignore-high-bit\naccept_blind f ignore-high-bit\n",
        spec_ref="f.warrant",
        generator=GEN,
        implemented=GATE_OPEN,
    )
    (control,) = out.corpus["controls"]
    assert control["required_claims"] == [] and control["blind_claims"] == ["warrant/f/exhaustive/c"]
    # under the shipped gate the same claim is not searched: the stand-in exists, so it is deferred
    out = derive(doc.encode(), b"fault f ignore-high-bit\n", spec_ref="f.warrant", generator=GEN,
                 implemented=("ir.core",))
    assert out.corpus["controls"][0]["slim_witness"] == [
        {"claim": "warrant/f/exhaustive/c", "deferred": "ir.fixed"}
    ]


def test_a_pre_fault_rewriting_a_binder_passed_twice_rewrites_both_positions():
    doc = (
        "warrant f v1\n  op f(a: U64, b: U64) -> Bool\n  claim c:\n"
        "    forall a: U64. f(a, a) == (a == 0u64)\n"
    )
    out = derive(doc.encode(), b"fault f ignore-high-bit\n", spec_ref="f.warrant", generator=GEN,
                 implemented=GATE_OPEN)
    # both positions are U64 and both are rewritten, so the one binder has one rewritten value
    # (a & mask): the pure side (a & mask) == 0 differs from a == 0 first at a = 2**63, ordinal 311
    (control,) = out.corpus["controls"]
    assert control["slim_witness"] == [{"claim": "warrant/f/exhaustive/c", "chunk": 0}]
