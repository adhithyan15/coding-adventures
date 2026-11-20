"""Slice S10t-1: the ``ir.tuple`` fault templates and their gate (VER04r 8.2, D27, D35, R3, R11).

``ok-as-err``, ``drop-consumed``, ``swap-components``, the tuple-payload ``missing-error`` and the
``Ok`` forms of ``flip-result-bit`` and ``drop-last-result-byte`` are instantiated as data; their
evaluation is **gated** like ``ir.fixed``'s: since the atomic flip (S10c, D29) the shipped gate is
open to ``ir.tuple``, and ``deferred`` witnesses are reachable only through an explicitly closed gate
(``GATE_CLOSED``), which these tests pass.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.corpus_v2 import derive
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.faults import (
    IMPLEMENTED,
    OK_PAYLOAD,
    REGISTRY,
    derive_controls,
    fault_row,
)
from coding_adventures_warrant_oracle.ir_eval import Program
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.profile import parse_profile
from coding_adventures_warrant_oracle.suites import SuitePlan, Work

GEN = {"name": "test", "version": "0"}
GATE_ON = ("ir.core", "ir.tuple")
GATE_OPEN = ("ir.core", "ir.fixed", "ir.tuple")
R = ["var", "result"]
N0 = ["lit", {"type": "Nat", "value": "0"}]
N1 = ["lit", {"type": "Nat", "value": "1"}]
U0 = ["lit", {"type": "U8", "value": "0"}]
U1 = ["lit", {"type": "U8", "value": "1"}]
B01 = ["lit", {"type": "Bytes", "value": "01"}]
B00 = ["lit", {"type": "Bytes", "value": "00"}]
GT = ["gt", ["len", R], N0]
OKP = ["var", "ok_payload"]

HEAD = (
    "warrant f v1\n"
    "  error E = A | B | C\n"
    "  record Point(x: U8, y: U8)\n"
    "  op pair(a: Bool, n: Nat) -> (Bool,Nat)\n"
    "  op tri(a: U8, b: U8, c: U8) -> (U8,U8,U8)\n"
    "  op dup(a: Bool) -> (Bool,Bool)\n"
    "  op res(a: Bool, n: Nat) -> Result<(Bool,Nat),E>\n"
    "  op res_same(a: Bool, b: Bool) -> Result<(Bool,Bool),E>\n"
    "  op res_bytes(d: Bytes<2>, n: Nat) -> Result<(Bytes<2>,Nat,Bool),E>\n"
    "  op res_nested(a: Bool) -> Result<((Bool,Bool),Nat),E>\n"
    "  op res_u8(a: U8) -> Result<U8,E>\n"
    "  op res_b(d: Bytes) -> Result<Bytes,E>\n"
    "  op res_bn(d: Bytes<4>) -> Result<Bytes<4>,E>\n"
    "  op raw(d: Bytes) -> Bytes\n"
    "  op rawn(d: Bytes<4>) -> Bytes<4>\n"
    "  op u8(a: U8) -> U8\n"
    "  op flag(a: Bool) -> Bool\n"
    "  op tflag(t: (Bool,Bool)) -> Bool\n"
    "  op rflag(p: Point) -> Bool\n"
    "  op clash(ok_payload: Bytes) -> Result<Bytes,E>\n"
    "  op res_pt(a: Bool) -> Result<Point,E>\n"
    "  op res_word<N: Nat>(w: Bytes<N>) -> Result<(Bytes<N>,Nat),E>\n"
)


def ops():
    return lower_document(parse_source(HEAD)).ops


def spec():
    return lower_document(parse_source(HEAD))


def row(op: str, fault: str) -> dict:
    s = spec()
    return fault_row(s.ops[op], fault, s.errors, frozenset([*s.records, *s.variants]))


def ok_form(f: list) -> list:
    return ["match", R, [[["ok", ["bind", "ok_payload"]], ["ok", f]], [["err_any"], R]]]


# -- the exact JSON of each template (R11: these are what the committed dump pins) ----------------


def test_ok_as_err_reports_success_as_the_first_error_case():
    r = row("res", "ok-as-err")
    assert r == {
        "fault": "ok-as-err",
        "op": "res",
        "kind": "post",
        "requires": ["ir.tuple"],
        "expr": ["match", R, [[["ok", ["wild"]], ["err", "A"]], [["err_any"], R]]],
    }


def test_drop_consumed_replaces_the_second_component_by_zero_of_its_type():
    bare = row("pair", "drop-consumed")
    assert bare["kind"] == "post" and bare["requires"] == ["ir.tuple"]
    assert bare["expr"] == ["tuple", [["project", R, 0], N0]]
    inside = row("res", "drop-consumed")
    assert inside["expr"] == ok_form(["tuple", [["project", OKP, 0], N0]])
    # a longer tuple keeps its other components in place
    three = row("res_bytes", "drop-consumed")
    assert three["expr"] == ok_form(["tuple", [["project", OKP, 0], N0, ["project", OKP, 2]]])
    # a fixed-width second component gets a literal of that width
    u = lower_document(parse_source("warrant d v1\n  op f(a: U8) -> (U8,U32)\n")).ops["f"]
    assert fault_row(u, "drop-consumed")["expr"] == [
        "tuple",
        [["project", R, 0], ["lit", {"type": "U32", "value": "0"}]],
    ]


def test_swap_components_reverses_a_homogeneous_tuple():
    assert row("dup", "swap-components")["expr"] == ["tuple", [["project", R, 1], ["project", R, 0]]]
    assert row("tri", "swap-components")["expr"] == [
        "tuple",
        [["project", R, 2], ["project", R, 1], ["project", R, 0]],
    ]
    assert row("res_same", "swap-components")["expr"] == ok_form(
        ["tuple", [["project", OKP, 1], ["project", OKP, 0]]]
    )
    assert row("res_same", "swap-components")["requires"] == ["ir.tuple"]


def test_missing_error_with_a_tuple_payload_uses_params_of_exactly_the_components_type():
    expected = ["if", ["is_err", R], ["ok", ["tuple", [["var", "a"], ["var", "n"]]]], R]
    r = row("res", "missing-error")
    assert r["kind"] == "replace" and r["requires"] == ["ir.tuple"] and r["expr"] == expected
    # (Bytes<2>,Nat,Bool): d is the first Bytes<2>, n the first Nat, and no Bool param: false
    r = row("res_bytes", "missing-error")
    assert r["expr"] == ["if", ["is_err", R], ["ok", ["tuple", [["var", "d"], ["var", "n"], ["lit", False]]]], R]
    # R3: "the first parameter, in parameter order, of exactly the type": both Bool components
    # of (Bool,Bool) take `a`, the first Bool parameter, never `b`
    r = row("res_same", "missing-error")
    assert r["expr"][2] == ["ok", ["tuple", [["var", "a"], ["var", "a"]]]]
    # no parameter of a component's type at all: the zero of every type
    z = lower_document(parse_source("warrant z v1\n  error E = A\n  op f(d: Bytes<0>) -> Result<(Bool,Nat,Bytes,Bytes<2>,U8),E>\n")).ops["f"]
    assert fault_row(z, "missing-error")["expr"][2] == [
        "ok",
        [
            "tuple",
            [
                ["lit", False],
                N0,
                ["lit", {"type": "Bytes", "value": ""}],
                ["lit", {"type": "Bytes", "value": "0000"}],
                ["lit", {"type": "U8", "value": "0"}],
            ],
        ],
    ]


def test_missing_error_keeps_its_core_form_for_a_bytes_payload():
    r = row("res_b", "missing-error")
    assert r["requires"] == ["ir.core"]
    assert r["expr"] == ["if", ["is_err", R], ["ok", ["var", "d"]], R]


def test_flip_result_bit_forms_are_those_of_section_8_2():
    flip_bytes = lambda src: [  # noqa: E731
        "if",
        ["gt", ["len", src], N0],
        ["concat", ["be_bytes", ["bit_xor", ["unwrap_or", ["at", src, N0], U0], U1]], ["drop", src, N1]],
        B01,
    ]
    fixed = row("u8", "flip-result-bit")
    assert fixed["requires"] == ["ir.fixed"] and fixed["expr"] == ["bit_xor", R, U1]
    for name in ("raw", "rawn"):  # Bytes and Bytes<N>: one template
        r = row(name, "flip-result-bit")
        assert r["requires"] == ["ir.fixed"] and r["expr"] == flip_bytes(R)
    for name in ("res_b", "res_bn"):
        r = row(name, "flip-result-bit")
        assert r["requires"] == ["ir.fixed", "ir.tuple"]
        assert r["expr"] == ok_form(flip_bytes(OKP))
    r = row("res_u8", "flip-result-bit")
    assert r["requires"] == ["ir.fixed", "ir.tuple"]
    assert r["expr"] == ok_form(["bit_xor", OKP, U1])


def test_drop_last_result_byte_forms_are_those_of_section_8_2():
    drop = lambda src: ["if", ["gt", ["len", src], N0], ["take", src, ["sub", ["len", src], N1]], B00]  # noqa: E731
    for name in ("raw", "rawn"):
        r = row(name, "drop-last-result-byte")
        assert r["requires"] == ["ir.core"] and r["expr"] == drop(R)
    r = row("res_b", "drop-last-result-byte")
    assert r["requires"] == ["ir.core", "ir.tuple"] and r["expr"] == ok_form(drop(OKP))
    assert row("res_bn", "drop-last-result-byte")["expr"] == ok_form(drop(OKP))


def test_every_existing_row_keeps_its_exact_shape():
    # the fault rows of the committed fixtures must not move a byte (S10c is the only bump)
    assert row("flag", "invert") == {
        "fault": "invert",
        "op": "flag",
        "kind": "post",
        "requires": ["ir.core"],
        "expr": ["not", R],
    }
    assert row("u8", "xor-lsb")["requires"] == ["ir.fixed"]


# -- templates the signature cannot take ------------------------------------------------------------------


@pytest.mark.parametrize(
    "op,fault",
    [
        ("pair", "ok-as-err"),  # a bare tuple is not a Result
        ("res_b", "ok-as-err"),  # a Bytes payload is not composite
        ("res_u8", "ok-as-err"),
        ("u8", "ok-as-err"),
        ("res", "swap-components"),  # (Bool,Nat) is not homogeneous
        ("pair", "swap-components"),
        ("res_bytes", "swap-components"),
        ("u8", "swap-components"),
        ("flag", "drop-consumed"),
        ("res_same", "drop-consumed"),  # the second component is a Bool: not numeric
        ("dup", "drop-consumed"),
        ("res_b", "drop-consumed"),
        ("res_nested", "swap-components"),  # ((Bool,Bool),Nat): components differ
        ("flag", "missing-error"),  # not a Result
        ("res_pt", "missing-error"),
        ("flag", "flip-result-bit"),
        ("res_pt", "flip-result-bit"),
        ("u8", "drop-last-result-byte"),
        ("res_u8", "drop-last-result-byte"),
        ("clash", "flip-result-bit"),  # an op parameter named ok_payload
        ("clash", "drop-last-result-byte"),
    ],
)
def test_a_template_the_signature_cannot_take_is_a_fault_error(op, fault):
    s = spec()
    with pytest.raises(OracleError) as err:
        fault_row(s.ops[op], fault, s.errors)
    assert err.value.code == "generation" and err.value.message.startswith("fault:")


def test_drop_consumed_on_a_nested_tuple_with_a_numeric_second_component_is_accepted():
    r = row("res_nested", "drop-consumed")
    assert r["expr"] == ok_form(["tuple", [["project", OKP, 0], N0]])


def test_missing_error_with_a_named_width_component_uses_the_parameter_of_that_type():
    # Bytes<N> (a named width) has no zero literal, but the parameter w: Bytes<N> has exactly the type
    r = row("res_word", "missing-error")
    assert r["expr"][2] == ["ok", ["tuple", [["var", "w"], N0]]]
    z = lower_document(parse_source("warrant z v1\n  error E = A\n  op f<N: Nat>(d: Bytes<0>) -> Result<(Bytes<N>,Nat),E>\n")).ops["f"]
    with pytest.raises(OracleError):  # no parameter of the type and no static width: no zero
        fault_row(z, "missing-error")


def test_a_component_with_no_zero_and_no_matching_parameter_is_a_fault_error():
    z = lower_document(parse_source("warrant z v1\n  error E = A\n  op f(a: Bool) -> Result<((Bool,Bool),Nat),E>\n")).ops["f"]
    with pytest.raises(OracleError):
        fault_row(z, "missing-error")  # the nested tuple has no zero literal
    assert OK_PAYLOAD == "ok_payload"


def test_ok_as_err_needs_the_error_types_cases_and_an_unknown_fault_is_an_error():
    s = spec()
    with pytest.raises(OracleError):
        fault_row(s.ops["res"], "ok-as-err")  # no error table
    with pytest.raises(OracleError):
        fault_row(s.ops["res"], "ok-as-err", {"E": ()})
    with pytest.raises(OracleError):
        fault_row(s.ops["res"], "no-such-fault", s.errors)


# -- requires: the types of the op count (R9) ------------------------------------------------------------------


def test_a_fault_on_an_operation_with_a_tuple_parameter_requires_ir_tuple():
    r = row("tflag", "invert")
    assert r["requires"] == ["ir.core", "ir.tuple"] and r["expr"] == ["not", R]
    # the same fault on a plain op does not
    assert row("flag", "invert")["requires"] == ["ir.core"]


def test_a_fault_on_an_operation_with_a_record_type_requires_ir_data():
    assert row("rflag", "invert")["requires"] == ["ir.core", "ir.data"]


# -- evaluating the faults (typed and run by the IR evaluator) -----------------------------------------------------


def evaluate(op: str, fault: str, result, params: dict | None = None):
    s = spec()
    r = fault_row(s.ops[op], fault, s.errors)
    prog = Program({}, s.ops, errors=s.errors)
    tenv = {p: t for p, t in s.ops[op].params}
    tenv["result"] = s.ops[op].ret
    fn = prog.compile(r["expr"], tenv, False).fn
    return fn({**(params or {}), "result": result})


def test_ok_as_err_evaluates():
    assert evaluate("res", "ok-as-err", ("ok", (True, 3))) == ("err", "A")
    assert evaluate("res", "ok-as-err", ("err", "C")) == ("err", "C")


def test_drop_consumed_and_swap_components_evaluate():
    assert evaluate("pair", "drop-consumed", (True, 7)) == (True, 0)
    assert evaluate("res", "drop-consumed", ("ok", (True, 7))) == ("ok", (True, 0))
    assert evaluate("res", "drop-consumed", ("err", "B")) == ("err", "B")
    assert evaluate("res_bytes", "drop-consumed", ("ok", (b"ab", 5, True))) == ("ok", (b"ab", 0, True))
    assert evaluate("dup", "swap-components", (True, False)) == (False, True)
    assert evaluate("tri", "swap-components", (1, 2, 3)) == (3, 2, 1)
    assert evaluate("res_same", "swap-components", ("ok", (True, False))) == ("ok", (False, True))
    assert evaluate("res_same", "swap-components", ("err", "A")) == ("err", "A")


def test_missing_error_with_a_tuple_payload_evaluates():
    params = {"a": True, "n": 4}
    assert evaluate("res", "missing-error", ("err", "B"), params) == ("ok", (True, 4))
    assert evaluate("res", "missing-error", ("ok", (False, 1)), params) == ("ok", (False, 1))


def test_drop_last_result_byte_evaluates_on_core():
    assert evaluate("raw", "drop-last-result-byte", b"\x01\x02\x03") == b"\x01\x02"
    assert evaluate("raw", "drop-last-result-byte", b"") == b"\x00"
    assert evaluate("res_b", "drop-last-result-byte", ("ok", b"\x01\x02")) == ("ok", b"\x01")
    assert evaluate("res_b", "drop-last-result-byte", ("ok", b"")) == ("ok", b"\x00")
    assert evaluate("res_b", "drop-last-result-byte", ("err", "C")) == ("err", "C")
    assert evaluate("rawn", "drop-last-result-byte", b"\x01\x02\x03\x04") == b"\x01\x02\x03"


def test_the_ir_fixed_forms_are_evaluated_since_s10c1():
    s = spec()
    r = fault_row(s.ops["res_b"], "flip-result-bit", s.errors)
    prog = Program({}, s.ops, errors=s.errors)
    tenv = {"d": s.ops["res_b"].params[0][1], "result": s.ops["res_b"].ret}
    compiled = prog.compile(r["expr"], tenv, False)  # the match arms use fixed tags
    assert compiled.ty.head == "Result"
    assert compiled.fn({"d": b"", "result": ("ok", b"\x01")}) == ("ok", b"\x00")
    assert compiled.fn({"d": b"", "result": ("ok", b"")}) == ("ok", b"\x01")
    assert compiled.fn({"d": b"", "result": ("err", "C")}) == ("err", "C")


# -- the gate -----------------------------------------------------------------------------------------------------------

DOC = """warrant g v1
  error E = A | B
  op pair(a: Bool, n: Nat) -> (Bool,Nat)
  model pair_m(a: Bool, n: Nat) -> (Bool,Nat) = (a, n)
  op res(a: Bool, n: Nat) -> Result<(Bool,Nat),E>
  model res_m(a: Bool, n: Nat) -> Result<(Bool,Nat),E> = if n > 60 then Err(E.B) else Ok((a, n))
  op raw(d: Bytes<2>) -> Bytes<2>
  op res_b(d: Bytes<2>) -> Result<Bytes<2>,E>
  claim pair_claim:
    forall a: Bool, n: Nat. pair(a, n) == pair_m(a, n)
  claim pair_pred:
    forall a: Bool, n: Nat. pair(a, n) != (true, 99)
  claim res_claim:
    forall a: Bool, n: Nat. res(a, n) == res_m(a, n)
  claim raw_claim:
    forall d: Bytes<2>. raw(d) == d
  claim res_b_claim:
    forall d: Bytes<2>. res_b(d) == (if len(d) == 2 then Ok(d) else Err(E.A))
"""
PROFILE = """chunk_size 1024
fault pair drop-consumed
fault res drop-consumed
fault res ok-as-err
fault res missing-error
fault raw drop-last-result-byte
fault raw flip-result-bit
fault res_b drop-last-result-byte
fault res_b flip-result-bit
"""


GATE_CLOSED = ("ir.core",)


def controls(implemented=GATE_CLOSED, profile=PROFILE, doc=DOC):
    out = derive(doc.encode(), profile.encode(), spec_ref="g.warrant", generator=GEN, implemented=implemented)
    return {c["id"].rsplit("/", 2)[-2] + "/" + c["fault"]: c for c in out.corpus["controls"]}, out


def witness(control: dict) -> dict:
    return {w["claim"].rsplit("/", 1)[-1]: w.get("chunk", w.get("deferred", w.get("unproven"))) for w in control["slim_witness"]}


def test_the_shipped_gate_is_open_and_the_closed_gate_defers_tuple_faults():
    assert IMPLEMENTED == ("ir.core", "ir.fixed", "ir.tuple")
    assert GATE_CLOSED == ("ir.core",)
    by, out = controls()
    assert witness(by["pair/drop-consumed"]) == {"pair_claim": "ir.tuple", "pair_pred": True}
    assert witness(by["res/ok-as-err"]) == {"res_claim": "ir.tuple"}
    assert witness(by["res/missing-error"]) == {"res_claim": "ir.tuple"}
    # a fault that needs ir.core only is evaluated even though it is in the same corpus
    assert witness(by["raw/drop-last-result-byte"]) == {"raw_claim": 0}
    # ir.fixed stays deferred under the closed gate this test passes; two unimplemented levels name the first in sorted order
    assert witness(by["raw/flip-result-bit"]) == {"raw_claim": "ir.fixed"}
    assert witness(by["res_b/flip-result-bit"]) == {"res_b_claim": "ir.fixed"}
    assert witness(by["res_b/drop-last-result-byte"]) == {"res_b_claim": "ir.tuple"}
    # the control's requires is the whole union, not its first level
    assert by["res_b/flip-result-bit"]["requires"] == ["ir.fixed", "ir.tuple"]
    assert by["res_b/drop-last-result-byte"]["requires"] == ["ir.core", "ir.tuple"]
    assert out.corpus["levels"] == ["ir.core", "ir.fixed", "ir.tuple"]


def test_the_witness_search_runs_for_tuple_faults_only_through_the_library_parameter():
    by, _ = controls(implemented=GATE_ON)
    assert witness(by["pair/drop-consumed"]) == {"pair_claim": 0, "pair_pred": True}
    assert witness(by["res/ok-as-err"]) == {"res_claim": 0}
    assert witness(by["res/drop-consumed"]) == {"res_claim": 0}
    assert witness(by["res/missing-error"]) == {"res_claim": 0}
    assert witness(by["res_b/drop-last-result-byte"]) == {"res_b_claim": 0}
    # still gated at ir.fixed
    assert witness(by["raw/flip-result-bit"]) == {"raw_claim": "ir.fixed"}
    assert witness(by["res_b/flip-result-bit"]) == {"res_b_claim": "ir.fixed"}


def test_witnesses_are_the_lowest_admitted_ordinal_and_feed_the_slim_set():
    doc = DOC.replace("forall a: Bool, n: Nat. pair(a, n) == pair_m(a, n)", "forall a: Bool, n: Nat | n >= 40. pair(a, n) == pair_m(a, n)")
    out = derive(
        doc.encode(),
        b"chunk_size 1024\nfault pair drop-consumed\n",
        spec_ref="g.warrant",
        generator=GEN,
        implemented=GATE_ON,
    )
    ctl = out.corpus["controls"][0]
    # product ordinal of (false, 40) is 40: the lowest case where n != 0 survives drop-consumed
    assert witness(ctl)["pair_claim"] == 0
    suite = next(s for s in out.corpus["suites"] if s["claim"] == "pair_claim")
    assert 0 in suite["slim_chunks"]


def test_a_tuple_fault_no_claim_can_see_is_blind_once_the_gate_is_on_and_deferred_before():
    doc = """warrant b v1
  error E = A
  op same(a: Bool) -> (Bool,Bool)
  model same_m(a: Bool) -> (Bool,Bool) = (a, a)
  claim c:
    forall a: Bool. same(a) == same_m(a)
"""
    profile = "fault same swap-components\n"
    by, _ = controls(profile=profile, doc=doc)  # shipped: deferred, nothing is judged
    assert witness(by["same/swap-components"]) == {"c": "ir.tuple"}
    with pytest.raises(OracleError) as err:
        controls(implemented=GATE_ON, profile=profile, doc=doc)
    assert err.value.code == "generation" and "blind" in err.value.message
    # accept_blind excuses it under the gate and is a stale line when nothing is evaluated... the
    # accepted fault is all-blind once the gate is on
    by, out = controls(implemented=GATE_ON, profile=profile + "accept_blind same swap-components\n", doc=doc)
    assert out.corpus["accepted_blind_faults"] == ["warrant/b/negctl/exhaustive/same/swap-components"]
    assert by["same/swap-components"]["required_claims"] == []


def test_derive_controls_takes_the_gate_as_a_parameter_and_defaults_to_the_shipped_one():
    # the shipped gate is open: ``ir.tuple`` and ``ir.fixed`` faults are searched, not deferred
    s = lower_document(parse_source(DOC))
    prog = build_program(s)
    plans = {x.claim: SuitePlan(s, prog, x, chunk_size=1024, work=Work("full")) for x in s.suites}
    prof = parse_profile(PROFILE.encode())
    shipped = derive_controls(s, prog, plans, prof.faults, 1024)
    closed = derive_controls(s, prog, plans, prof.faults, 1024, implemented=GATE_CLOSED)
    on = derive_controls(s, prog, plans, prof.faults, 1024, implemented=GATE_ON)
    opened = derive_controls(s, prog, plans, prof.faults, 1024, implemented=GATE_OPEN)
    assert shipped[0] == closed[0] == on[0]  # the fault rows never depend on the gate
    assert closed[1] != on[1]  # the controls do
    assert shipped[1] == opened[1] == derive_controls(s, prog, plans, prof.faults, 1024, implemented=IMPLEMENTED)[1]
    assert shipped[1] != closed[1]  # the shipped gate is no longer the closed one


def test_the_registry_has_the_seventeen_names():
    assert sorted(REGISTRY) == sorted(
        [
            "invert",
            "missing-error",
            "swap-choice",
            "off-by-one",
            "truncate-last",
            "ignore-last-byte",
            "xor-lsb",
            "ignore-high-bit",
            "flip-result-bit",
            "drop-last-result-byte",
            "ok-as-err",
            "drop-consumed",
            "swap-components",
            # S10o-1 (VER04r 8.2, D36)
            "off-by-one-consumed",
            # S11f-1 (VER04r 8.2)
            "saturate-add",
            "saturate-sub",
            "shift-mask",
        ]
    )


def test_an_op_with_a_parameter_named_ok_payload_is_refused_by_the_result_forms_only():
    s = spec()
    with pytest.raises(OracleError):
        fault_row(s.ops["clash"], "flip-result-bit", s.errors)
    # a core form has no binder, so the name cannot clash
    assert fault_row(s.ops["clash"], "missing-error", s.errors)["kind"] == "replace"


def test_the_pre_rows_are_untouched_by_the_new_requires_computation():
    s = lower_document(parse_source("warrant p v1\n  op f(a: Bytes) -> Bytes\n  op g(a: U64) -> U64\n"))
    assert fault_row(s.ops["f"], "ignore-last-byte")["requires"] == ["ir.core"]
    assert fault_row(s.ops["g"], "ignore-high-bit")["requires"] == ["ir.fixed"]
