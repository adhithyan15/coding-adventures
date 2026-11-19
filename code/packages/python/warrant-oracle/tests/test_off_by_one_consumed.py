"""Slice S10o-1: the fault ``off-by-one-consumed`` (VER04r 8.2 and 16.10 [D36]).

The second component ``n`` of a tuple result (or of the tuple inside an ``Ok``) becomes ``n + 1`` when
``n`` is even and ``n - 1`` when it is odd.  The Rust harness computes exactly that on the value::

    n  : 0 1 2 3 4 5 ...        the pair 2k, 2k+1 is exchanged
    n' : 1 0 3 2 5 4 ...

so both directions occur and the value never leaves its type.  These tests pin (1) the exact JSON of
section 8.2, (2) the values the template computes against that harness rule, (3) the refusals, which
are those of ``drop-consumed``, and (4) the committed dump in ``fixtures/off-by-one-consumed-v1``.
"""

from __future__ import annotations

import json
import pathlib
import re
import subprocess
import sys

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.faults import REGISTRY, fault_row
from coding_adventures_warrant_oracle.ir_eval import Program
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "off-by-one-consumed-v1"
SPEC = HERE.parents[3] / "specs" / "VER04r-warrant-corpus-v2.md"

R = ["var", "result"]
OKP = ["var", "ok_payload"]
N1 = ["lit", {"type": "Nat", "value": "1"}]
I0 = ["lit", {"type": "Int", "value": "0"}]
I2 = ["lit", {"type": "Int", "value": "2"}]

SOURCE = """warrant f v1
  error E = A | B
  op bare_nat(a: Bool, n: Nat) -> (Bool,Nat)
  op bare_u8(a: Bool, n: U8) -> (Bool,U8)
  op bare_i8(a: Bool, n: I8) -> (Bool,I8)
  op dec_u64(d: Bytes<2>) -> Result<(U64,Nat),E>
  op dec_i64(d: Bytes<2>) -> Result<(I64,Nat),E>
  op dec_u8(a: Bool) -> Result<(Bool,U8),E>
  op tri(a: U8, n: Nat, c: Bool) -> (U8,Nat,Bool)
  op clash_ok(ok_payload: Bool) -> Result<(Bool,Nat),E>
  op clash_bare(ok_payload: Bool) -> (Bool,Nat)
"""


def spec():
    return lower_document(parse_source(SOURCE.encode()))


def row(op: str) -> dict:
    s = spec()
    return fault_row(s.ops[op], "off-by-one-consumed", s.errors)


def nat_x(src: list) -> list:
    """``X`` for a ``Nat`` second component of the tuple ``src`` (VER04r 8.2)."""
    c = ["project", src, 1]
    return ["if", ["eq", ["unwrap_or", ["rem", c, I2], I0], I0], ["add", c, N1], ["sub", c, N1]]


def ok_form(f: list) -> list:
    return ["match", R, [[["ok", ["bind", "ok_payload"]], ["ok", f]], [["err_any"], R]]]


def evaluate(op: str, result, params: dict | None = None):
    s = spec()
    r = fault_row(s.ops[op], "off-by-one-consumed", s.errors)
    prog = Program({}, s.ops, errors=s.errors)
    tenv = {p: t for p, t in s.ops[op].params}
    tenv["result"] = s.ops[op].ret
    return prog.compile(r["expr"], tenv, False).fn({**(params or {}), "result": result})


# -- the exact JSON of VER04r 8.2 --------------------------------------------------------------------------------

#: The compact JSON of VER04r 8.2 for both wasm-leb128 result types (``Result<(U64,Nat),..>``).
SPEC_COMPACT = (
    '["match",["var","result"],[[["ok",["bind","ok_payload"]],["ok",["tuple",[["project",["var","ok_payload"],0],'
    '["if",["eq",["unwrap_or",["rem",["project",["var","ok_payload"],1],["lit",{"type":"Int","value":"2"}]],'
    '["lit",{"type":"Int","value":"0"}]],["lit",{"type":"Int","value":"0"}]],'
    '["add",["project",["var","ok_payload"],1],["lit",{"type":"Nat","value":"1"}]],'
    '["sub",["project",["var","ok_payload"],1],["lit",{"type":"Nat","value":"1"}]]]]]]],'
    '[["err_any"],["var","result"]]]]'
)


def compact(value) -> str:
    return json.dumps(value, separators=(",", ":"))


def test_the_wasm_leb128_ok_form_is_the_compact_json_of_the_spec():
    for op in ("dec_u64", "dec_i64"):
        assert compact(row(op)["expr"]) == SPEC_COMPACT


def test_the_compact_json_here_is_the_one_in_the_spec_text():
    if not SPEC.exists():  # a sparse checkout without the specs directory
        pytest.skip("VER04r is not checked out")
    text = SPEC.read_text(encoding="utf-8")
    start = text.index("**`off-by-one-consumed`: exact form")
    blocks = re.findall(r"```json\n(.*?)\n```", text[start:], re.S)
    assert blocks[0].strip() == SPEC_COMPACT


def test_the_row_is_a_post_fault_at_fixed_and_tuple_in_all_four_combinations():
    for op in ("bare_nat", "bare_u8", "bare_i8", "dec_u64", "dec_i64", "dec_u8", "tri"):
        r = row(op)
        assert r["fault"] == "off-by-one-consumed" and r["op"] == op and r["kind"] == "post"
        assert r["requires"] == ["ir.fixed", "ir.tuple"]


def test_a_bare_tuple_with_a_nat_second_component():
    assert row("bare_nat")["expr"] == ["tuple", [["project", R, 0], nat_x(R)]]


def test_a_fixed_width_second_component_is_the_xor_with_one():
    def x(ty):
        return ["bit_xor", ["project", R, 1], ["lit", {"type": ty, "value": "1"}]]

    assert row("bare_u8")["expr"] == ["tuple", [["project", R, 0], x("U8")]]
    assert row("bare_i8")["expr"] == ["tuple", [["project", R, 0], x("I8")]]


def test_the_ok_forms_project_from_the_bound_payload_and_leave_an_err_alone():
    assert row("dec_u64")["expr"] == ok_form(["tuple", [["project", OKP, 0], nat_x(OKP)]])
    assert row("dec_u8")["expr"] == ok_form(
        ["tuple", [["project", OKP, 0], ["bit_xor", ["project", OKP, 1], ["lit", {"type": "U8", "value": "1"}]]]]
    )


def test_a_longer_tuple_passes_its_other_components_through_in_order():
    assert row("tri")["expr"] == ["tuple", [["project", R, 0], nat_x(R), ["project", R, 2]]]
    assert evaluate("tri", (9, 4, True)) == (9, 5, True)
    assert evaluate("tri", (9, 5, False)) == (9, 4, False)


def test_the_registry_holds_the_name_beside_the_core_off_by_one():
    assert "off-by-one-consumed" in REGISTRY and "off-by-one" in REGISTRY


# -- the values: the harness rule ----------------------------------------------------------------------------------

U64_MAX = (1 << 64) - 1


def harness_nat(n: int) -> int:
    return n + 1 if n % 2 == 0 else n - 1


@pytest.mark.parametrize("n", [0, 1, 2, 3, 4, 5, U64_MAX - 1, U64_MAX])
def test_a_nat_component_follows_the_harness_rule(n):
    want = harness_nat(n)
    assert evaluate("bare_nat", (True, n)) == (True, want)
    assert evaluate("dec_u64", ("ok", (7, n))) == ("ok", (7, want))
    assert 0 <= want <= U64_MAX  # a Nat value stays a Nat value: the largest one is odd and goes down


def test_the_two_directions_of_the_nat_pair():
    assert [evaluate("bare_nat", (False, n))[1] for n in range(6)] == [1, 0, 3, 2, 5, 4]


def flip_bit_zero(v: int, bits: int, signed: bool) -> int:
    """The harness: test ``v % 2``, then wrapping ``+ 1`` or ``- 1`` on the ``bits``-bit pattern."""
    mask = (1 << bits) - 1
    out = ((v + 1) if v % 2 == 0 else (v - 1)) & mask
    if signed and out >= 1 << (bits - 1):
        out -= 1 << bits
    return out


@pytest.mark.parametrize("v", [0, 1, 254, 255])
def test_a_u8_component_flips_bit_zero(v):
    assert evaluate("bare_u8", (True, v)) == (True, v ^ 1) == (True, flip_bit_zero(v, 8, False))
    assert evaluate("dec_u8", ("ok", (True, v))) == ("ok", (True, v ^ 1))


@pytest.mark.parametrize("v", [-128, -127, -1, 0, 1, 126, 127])
def test_an_i8_component_flips_bit_zero_on_the_pattern(v):
    got = evaluate("bare_i8", (False, v))
    assert got == (False, flip_bit_zero(v, 8, True))
    assert -128 <= got[1] <= 127 and got[1] != v and abs(got[1] - v) == 1


def test_signed_pairs_are_exchanged_too():
    assert [evaluate("bare_i8", (True, v))[1] for v in (-128, -127, -2, -1, 0, 1)] == [-127, -128, -1, -2, 1, 0]


@pytest.mark.parametrize("op", ["dec_u64", "dec_i64", "dec_u8"])
def test_an_err_is_returned_unchanged(op):
    assert evaluate(op, ("err", "B")) == ("err", "B")


def test_an_i64_first_component_is_untouched():
    assert evaluate("dec_i64", ("ok", (-5, 3))) == ("ok", (-5, 2))


# -- refusals: those of drop-consumed ------------------------------------------------------------------------------


@pytest.mark.parametrize(
    "ret",
    ["U8", "Bytes", "Bool", "Result<U8,E>", "Result<Bytes,E>", "Option<(U8,Nat)>", "(U8,Bool)", "(Nat,Bytes<2>)", "Result<(Nat,Bool),E>"],
)
def test_the_refusals_match_drop_consumed(ret):
    s = lower_document(parse_source(f"warrant r v1\n  error E = A | B\n  op f(a: U8) -> {ret}\n".encode()))
    with pytest.raises(OracleError) as mine:
        fault_row(s.ops["f"], "off-by-one-consumed", s.errors)
    with pytest.raises(OracleError) as theirs:
        fault_row(s.ops["f"], "drop-consumed", s.errors)
    assert mine.value.code == theirs.value.code == "generation"
    assert mine.value.message.startswith("fault: off-by-one-consumed needs")
    assert mine.value.message.replace("off-by-one-consumed", "X") == theirs.value.message.replace("drop-consumed", "X")


def test_a_parameter_named_ok_payload_is_refused_in_the_ok_form_only():
    s = spec()
    with pytest.raises(OracleError) as err:
        fault_row(s.ops["clash_ok"], "off-by-one-consumed", s.errors)
    assert err.value.code == "generation" and "ok_payload" in err.value.message
    # the bare tuple introduces no binder, so the name cannot clash
    assert fault_row(s.ops["clash_bare"], "off-by-one-consumed", s.errors)["kind"] == "post"


def test_ast_types_are_what_the_template_reads():
    ret = spec().ops["dec_u64"].ret
    assert ret.head == "Result" and ret.args[0].head == "()" and ret.args[0].args[1] == A.Ty("Nat")


# -- the committed dump --------------------------------------------------------------------------------------------


def test_the_committed_dump_is_the_generators_output():
    out = subprocess.run(
        [sys.executable, str(FIXTURE / "make_off_by_one_consumed.py"), "--check"], capture_output=True, text=True
    )
    assert out.returncode == 0, out.stderr


def test_the_committed_faults_dump_holds_the_spec_json_and_live_controls():
    dump = json.loads((FIXTURE / "off_by_one_consumed_v1.oracle-faults.json").read_text(encoding="utf-8"))
    by_op = {f["op"]: f for f in dump["faults"]}
    assert set(by_op) == {"bare_nat", "bare_u8", "bare_i8", "dec_u64", "dec_i64", "dec_u8", "tri"}
    assert compact(by_op["dec_u64"]["expr"]) == SPEC_COMPACT
    # `controls_shipped` = the explicit closed gate GATE_CLOSED (`gate_shipped` in the dump; a historical key
    # name shared with fixed-eval-v1), so every claim defers; `controls_gate_on` = GATE_OPEN, which is what
    # both tools ship since the S10c flip, so every claim evaluates
    assert all("deferred" in w for c in dump["controls_shipped"] for w in c["slim_witness"])
    assert all("chunk" in w for c in dump["controls_gate_on"] for w in c["slim_witness"])
    assert dump["gate_shipped"] == ["ir.core"]
    assert all(c["blind_claims"] == [] for c in dump["controls_shipped"])
    # the one Err-region claim is proven blind under the open gate (section 8.2): its stand-in expects only Err
    blind = {c["op"]: c["blind_claims"] for c in dump["controls_gate_on"] if c["blind_claims"]}
    assert blind == {"dec_u64": ["warrant/off_by_one_consumed/exhaustive/c_dec_u64_err"]}
    ctl = next(c for c in dump["controls_gate_on"] if c["op"] == "dec_u64")
    assert ctl["required_claims"] == ["warrant/off_by_one_consumed/exhaustive/c_dec_u64"]
    assert [w["claim"] for w in ctl["slim_witness"]] == ctl["required_claims"]
    closed = next(c for c in dump["controls_shipped"] if c["op"] == "dec_u64")
    assert closed["required_claims"] == ctl["required_claims"] + blind["dec_u64"]


def test_the_committed_refusals_are_all_generation_fault_errors():
    dump = json.loads((FIXTURE / "off_by_one_consumed_v1.oracle-refusals.json").read_text(encoding="utf-8"))
    assert dump["refusals"] and all(
        r["oracle_code"] == "generation" and r["message"].startswith("fault:") for r in dump["refusals"]
    )


def test_the_committed_refusals_pin_the_bytes_result_and_the_second_component_first_ordering():
    dump = json.loads((FIXTURE / "off_by_one_consumed_v1.oracle-refusals.json").read_text(encoding="utf-8"))
    by_name = {r["name"]: r for r in dump["refusals"]}
    bytes_row = by_name["a-result-of-bytes"]
    assert "-> Result<Bytes,E>" in bytes_row["source"]
    assert bytes_row["message"] == "fault: off-by-one-consumed needs a tuple or Result<tuple, E> result"
    order = by_name["a-non-numeric-second-component-before-the-ok-payload-clash"]
    assert "op f(ok_payload: U8) -> (U8,Bool)" in order["source"]
    assert order["message"] == "fault: off-by-one-consumed needs a numeric second component"
    assert "ok_payload" not in order["message"]
    ok_order = by_name["a-non-numeric-second-component-before-the-ok-payload-clash-in-the-ok-form"]
    assert "op f(ok_payload: U8) -> Result<(U8,Bool),E>" in ok_order["source"]
    assert ok_order["message"] == "fault: off-by-one-consumed needs a numeric second component"
    assert "ok_payload" not in ok_order["message"]


def test_a_numeric_second_component_is_checked_before_the_ok_payload_name():
    s = lower_document(parse_source(b"warrant r v1\n  error E = A | B\n  op f(ok_payload: U8) -> (U8,Bool)\n"))
    with pytest.raises(OracleError) as mine:
        fault_row(s.ops["f"], "off-by-one-consumed", s.errors)
    with pytest.raises(OracleError) as theirs:
        fault_row(s.ops["f"], "drop-consumed", s.errors)
    assert "numeric second component" in mine.value.message
    assert mine.value.message.replace("off-by-one-consumed", "X") == theirs.value.message.replace("drop-consumed", "X")
