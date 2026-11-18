"""IR evaluator: VER04r section 3 wrapping, bounds, typing, call counting."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import (
    EXACT_BOUND,
    MAX_IR_COST,
    MAX_IR_VALUE,
    ModelDef,
    Program,
    compile_calls,
    decode_int_text,
    has_subject,
    wrap_to,
)
from coding_adventures_warrant_oracle.lower import OpSig

T = A.Ty


def lit(ty: str, value: object) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


def run(ir, tenv=None, env=None, prog=None):
    prog = prog or Program({}, {})
    return prog.compile(ir, tenv or {}, allow_subject=False).fn(env or {})


# -- section 3.4: the conformance cases that pin the wrapping rule ----------------


@pytest.mark.parametrize(
    "tag,ty,a,b,expected",
    [
        ("add", "U8", 255, 1, 0),  # arith/u8-add-wrap
        ("sub", "U8", 0, 1, 255),  # arith/u8-sub-wrap
        ("mul", "U8", 16, 16, 0),  # arith/u8-mul-wrap
        ("add", "U64", (1 << 64) - 1, 1, 0),  # arith/u64-add-wrap
        ("mul", "U64", (1 << 64) - 1, (1 << 64) - 1, 1),  # arith/u64-mul-wrap: MAX * MAX = 1
        ("add", "I8", 127, 1, -128),
        ("sub", "I64", -(1 << 63), 1, (1 << 63) - 1),  # arith/i64-sub-wrap
        ("add", "U16", 65535, 2, 1),
        ("mul", "U32", 65536, 65536, 0),
        ("sub", "I8", -128, 1, 127),
        ("mul", "I16", -32768, -1, -32768),
        ("add", "I32", (1 << 31) - 1, 1, -(1 << 31)),
        ("add", "U8", 3, 4, 7),  # no wrap: unchanged
    ],
)
def test_binary_arithmetic_wraps_modulo_two_to_w(tag, ty, a, b, expected):
    assert run([tag, lit(ty, a), lit(ty, b)]) == expected


def test_i8_negation_of_the_minimum_wraps_to_itself():  # arith/i8-neg-min
    assert run(["neg", lit("I8", -128)]) == -128
    assert run(["neg", lit("I8", 5)]) == -5
    assert run(["neg", lit("U8", 1)]) == 255  # an unsigned neg wraps (section 3.2)
    assert run(["neg", lit("I64", -(1 << 63))]) == -(1 << 63)


def test_nat_subtraction_is_an_int():  # arith/nat-sub-is-int: 3 - 5 = -2 as Int
    assert run(["sub", lit("Nat", 3), lit("Nat", 5)]) == -2
    assert run(["add", lit("Nat", 3), lit("Int", 5)]) == 8
    assert run(["neg", lit("Nat", 4)]) == -4


def test_wrap_helper_matches_the_table():
    assert wrap_to(-1, 8, False) == 255 and wrap_to(255, 8, True) == -1
    assert wrap_to(1 << 70, 64, False) == 0


def test_wraps_counter_counts_only_nodes_whose_wrapped_result_differs():
    prog = Program({}, {})
    prog.stats.counting = True
    assert run(["add", lit("U8", 3), lit("U8", 4)], prog=prog) == 7
    assert prog.stats.wraps == 0
    run(["add", lit("U8", 255), lit("U8", 1)], prog=prog)
    run(["neg", lit("I8", -128)], prog=prog)
    run(["sub", lit("U8", 0), lit("U8", 1)], prog=prog)
    assert prog.stats.wraps == 3


def test_nat_node_outside_128_bits_is_ir_overflow():  # arith/nat-bound-generation-error
    big = lit("Int", EXACT_BOUND)
    with pytest.raises(OracleError) as err:
        run(["add", big, lit("Int", 1)])
    assert err.value.code == "ir-overflow"
    with pytest.raises(OracleError):
        run(["mul", big, lit("Int", 2)])  # the product needs 128+ bits
    with pytest.raises(OracleError):
        run(["neg", ["sub", lit("Int", 0), big]]) if False else run(
            ["sub", lit("Int", -EXACT_BOUND), lit("Int", 1)]
        )
    assert run(["add", lit("Int", EXACT_BOUND - 1), lit("Int", 1)]) == EXACT_BOUND


def test_a_product_of_two_in_range_values_is_checked_not_wrapped():
    a = lit("Int", 1 << 100)
    with pytest.raises(OracleError) as err:
        run(["mul", a, a])
    assert err.value.code == "ir-overflow"


# -- literals ----------------------------------------------------------------------------


@pytest.mark.parametrize(
    "ty,text",
    [
        ("U8", "256"),
        ("U8", "-1"),
        ("U8", "01"),
        ("U8", "+1"),
        ("U8", ""),
        ("U8", "1.0"),
        ("I8", "128"),
        ("I8", "-129"),
        ("I8", "-0"),
        ("U64", "18446744073709551616"),
        ("Nat", "18446744073709551616"),
        ("Nat", "-1"),
        ("U8", " 1"),
        ("U8", "１"),
    ],
)
def test_literal_decoding_rejects_the_section_2_3_hazards(ty, text):
    with pytest.raises(OracleError):
        decode_int_text(text, ty)


def test_literal_decoding_accepts_the_extremes():
    assert decode_int_text("18446744073709551615", "U64") == (1 << 64) - 1
    assert decode_int_text("-128", "I8") == -128
    assert decode_int_text("0", "U8") == 0
    assert decode_int_text("-5", "Int") == -5
    with pytest.raises(OracleError):
        decode_int_text(5, "U8")
    with pytest.raises(OracleError):
        decode_int_text("5", "Bool")
    with pytest.raises(OracleError):
        decode_int_text("9" * 60, "Int")


@pytest.mark.parametrize(
    "ir",
    [
        ["lit"],
        ["lit", 5],
        ["lit", {"type": "U8"}],
        ["lit", {"type": "Bytes", "value": "0"}],
        ["lit", {"type": "Bytes", "value": "AB"}],
        ["lit", {"type": "Float", "value": "1"}],
        ["lit", {"type": "Bytes", "value": "00" * (MAX_IR_VALUE + 1)}],
        "lit",
        [],
        [5],
        ["var"],
        ["var", 3],
        ["var", "nope"],
        ["len"],
        ["add", ["lit", True]],
        ["if", ["lit", True], ["lit", True]],
        ["err"],
        ["err", 5],
    ],
)
def test_malformed_ir_is_an_error(ir):
    with pytest.raises(OracleError):
        run(ir)


def test_unknown_tags_are_errors_never_skipped():
    with pytest.raises(OracleError) as err:
        run(["frobnicate", ["lit", True]])
    assert err.value.code == "ir"


@pytest.mark.parametrize(
    "tag,level",
    [
        ("shl", "ir.fixed"),
        ("wrap", "ir.fixed"),
        ("bit_and", "ir.fixed"),
        ("at", "ir.fixed"),
        ("try", "ir.fixed"),
        ("unwrap_or", "ir.fixed"),
        ("max_value", "ir.fixed"),
        ("some", "ir.data"),
        ("fold", "ir.seq"),
        ("all", "ir.seq"),
    ],
)
def test_tags_of_other_levels_are_named_not_evaluated(tag, level):
    with pytest.raises(OracleError) as err:
        run([tag, ["lit", True]])
    assert err.value.code == "level" and err.value.level == level


# -- evaluation rules ---------------------------------------------------------------------


def test_short_circuit_does_not_evaluate_the_untaken_operand():
    boom = ["add", lit("Int", EXACT_BOUND), lit("Int", EXACT_BOUND)]  # would be ir-overflow
    guard = ["eq", boom, lit("Int", 0)]
    assert run(["and", ["lit", False], guard]) is False
    assert run(["or", ["lit", True], guard]) is True
    assert run(["implies", ["lit", False], guard]) is True
    assert run(["if", ["lit", True], ["lit", True], guard]) is True
    with pytest.raises(OracleError):
        run(["and", ["lit", True], guard])


def test_comparisons_use_mathematical_value():
    assert run(["lt", lit("I8", -1), lit("I8", 1)]) is True
    assert run(["gt", lit("U8", 255), lit("U8", 0)]) is True
    assert run(["le", lit("Nat", 2), lit("Int", 2)]) is True
    assert run(["ge", lit("Nat", 2), lit("Int", 3)]) is False
    assert run(["ne", ["lit", True], ["lit", False]]) is True
    assert run(["eq", lit("Nat", 2), lit("Int", 2)]) is True


def test_bool_is_not_an_integer():
    with pytest.raises(OracleError):
        run(["eq", ["lit", True], lit("U8", 1)])
    with pytest.raises(OracleError):
        run(["lt", ["lit", True], ["lit", False]])
    with pytest.raises(OracleError):
        run(["add", ["lit", True], ["lit", True]])
    with pytest.raises(OracleError):
        run(["not", lit("U8", 1)])
    with pytest.raises(OracleError):
        run(["and", lit("U8", 1), ["lit", True]])


def test_fixed_width_never_mixes_with_int_or_other_widths():
    with pytest.raises(OracleError):
        run(["add", lit("U8", 1), lit("U16", 1)])
    with pytest.raises(OracleError):
        run(["add", lit("U8", 1), lit("Int", 1)])
    with pytest.raises(OracleError):
        run(["lt", lit("U8", 1), lit("Int", 1)])


def test_bytes_operations_are_total():
    b = lambda h: ["lit", {"type": "Bytes", "value": h}]  # noqa: E731
    assert run(["concat", b("01"), b("0203")]) == b"\x01\x02\x03"
    assert run(["take", b("010203"), lit("Nat", 2)]) == b"\x01\x02"
    assert run(["take", b("010203"), lit("Nat", 9)]) == b"\x01\x02\x03"  # clamps
    assert run(["drop", b("010203"), lit("Nat", 1)]) == b"\x02\x03"
    assert run(["drop", b("010203"), lit("Nat", 9)]) == b""
    assert run(["take", b("010203"), ["neg", lit("Nat", 1)]]) == b""  # a negative count is 0
    assert run(["drop", b("010203"), ["neg", lit("Nat", 1)]]) == b"\x01\x02\x03"
    assert run(["len", b("0102")]) == 2
    with pytest.raises(OracleError):
        run(["len", lit("U8", 1)])
    with pytest.raises(OracleError):
        run(["concat", lit("U8", 1), b("00")])
    with pytest.raises(OracleError):
        run(["take", b("00"), ["lit", True]])


def test_result_values_and_is_err():
    ok = ["ok", lit("U8", 1)]
    err = ["err", "BAD"]
    assert run(ok) == ("ok", 1) and run(err) == ("err", "BAD")
    assert run(["is_err", err]) is True and run(["is_err", ok]) is False
    assert run(["eq", ok, ok]) is True and run(["eq", ok, err]) is False
    assert run(["if", ["lit", True], err, ok]) == ("err", "BAD")
    with pytest.raises(OracleError):
        run(["is_err", lit("U8", 1)])
    with pytest.raises(OracleError):
        run(["eq", ok, lit("U8", 1)])


# -- resource bounds ----------------------------------------------------------------------


def doubling_chain(levels: int) -> dict:
    """m1(x) = concat(x, x); m2(x) = m1(m1(x)); ... multiplies a length by 2**(2**n)."""
    models = {
        "m1": ModelDef(
            "m1", [("x", T("Bytes"))], T("Bytes"), ["concat", ["var", "x"], ["var", "x"]]
        )
    }
    for i in range(2, levels + 1):
        inner = ["model_call", f"m{i - 1}", [["var", "x"]]]
        models[f"m{i}"] = ModelDef(
            f"m{i}", [("x", T("Bytes"))], T("Bytes"), ["model_call", f"m{i - 1}", [inner]]
        )
    return models


def test_value_size_bound_is_checked_before_allocation():
    prog = Program(doubling_chain(9), {})
    ir = ["model_call", "m9", [["lit", {"type": "Bytes", "value": "00"}]]]
    with pytest.raises(OracleError) as err:
        prog.compile(ir, {}).fn({})
    assert err.value.code == "ir-overflow"  # ~1,500 nodes would build 2**256 bytes


def test_concat_at_exactly_the_value_bound_is_allowed():
    half = ["lit", {"type": "Bytes", "value": "00" * (MAX_IR_VALUE // 2)}]
    assert len(run(["concat", half, half])) == MAX_IR_VALUE
    with pytest.raises(OracleError):
        run(["concat", ["concat", half, half], ["lit", {"type": "Bytes", "value": "00"}]])


def test_cost_bound_counts_model_bodies_at_full_cost():
    # f_k calls f_{k-1} twice: cost doubles per level, so a chain is refused statically.
    models = {"f0": ModelDef("f0", [("x", T("Bool"))], T("Bool"), ["var", "x"])}
    for k in range(1, 12):
        call = ["model_call", f"f{k - 1}", [["var", "x"]]]
        models[f"f{k}"] = ModelDef(f"f{k}", [("x", T("Bool"))], T("Bool"), ["and", call, call])
    prog = Program(models, {})
    assert prog.model("f5").cost < MAX_IR_COST
    with pytest.raises(OracleError) as err:
        prog.model("f11")
    assert err.value.code == "limit"
    with pytest.raises(OracleError):
        Program(models, {}).compile(["model_call", "f11", [["lit", True]]], {})


def test_cost_is_the_worst_path_one_branch_of_an_if():
    body = ["if", ["lit", True], ["and", ["lit", True], ["lit", True]], ["lit", True]]
    m = {"m": ModelDef("m", [], T("Bool"), body)}
    assert Program(m, {}).model("m").cost == 1 + 1 + 3  # the larger branch only


def test_a_raised_cost_bound_is_a_diagnostic_setting():
    models = {"f0": ModelDef("f0", [("x", T("Bool"))], T("Bool"), ["var", "x"])}
    for k in range(1, 12):
        c = ["model_call", f"f{k - 1}", [["var", "x"]]]
        models[f"f{k}"] = ModelDef(f"f{k}", [("x", T("Bool"))], T("Bool"), ["and", c, c])
    with pytest.raises(OracleError):
        Program(models, {}, max_cost=10**9).compile(["lit", True], {})  # above the ceiling
    assert Program(models, {}, max_cost=65_536).model("f11").cost == 12_283


def test_ir_nesting_is_capped_at_64():
    ir: list = ["lit", True]
    for _ in range(70):
        ir = ["not", ir]
    with pytest.raises(OracleError) as err:
        run(ir)
    assert err.value.code == "limit"
    ir = ["lit", True]
    for _ in range(60):
        ir = ["not", ir]
    assert run(ir) is True


# -- models, memoisation and typing ----------------------------------------------------


def test_model_calls_evaluate_and_memoise_but_hits_equal_recomputation():
    models = {
        "inc": ModelDef("inc", [("a", T("U8"))], T("U8"), ["add", ["var", "a"], lit("U8", 1)]),
        "twice": ModelDef(
            "twice",
            [("a", T("U8"))],
            T("U8"),
            ["model_call", "inc", [["model_call", "inc", [["var", "a"]]]]],
        ),
    }
    memo = Program(models, {}, memo=True)
    plain = Program(models, {}, memo=False)
    f = memo.compile(["model_call", "twice", [["var", "x"]]], {"x": T("U8")}).fn
    g = plain.compile(["model_call", "twice", [["var", "x"]]], {"x": T("U8")}).fn
    for x in range(256):  # every value, twice, so the second pass is all hits
        assert f({"x": x}) == g({"x": x}) == (x + 2) & 255
    for x in range(256):
        assert f({"x": x}) == g({"x": x})


def test_model_rule_4_2_1_forward_and_self_reference():
    models = {
        "a": ModelDef("a", [], T("Bool"), ["model_call", "b", []]),
        "b": ModelDef("b", [], T("Bool"), ["lit", True]),
    }
    with pytest.raises(OracleError):
        Program(models, {}).model("a")
    me = {"a": ModelDef("a", [], T("Bool"), ["model_call", "a", []])}
    with pytest.raises(OracleError):
        Program(me, {}).model("a")
    with pytest.raises(OracleError):
        run(["model_call", "ghost", []])
    ok = {"a": ModelDef("a", [("x", T("Bool"))], T("Bool"), ["var", "x"])}
    with pytest.raises(OracleError):
        Program(ok, {}).compile(["model_call", "a", []], {})  # wrong arity
    with pytest.raises(OracleError):
        Program(ok, {}).compile(["model_call", "a", [lit("U8", 1)]], {})  # wrong type
    with pytest.raises(OracleError):
        Program({"a": ModelDef("a", [], T("U8"), ["lit", True])}, {}).model("a")  # wrong ret


def test_subject_calls_are_typed_but_never_executed():
    op = OpSig("f", (), [("a", T("U8"))], T("U8"), False)
    prog = Program({}, {"f": op})
    ir = ["subject_op", "f", [lit("U8", 1)]]
    c = prog.compile(ir, {})
    assert c.subject and c.ty == T("U8")
    with pytest.raises(OracleError):
        c.fn({})
    with pytest.raises(OracleError):
        prog.compile(ir, {}, allow_subject=False)
    for bad in (
        ["subject_op", "g", []],
        ["subject_op", "f", []],
        ["subject_op", "f", [lit("U16", 1)]],
        ["subject_op", "f", [lit("U8", 1)], [3]],
        ["subject_op", "f"],
        ["subject_op", 1, []],
    ):
        with pytest.raises(OracleError):
            prog.compile(bad, {})
    # a model may not contain a subject call
    bad_model = {"m": ModelDef("m", [], T("U8"), ir)}
    with pytest.raises(OracleError):
        Program(bad_model, {"f": op}).model("m")


def test_static_widths_on_subject_calls():
    op = OpSig("g", ("N",), [("a", T("Bytes", (A.Ty("N").head,)))], T("Bool"), False)
    op.params[0] = ("a", T("Bytes", ("N",)))
    prog = Program({}, {"g": op})
    b = lambda h: ["lit", {"type": "Bytes", "value": h}]  # noqa: E731
    assert prog.compile(["subject_op", "g", [b("0000")], [2]], {}).ty == T("Bool")
    for bad in (
        ["subject_op", "g", [b("00")]],
        ["subject_op", "g", [b("00")], [1, 2]],
        ["subject_op", "g", [b("00")], [-1]],
        ["subject_op", "g", [b("00")], [5000]],
        ["subject_op", "g", [b("00")], ["x"]],
    ):
        with pytest.raises(OracleError):
            prog.compile(bad, {})


def test_if_typing_and_unify():
    assert run(["if", ["lit", True], lit("Nat", 1), lit("Int", 2)]) == 1
    with pytest.raises(OracleError):
        run(["if", lit("U8", 1), lit("U8", 1), lit("U8", 1)])
    with pytest.raises(OracleError):
        run(["if", ["lit", True], lit("U8", 1), ["lit", True]])


# -- call counting (section 4.4) ---------------------------------------------------------


F = OpSig("f", (), [("a", T("U8"))], T("U8"), False)
G = OpSig("g", (), [("a", T("U8"))], T("Bool"), False)


def calls(ir, tenv=None):
    prog = Program({}, {"f": F, "g": G})
    return compile_calls(prog, ir, tenv or {"a": T("U8")})


def f(x):
    return ["subject_op", "f", [x]]


def test_call_counts_for_straight_line_bodies():
    a = ["var", "a"]
    assert calls(["eq", f(a), lit("U8", 0)]) == 1
    assert calls(["eq", f(f(a)), f(a)]) == 3  # nested subject calls are unconditional
    assert calls(["lit", True]) == 0


def test_add_associativity_claim_has_four_calls():
    a, b, c = (["var", n] for n in "abc")
    env = {n: T("U8") for n in "abc"}
    lhs = f(f(a)) if False else ["subject_op", "f", [a]]
    add = lambda x, y: ["add", x, y]  # noqa: E731
    left = add(add(a, b), c)
    right = add(a, add(b, c))
    # add(add(a,b),c) == add(a,add(b,c)) with add as a subject op has 4 calls per case
    op = OpSig("add", (), [("x", T("U8")), ("y", T("U8"))], T("U8"), False)
    prog = Program({}, {"add": op})
    s = lambda x, y: ["subject_op", "add", [x, y]]  # noqa: E731
    del left, right, lhs
    assert compile_calls(prog, ["eq", s(s(a, b), c), s(a, s(b, c))], env) == 4


def test_pure_guard_makes_the_count_depend_on_the_case():
    a = ["var", "a"]
    ir = ["if", ["eq", a, lit("U8", 0)], ["lit", True], ["eq", f(a), f(a)]]
    fn = calls(ir)
    assert callable(fn) and fn({"a": 0}) == 0 and fn({"a": 1}) == 2
    both = ["if", ["eq", a, lit("U8", 0)], ["eq", f(a), lit("U8", 1)], ["eq", f(a), f(a)]]
    assert calls(both)({"a": 0}) == 1 and calls(both)({"a": 5}) == 2
    short = ["and", ["ne", a, lit("U8", 0)], ["eq", f(a), lit("U8", 1)]]
    fn = calls(short)
    assert fn({"a": 0}) == 0 and fn({"a": 3}) == 1
    short_or = ["or", ["eq", a, lit("U8", 0)], ["eq", f(a), lit("U8", 1)]]
    fn = calls(short_or)
    assert fn({"a": 0}) == 0 and fn({"a": 3}) == 1
    imp = ["implies", ["eq", a, lit("U8", 0)], ["eq", f(a), lit("U8", 1)]]
    fn = calls(imp)
    assert fn({"a": 0}) == 1 and fn({"a": 3}) == 0


def test_a_subject_call_under_an_impure_guard_is_w0701():
    a = ["var", "a"]
    bad_if = ["if", ["eq", f(a), lit("U8", 0)], ["eq", f(a), lit("U8", 1)], ["lit", True]]
    with pytest.raises(OracleError) as err:
        calls(bad_if)
    assert err.value.code == "generation" and "W0701" in err.value.message
    bad_and = ["and", ["eq", f(a), lit("U8", 0)], ["eq", f(a), lit("U8", 1)]]
    with pytest.raises(OracleError):
        calls(bad_and)


def test_impure_condition_is_fine_when_no_branch_has_a_subject_call():
    a = ["var", "a"]
    ir = ["if", ["eq", f(a), lit("U8", 0)], ["lit", True], ["lit", False]]
    assert calls(ir) == 1
    assert calls(["and", ["eq", f(a), lit("U8", 0)], ["lit", True]]) == 1


def test_dynamic_counts_combine_with_static_ones():
    a = ["var", "a"]
    guarded = ["if", ["eq", a, lit("U8", 0)], ["eq", f(a), lit("U8", 1)], ["lit", True]]
    both = ["eq", guarded, ["eq", f(a), f(a)]]
    fn = calls(both)
    assert fn({"a": 0}) == 3 and fn({"a": 1}) == 2


def test_has_subject_looks_through_models_args_and_branches():
    assert has_subject(f(["var", "a"]))
    assert has_subject(["not", ["eq", f(["var", "a"]), lit("U8", 1)]])
    assert not has_subject(["model_call", "m", [["var", "a"]]])
    assert not has_subject(["err", "X"]) and not has_subject(["lit", True])
    assert not has_subject(5)


def test_call_counting_nesting_limit():
    ir: list = f(["var", "a"])
    for _ in range(70):
        ir = ["not", ir]
    with pytest.raises(OracleError):
        calls(ir)
