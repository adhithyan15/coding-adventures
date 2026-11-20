"""Slice S11f-1: ``saturate-add``, ``saturate-sub``, ``shift-mask`` and the scalar ``missing-error``.

VER04r 8.2 gives the JSON of these faults, and the Rust harness is their meaning
(``V1_FAULTS`` and ``scalar_adapter_v1`` in ``rust/warrant-grammar/src/evidence.rs``)::

    saturate-add   Replace("p0.saturating_add(p1)")     the export is NOT called; Ok(value) on every input
    saturate-sub   Replace("p0.saturating_sub(p1)")
    shift-mask     PreLast("p % {w}")                   the last argument becomes p % w, w = width of the FIRST

These tests pin (1) the exact JSON, (2) that each form evaluates to a Python model of the Rust rule (a clamp;
the truncating ``%``) on the whole U8 and I8 domains and on the edges of the wider types, (3) levels, kinds, types
and costs, (4) every refusal, (5) the 22 ``missing-error`` and 21 saturating or shifting rows of logic-gates, and
(6) the committed dump of ``fixtures/scalar-faults-v1``.
"""

from __future__ import annotations

import itertools
import json
import pathlib
import subprocess
import sys

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.faults import REGISTRY, fault_row
from coding_adventures_warrant_oracle.ir_eval import Program
from coding_adventures_warrant_oracle.lower import FIXED, lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.verify_v2 import parse_bindings_faults

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "scalar-faults-v1"
CODE = HERE.parents[3]
LOGIC_GATES_SPEC = CODE / "specs" / "logic-gates.warrant"
LOGIC_GATES_BINDINGS = CODE / "packages" / "rust" / "logic-gates" / "evidence.bindings"

TYPES = list(FIXED)

SOURCE = (
    "warrant f v1\n  error E = A | B\n"
    + "".join(
        f"  op s_{t}(a: {t}, b: {t}) -> {t}\n  op r_{t}(a: {t}, b: {t}, c: Bool) -> Result<{t},E>\n" for t in TYPES
    )
    + "  op sh_u8(a: U8, n: U8) -> U8\n"
    "  op sh_u32_u8(a: U32, n: U8) -> U8\n"
    "  op sh_i16_u8(a: I16, n: U8) -> U8\n"
    "  op sh_u8_i8(a: U8, n: I8) -> I8\n"
    "  op sh_i64_i32(a: I64, n: I32) -> I32\n"
    "  op sh_one(a: U16) -> U16\n"
    "  op sh_first_of_three(a: U64, x: Bool, n: I16) -> Bool\n"
    "  op me_u8(a: U8, b: U16) -> Result<U8,E>\n"
    "  op me_second(a: U16, b: U8) -> Result<U8,E>\n"
    "  op me_two(a: U8, b: U8) -> Result<U8,E>\n"
    "  op me_zero_u16(a: U8) -> Result<U16,E>\n"
    "  op me_i8(a: U8, b: I8) -> Result<I8,E>\n"
    "  op me_bool(a: U8, f: Bool) -> Result<Bool,E>\n"
    "  op me_bool_zero(a: U8) -> Result<Bool,E>\n"
    "  op me_nat(a: Bool, n: Nat) -> Result<Nat,E>\n"
    "  op me_nat_zero(a: Bool) -> Result<Nat,E>\n"
    "  op me_bytes_zero(a: U8) -> Result<Bytes,E>\n"
    "  op me_bytes_n_zero(a: U8) -> Result<Bytes<4>,E>\n"
    "  op me_bytes_n(d: Bytes<2>) -> Result<Bytes<4>,E>\n"
    "  op me_bytes(a: U8, d: Bytes) -> Result<Bytes,E>\n"
)

SPEC = lower_document(parse_source(SOURCE.encode()))
PROG = Program({}, SPEC.ops, errors=SPEC.errors)


def row(op: str, fault: str) -> dict:
    return fault_row(SPEC.ops[op], fault, SPEC.errors)


def compile_expr(op: str, expr):
    tenv = {p: t for p, t in SPEC.ops[op].params}
    tenv["result"] = SPEC.ops[op].ret
    return PROG.compile(expr, tenv, False)


def run_replace(op: str, fault: str, result=("err", "A"), **params):
    return compile_expr(op, row(op, fault)["expr"]).fn({**params, "result": result})


def compact(value) -> str:
    return json.dumps(value, separators=(",", ":"))


# -- Python models of the Rust harness --------------------------------------------------------------------------


def bounds(ty: str) -> tuple[int, int]:
    width, signed = FIXED[ty]
    return (-(1 << (width - 1)), (1 << (width - 1)) - 1) if signed else (0, (1 << width) - 1)


def saturating(ty: str, exact: int) -> int:
    """Rust's ``saturating_add`` / ``saturating_sub``: the exact value, or the nearer bound of the type."""
    lo, hi = bounds(ty)
    return min(max(exact, lo), hi)


def rust_rem(n: int, w: int) -> int:
    """Rust's ``%``: the sign of the dividend (truncating), which is Python's ``%`` only for ``n >= 0``."""
    r = abs(n) % w
    return -r if n < 0 else r


def edges(ty: str) -> list[int]:
    lo, hi = bounds(ty)
    pts = {lo, lo + 1, lo + 2, hi, hi - 1, hi - 2, 0, 1, 2, 3, 7, 8, 9, 63, 64, 65, 127, 128, 129, 255, 256}
    pts |= {-1, -2, -8, -64, -127, -128, -129, hi // 2, lo // 2, hi // 2 + 1, lo // 2 - 1}
    return sorted(p for p in pts if lo <= p <= hi)


def domain(ty: str) -> list[int]:
    lo, hi = bounds(ty)
    return list(range(lo, hi + 1)) if FIXED[ty][0] == 8 else edges(ty)


# -- the exact JSON of VER04r 8.2 --------------------------------------------------------------------------------


def test_the_two_unsigned_u8_rows_are_the_json_of_the_spec():
    assert compact(row("r_U8", "saturate-add")["expr"]) == (
        '["ok",["unwrap_or",["checked_add",["var","a"],["var","b"]],["max_value","U8"]]]'
    )
    assert compact(row("r_U8", "saturate-sub")["expr"]) == (
        '["ok",["unwrap_or",["checked_sub",["var","a"],["var","b"]],["min_value","U8"]]]'
    )


def test_the_bare_forms_are_v_and_the_result_forms_are_ok_of_v():
    for fault in ("saturate-add", "saturate-sub"):
        for t in TYPES:
            assert row(f"r_{t}", fault)["expr"] == ["ok", row(f"s_{t}", fault)["expr"]]


def test_the_signed_bounds_are_chosen_by_the_sign_of_the_second_operand():
    neg = ["lt", ["var", "b"], ["lit", {"type": "I8", "value": "0"}]]
    mx, mn = ["max_value", "I8"], ["min_value", "I8"]
    ck = lambda tag: [tag, ["var", "a"], ["var", "b"]]  # noqa: E731
    assert row("s_I8", "saturate-add")["expr"] == ["unwrap_or", ck("checked_add"), ["if", neg, mn, mx]]
    assert row("s_I8", "saturate-sub")["expr"] == ["unwrap_or", ck("checked_sub"), ["if", neg, mx, mn]]


def test_the_unsigned_sub_bound_is_min_value_not_a_zero_literal():
    assert row("s_U32", "saturate-sub")["expr"][2] == ["min_value", "U32"]
    assert row("s_U32", "saturate-add")["expr"][2] == ["max_value", "U32"]


def test_the_parameters_are_the_first_two_whatever_their_names_and_count():
    s = lower_document(parse_source(b"warrant z v1\n  error E = A\n  op f(x: U16, y: U16, z: U8, w: Bool) -> Result<U16,E>\n"))
    expr = fault_row(s.ops["f"], "saturate-add", s.errors)["expr"]
    assert expr == ["ok", ["unwrap_or", ["checked_add", ["var", "x"], ["var", "y"]], ["max_value", "U16"]]]


def test_shift_mask_rows_are_the_json_of_the_spec():
    r = row("sh_u8", "shift-mask")
    assert r["kind"] == "pre" and r["fault"] == "shift-mask" and r["op"] == "sh_u8"
    assert compact(r["rewrites"]) == (
        '[{"param":"n","expr":["unwrap_or",["rem",["var","n"],["lit",{"type":"U8","value":"8"}]],'
        '["lit",{"type":"U8","value":"0"}]]}]'
    )


def test_shift_mask_width_is_the_first_parameters_and_the_literal_is_in_the_lasts_type():
    def lit(r):
        return r["rewrites"][0]["expr"][1][2]

    assert lit(row("sh_u32_u8", "shift-mask")) == ["lit", {"type": "U8", "value": "32"}]
    assert lit(row("sh_i16_u8", "shift-mask")) == ["lit", {"type": "U8", "value": "16"}]
    assert lit(row("sh_u8_i8", "shift-mask")) == ["lit", {"type": "I8", "value": "8"}]
    assert lit(row("sh_i64_i32", "shift-mask")) == ["lit", {"type": "I32", "value": "64"}]


def test_a_signed_last_parameter_uses_rem_trunc_and_an_unsigned_one_rem():
    assert row("sh_u8_i8", "shift-mask")["rewrites"][0]["expr"][1][0] == "rem_trunc"
    assert row("sh_i64_i32", "shift-mask")["rewrites"][0]["expr"][1][0] == "rem_trunc"
    assert row("sh_u8", "shift-mask")["rewrites"][0]["expr"][1][0] == "rem"


def test_with_one_parameter_the_first_is_also_the_last_and_with_three_the_last_is_rewritten():
    r = row("sh_one", "shift-mask")
    assert r["rewrites"][0]["param"] == "a"
    assert r["rewrites"][0]["expr"][1][2] == ["lit", {"type": "U16", "value": "16"}]
    r3 = row("sh_first_of_three", "shift-mask")  # the result type does not matter to a pre fault
    assert r3["rewrites"][0]["param"] == "n"
    assert r3["rewrites"][0]["expr"][1][2] == ["lit", {"type": "I16", "value": "64"}]


# -- kinds, levels ----------------------------------------------------------------------------------------------


def test_levels_are_ir_fixed_alone_in_every_combination_and_the_rows_are_live():
    for t in TYPES:
        for fault in ("saturate-add", "saturate-sub"):
            for op in (f"s_{t}", f"r_{t}"):
                r = row(op, fault)
                assert r["requires"] == ["ir.fixed"] and r["kind"] == "replace"
    for op in ("sh_u8", "sh_u8_i8", "sh_one", "sh_i64_i32"):
        assert row(op, "shift-mask")["requires"] == ["ir.fixed"]


def test_scalar_missing_error_requires_ir_core_only():
    for op in ("me_u8", "me_zero_u16", "me_i8", "me_bool", "me_bool_zero", "me_nat", "me_nat_zero", "me_bytes_zero"):
        r = row(op, "missing-error")
        assert r["requires"] == ["ir.core"] and r["kind"] == "replace"


# -- types and costs ---------------------------------------------------------------------------------------------


@pytest.mark.parametrize("ty", TYPES)
def test_costs_and_types_match_the_spec(ty):
    signed = FIXED[ty][1]
    for fault in ("saturate-add", "saturate-sub"):
        bare = compile_expr(f"s_{ty}", row(f"s_{ty}", fault)["expr"])
        wrapped = compile_expr(f"r_{ty}", row(f"r_{ty}", fault)["expr"])
        assert bare.cost == (9 if signed else 5)
        assert wrapped.cost == bare.cost + 1
        assert str(bare.ty) == ty
        assert wrapped.ty.head == "Result" and wrapped.ty.args[0] == A.Ty(ty)


def test_shift_mask_costs_5_with_exactly_the_type_of_the_last_parameter():
    for op, last in (("sh_u8", "U8"), ("sh_u32_u8", "U8"), ("sh_u8_i8", "I8"), ("sh_i64_i32", "I32"), ("sh_one", "U16")):
        x = row(op, "shift-mask")["rewrites"][0]["expr"]
        c = compile_expr(op, x)
        assert c.cost == 5 and str(c.ty) == last


def test_scalar_missing_error_costs_5():
    for op in ("me_u8", "me_zero_u16", "me_bool", "me_bool_zero", "me_nat", "me_nat_zero", "me_bytes_zero", "me_i8"):
        assert compile_expr(op, row(op, "missing-error")["expr"]).cost == 5


# -- evaluation against the model of the harness -------------------------------------------------------------


@pytest.mark.parametrize("ty", TYPES)
def test_saturate_add_and_sub_equal_the_clamp_of_the_exact_value(ty):
    dom = domain(ty)
    fns = {
        (fault, form): compile_expr(f"{form}_{ty}", row(f"{form}_{ty}", fault)["expr"]).fn
        for fault in ("saturate-add", "saturate-sub")
        for form in ("s", "r")
    }
    for a, b in itertools.product(dom, dom):
        add, sub = saturating(ty, a + b), saturating(ty, a - b)
        env = {"a": a, "b": b, "c": True, "result": ("err", "A")}
        assert fns["saturate-add", "s"](env) == add
        assert fns["saturate-sub", "s"](env) == sub
        assert fns["saturate-add", "r"](env) == ("ok", add)
        assert fns["saturate-sub", "r"](env) == ("ok", sub)


def test_the_u8_and_i8_domains_were_exhaustive():
    assert len(domain("U8")) == 256 and len(domain("I8")) == 256


def test_the_result_form_returns_ok_unconditionally_whatever_the_call_returned():
    """Under a ``Replace`` fault the harness never calls the export (``scalar_adapter_v1``): ``Ok`` on every input."""
    for result in (("err", "A"), ("err", "B"), ("ok", 0), ("ok", 255)):
        assert run_replace("r_U8", "saturate-add", result, a=250, b=10, c=False) == ("ok", 255)
        assert run_replace("r_U8", "saturate-sub", result, a=3, b=10, c=False) == ("ok", 0)
        assert run_replace("r_I8", "saturate-add", result, a=-100, b=-100, c=True) == ("ok", -128)
        assert run_replace("r_I8", "saturate-sub", result, a=-100, b=100, c=True) == ("ok", -128)
        assert run_replace("r_I8", "saturate-sub", result, a=100, b=-100, c=True) == ("ok", 127)


@pytest.mark.parametrize(
    ("op", "ty"),
    [
        ("sh_u8", "U8"),
        ("sh_u32_u8", "U8"),
        ("sh_i16_u8", "U8"),
        ("sh_u8_i8", "I8"),
        ("sh_i64_i32", "I32"),
        ("sh_one", "U16"),
    ],
)
def test_shift_mask_equals_rust_remainder_by_the_first_parameters_width(op, ty):
    sig = SPEC.ops[op]
    w = FIXED[sig.params[0][1].head][0]
    pname = sig.params[-1][0]
    fn = compile_expr(op, row(op, "shift-mask")["rewrites"][0]["expr"]).fn
    other = {n: 0 for n, _ in sig.params}
    for n in domain(ty if op != "sh_one" else "U16"):
        assert fn({**other, pname: n}) == rust_rem(n, w), (op, n)


def test_a_signed_remainder_keeps_the_sign_of_the_dividend_unlike_python():
    fn = compile_expr("sh_u8_i8", row("sh_u8_i8", "shift-mask")["rewrites"][0]["expr"]).fn
    assert fn({"a": 0, "n": -1}) == -1  # Python's -1 % 8 would be 7
    assert fn({"a": 0, "n": -9}) == -1
    assert fn({"a": 0, "n": -128}) == 0
    assert fn({"a": 0, "n": 127}) == 7


def test_the_shift_divisor_never_takes_the_default_and_the_rewrite_never_leaves_its_type():
    for op in ("sh_u8_i8", "sh_i64_i32", "sh_u32_u8"):
        x = row(op, "shift-mask")["rewrites"][0]["expr"]
        fn = compile_expr(op, x).fn
        ty = str(compile_expr(op, x).ty)
        lo, hi = bounds(ty)
        for n in edges(ty):
            out = fn({"a": 0, "n": n})
            assert lo <= out <= hi and abs(out) < FIXED[SPEC.ops[op].params[0][1].head][0]


# -- scalar missing-error --------------------------------------------------------------------------------------


def test_a_scalar_payload_is_the_first_parameter_of_exactly_that_type_else_the_zero():
    def expr(op):
        return row(op, "missing-error")["expr"]

    def ok(p):
        return ["if", ["is_err", ["var", "result"]], ["ok", p], ["var", "result"]]

    assert expr("me_u8") == ok(["var", "a"])
    assert expr("me_second") == ok(["var", "b"])  # a: U16 is not U8: the first parameter of exactly the type
    assert expr("me_two") == ok(["var", "a"])  # two candidates: the first
    assert expr("me_i8") == ok(["var", "b"])
    assert expr("me_bool") == ok(["var", "f"])
    assert expr("me_nat") == ok(["var", "n"])
    assert expr("me_zero_u16") == ok(["lit", {"type": "U16", "value": "0"}])
    assert expr("me_bool_zero") == ok(["lit", False])
    assert expr("me_nat_zero") == ok(["lit", {"type": "Nat", "value": "0"}])


def test_scalar_missing_error_evaluates_to_ok_of_the_payload_on_an_err_and_leaves_an_ok_alone():
    assert run_replace("me_u8", "missing-error", ("err", "A"), a=7, b=9) == ("ok", 7)
    assert run_replace("me_u8", "missing-error", ("ok", 3), a=7, b=9) == ("ok", 3)
    assert run_replace("me_second", "missing-error", ("err", "B"), a=1000, b=9) == ("ok", 9)
    assert run_replace("me_zero_u16", "missing-error", ("err", "A"), a=5) == ("ok", 0)
    assert run_replace("me_bool_zero", "missing-error", ("err", "A"), a=5) == ("ok", False)
    assert run_replace("me_nat_zero", "missing-error", ("err", "A"), a=True) == ("ok", 0)
    assert run_replace("me_bytes_zero", "missing-error", ("err", "A"), a=5) == ("ok", b"")


def test_the_bytes_payload_keeps_its_looser_match_and_gains_a_zero():
    """16.14 keeps the top-level ``Bytes`` rule (any ``Bytes<M>`` parameter); with none, the zero literal."""
    assert row("me_bytes", "missing-error")["expr"][2] == ["ok", ["var", "d"]]
    assert row("me_bytes_n", "missing-error")["expr"][2] == ["ok", ["var", "d"]]  # Bytes<2> taken for Bytes<4>
    assert row("me_bytes_zero", "missing-error")["expr"][2] == ["ok", ["lit", {"type": "Bytes", "value": ""}]]
    assert row("me_bytes_n_zero", "missing-error")["expr"][2] == ["ok", ["lit", {"type": "Bytes", "value": "00000000"}]]
    assert row("me_bytes", "missing-error")["requires"] == ["ir.core"]


def test_the_tuple_payload_form_is_unchanged():
    s = lower_document(parse_source(b"warrant z v1\n  error E = A\n  op f(a: Bool, n: Nat) -> Result<(Bool,Nat),E>\n"))
    r = fault_row(s.ops["f"], "missing-error", s.errors)
    assert r["requires"] == ["ir.tuple"]
    assert r["expr"] == ["if", ["is_err", ["var", "result"]], ["ok", ["tuple", [["var", "a"], ["var", "n"]]]], ["var", "result"]]


# -- refusals: the code `fault`, nothing skipped ----------------------------------------------------------------

SATURATE_REFUSED = [
    "op f(a: Nat, b: U8) -> U8",
    "op f(a: U8, b: Nat) -> U8",
    "op f(a: Nat, b: Nat) -> Nat",
    "op f(a: U8, b: U16) -> U8",
    "op f(a: U8, b: I8) -> U8",
    "op f(a: U8) -> U8",
    "op f() -> U8",
    "op f(a: Bool, b: Bool) -> Bool",
    "op f(a: Bytes, b: Bytes) -> Bytes",
    "op f(a: U8, b: U8) -> Bool",
    "op f(a: U8, b: U8) -> Nat",
    "op f(a: U8, b: U8) -> U16",
    "op f(a: U8, b: U8) -> Option<U8>",
    "op f(a: U8, b: U8) -> Bytes",
    "op f(a: U8, b: U8) -> (U8,U8)",
    "op f(a: U8, b: U8) -> Result<Bool,E>",
    "op f(a: U8, b: U8) -> Result<U16,E>",
    "op f(a: U8, b: U8) -> Result<(U8,U8),E>",
    "op f(a: U8, b: U8) -> Result<Bytes,E>",
]
SHIFT_REFUSED = [
    "op f(a: Nat, n: U8) -> U8",
    "op f(a: U8, n: Nat) -> U8",
    "op f(n: Nat) -> U8",
    "op f(a: U8, n: Bool) -> U8",
    "op f(a: Bool, n: U8) -> U8",
    "op f(a: U8, n: Bytes) -> U8",
    "op f(a: Bytes, n: U8) -> U8",
    "op f() -> U8",
]
MISSING_ERROR_REFUSED = [
    "op f(a: U8) -> U8",
    "op f(a: U8) -> Bool",
    "op f(a: U8) -> Bytes",
    "op f(a: U8) -> Option<U8>",
    "op f(a: U8) -> Result<Option<U8>,E>",  # ir.data: stays refused until the Option payload slice (S11c)
    "variant K = P | Q\n  op f(a: U8) -> Result<K,E>",  # a variant payload has no zero and no parameter of its type
    "op f<N: Nat>(a: U8) -> Result<Bytes<N>,E>",  # a named width has no zero literal
]


def refused(decl: str, fault: str) -> OracleError:
    s = lower_document(parse_source(f"warrant r v1\n  error E = A | B\n  {decl}\n".encode()))
    with pytest.raises(OracleError) as err:
        fault_row(s.ops["f"], fault, s.errors)
    return err.value


@pytest.mark.parametrize("fault", ["saturate-add", "saturate-sub"])
@pytest.mark.parametrize("decl", SATURATE_REFUSED)
def test_saturating_shapes_without_a_template_are_refused_with_the_fault_code(decl, fault):
    e = refused(decl, fault)
    assert e.code == "generation" and e.message.startswith(f"fault: {fault}")


@pytest.mark.parametrize("decl", SHIFT_REFUSED)
def test_shift_mask_shapes_without_a_template_are_refused_with_the_fault_code(decl):
    e = refused(decl, "shift-mask")
    assert e.code == "generation" and e.message.startswith("fault: shift-mask")


@pytest.mark.parametrize("decl", MISSING_ERROR_REFUSED)
def test_missing_error_shapes_without_a_template_are_refused_with_the_fault_code(decl):
    assert refused(decl, "missing-error").message.startswith("fault:")


# -- logic-gates: the 21 saturating and shifting rows and the 22 missing-error rows -------------------------------


def logic_gates():
    if not LOGIC_GATES_SPEC.exists() or not LOGIC_GATES_BINDINGS.exists():
        pytest.skip("logic-gates is not checked out")
    spec = lower_document(parse_source(LOGIC_GATES_SPEC.read_bytes()))
    return spec, parse_bindings_faults(LOGIC_GATES_BINDINGS.read_bytes())


def test_every_bound_logic_gates_row_of_the_four_faults_instantiates_and_compiles_except_the_option_payload():
    spec, bound = logic_gates()
    four = {"saturate-add", "saturate-sub", "shift-mask", "missing-error"}
    instantiated: dict[str, list[str]] = {f: [] for f in sorted(four)}
    refused_rows: list[tuple[str, str]] = []
    for op, faults in sorted(bound.items()):
        for fault in sorted(faults & four):
            try:
                r = fault_row(spec.ops[op], fault, spec.errors)
            except OracleError as exc:
                assert exc.code == "generation" and exc.message.startswith("fault:")
                refused_rows.append((op, fault))
                continue
            instantiated[fault].append(op)
            tenv = {p: t for p, t in spec.ops[op].params}
            tenv["result"] = spec.ops[op].ret
            prog = Program({}, spec.ops, errors=spec.errors)
            for expr in [r["expr"]] if "expr" in r else [w["expr"] for w in r["rewrites"]]:
                prog.compile(expr, tenv, False)  # well typed and within the cost limits
    assert len(instantiated["saturate-add"]) == 10 and len(instantiated["saturate-sub"]) == 9
    assert len(instantiated["shift-mask"]) == 2
    assert sum(len(instantiated[f]) for f in ("saturate-add", "saturate-sub", "shift-mask")) == 21
    # 22 bound missing-error rows: 17 scalar `Result<U8,E>`, 3 `Result<Bytes,E>`, the tuple of priority_encoder
    # (all instantiated, as before) and tri_state's `Result<Option<U8>,E>`, which is ir.data and stays refused.
    assert len(instantiated["missing-error"]) == 21
    assert refused_rows == [("tri_state", "missing-error")]


def test_the_logic_gates_rows_are_the_json_of_the_spec():
    spec, _ = logic_gates()
    add = fault_row(spec.ops["and_gate"], "saturate-add", spec.errors)["expr"]
    sub = fault_row(spec.ops["and_gate"], "saturate-sub", spec.errors)["expr"]
    assert compact(add) == '["ok",["unwrap_or",["checked_add",["var","a"],["var","b"]],["max_value","U8"]]]'
    assert compact(sub) == '["ok",["unwrap_or",["checked_sub",["var","a"],["var","b"]],["min_value","U8"]]]'
    mux = fault_row(spec.ops["mux2"], "saturate-add", spec.errors)["expr"]
    assert mux == ["ok", ["unwrap_or", ["checked_add", ["var", "d0"], ["var", "d1"]], ["max_value", "U8"]]]
    for op in ("not_gate", "nand_not"):
        rw = fault_row(spec.ops[op], "shift-mask", spec.errors)["rewrites"]
        assert compact(rw) == (
            '[{"param":"a","expr":["unwrap_or",["rem",["var","a"],["lit",{"type":"U8","value":"8"}]],'
            '["lit",{"type":"U8","value":"0"}]]}]'
        )


def test_the_logic_gates_missing_error_payloads():
    spec, _ = logic_gates()

    def payload(op):
        return fault_row(spec.ops[op], "missing-error", spec.errors)["expr"][2][1]

    assert payload("and_gate") == ["var", "a"]
    assert payload("not_gate") == ["var", "a"]
    assert payload("mux4") == ["var", "d0"]
    assert payload("and_n") == ["lit", {"type": "U8", "value": "0"}]  # the first U8 parameter, else the zero
    assert payload("demux") == ["var", "sel"]  # Bytes payload: the first Bytes parameter
    assert payload("decoder") == ["var", "inputs"]


def test_the_logic_gates_bound_faults_are_all_in_the_registry():
    _, bound = logic_gates()
    names = {f for fs in bound.values() for f in fs}
    assert {"saturate-add", "saturate-sub", "shift-mask", "missing-error"} <= set(REGISTRY)
    assert names - set(REGISTRY) == set()


# -- the committed dump -----------------------------------------------------------------------------------------


def test_the_committed_dump_is_the_oracles_output():
    out = subprocess.run(
        [sys.executable, str(FIXTURE / "make_scalar_faults.py"), "--check"], capture_output=True, text=True
    )
    assert out.returncode == 0, out.stderr


def test_the_committed_faults_dump_holds_every_row_live_with_witnesses():
    dump = json.loads((FIXTURE / "scalar_faults_v1.oracle-faults.json").read_text(encoding="utf-8"))
    rows = {(f["op"], f["fault"]): f for f in dump["faults"]}
    assert {f for _op, f in rows} == {"saturate-add", "saturate-sub", "shift-mask", "missing-error"}
    assert compact(rows["satr_u8", "saturate-add"]["expr"]) == (
        '["ok",["unwrap_or",["checked_add",["var","a"],["var","b"]],["max_value","U8"]]]'
    )
    assert all(r["requires"] in (["ir.fixed"], ["ir.core"], ["ir.fixed", "ir.tuple"]) for r in rows.values())
    for control in dump["controls_gate_on"]:
        assert control["required_claims"] and not control["blind_claims"], control["id"]
        assert all("deferred" not in w for w in control["slim_witness"])


def test_the_committed_refusals_dump_names_the_fault_code_for_every_row():
    dump = json.loads((FIXTURE / "scalar_faults_v1.oracle-refusals.json").read_text(encoding="utf-8"))
    assert dump["schema"] == "warrant-faults-refusals-v2"
    assert dump["faults"] == ["saturate-add", "saturate-sub", "shift-mask", "missing-error"]
    assert len(dump["refusals"]) == 2 * len(SATURATE_REFUSED) + len(SHIFT_REFUSED) + len(MISSING_ERROR_REFUSED)
    assert all(r["oracle_code"] == "generation" and r["message"].startswith("fault:") for r in dump["refusals"])


def test_the_committed_dump_pins_the_widened_shapes():
    dump = json.loads((FIXTURE / "scalar_faults_v1.oracle-faults.json").read_text(encoding="utf-8"))
    rows = {(f["op"], f["fault"]): f for f in dump["faults"]}
    assert rows["me_bytes4_zero", "missing-error"]["expr"][2] == ["ok", ["lit", {"type": "Bytes", "value": "00000000"}]]
    # R9: a tuple parameter adds ir.tuple to the row's requires
    assert rows["sat_u8_tuple_param", "saturate-add"]["requires"] == ["ir.fixed", "ir.tuple"]
    controls = {c["id"].split("/negctl/exhaustive/")[1]: c for c in dump["controls_gate_on"]}
    assert len(controls["me_u8/missing-error"]["required_claims"]) == 2  # the Ok-side and the Err-side claim
    assert len(dump["controls_gate_on"]) == 41
    refusals = json.loads((FIXTURE / "scalar_faults_v1.oracle-refusals.json").read_text(encoding="utf-8"))
    assert any(r["name"] == "a-variant-payload" for r in refusals["refusals"])
