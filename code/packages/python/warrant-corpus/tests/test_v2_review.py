"""The S6a review findings, one test per finding (and the erratum's readings R69-R75).

B2 static widths, B3 bytes literals, [D64] per-suite levels, F4/R71 the overflow
class, F5 the oracle block, F6 fault rows, F7 chunk-file hostility, F9, F11, F13,
F14, R70, R72, R74, R75.  Each case here is built from the spec text alone.
"""

from __future__ import annotations

import sys

import pytest
from test_v2_ir import INC, bump, case, u8, var

from coding_adventures_warrant_corpus.v2 import STANDARD, Program
from coding_adventures_warrant_corpus.v2 import chunks as C
from coding_adventures_warrant_corpus.v2.errors import KitError, Refusal
from coding_adventures_warrant_corpus.v2.faults import compile_fault
from coding_adventures_warrant_corpus.v2.ir import guarded
from coding_adventures_warrant_corpus.v2.kit import kit_hex, outcome_domain, outcome_ir
from coding_adventures_warrant_corpus.v2.wtypes import (
    Ty,
    bind_widths,
    forget_widths,
    parse_type,
)


def code(case_):
    return outcome_ir(case_, STANDARD).get("error")


def bare_case(ir, **fields):
    """A case with no stand-in: for expressions that never call the subject."""
    return case(ir, **({"subjects": {}} | fields))


def hexlit(text):
    return ["lit", {"type": "Bytes", "value": text}]


# -- B2: static widths ---------------------------------------------------------------------
EQ_FIXED = {
    "eq_fixed": {
        "params": [["a", "Bytes<N>"], ["b", "Bytes<N>"]],
        "ret": "Bool",
        "width_params": ["N"],
        "requires": ["ir.core"],
    }
}


def fixed_call(widths, *, a="abcd", b="ef01"):
    node = ["subject_op", "eq_fixed", [hexlit(a), hexlit(b)]]
    return node if widths is None else [*node, widths]


@pytest.mark.parametrize(
    ("widths", "want"),
    [
        ([3], "ill-typed"),  # the arguments are two bytes, the signature says three
        (None, "ill-typed"),  # a missing list
        ([], "ill-typed"),  # a missing entry
        ([2, 2], "ill-typed"),  # an extra entry
        ([4097], "ill-typed"),
        ([-1], "ill-typed"),
        (["2"], "ill-typed"),
        ([True], "ill-typed"),
        ("2", "ill-typed"),
    ],
)
def test_static_widths_are_bound_into_the_signature(widths, want):
    got = outcome_ir(
        case(fixed_call(widths), ops=EQ_FIXED, models={}, model_order=[], subjects={}),
        STANDARD,
    )
    assert got == {"error": want}


def test_a_well_typed_width_call_reaches_the_dispatch():
    models = {
        "same": {"params": [["a", "Bytes"], ["b", "Bytes"]], "body": ["lit", True]}
    }
    got = outcome_ir(
        case(
            fixed_call([2]),
            ops=EQ_FIXED,
            models=models,
            model_order=["same"],
            subjects={"eq_fixed": "same"},
        ),
        STANDARD,
    )
    assert got == {"type": "Bool", "value": True, "calls": 1}


def test_a_widths_list_on_an_op_without_width_params_is_ill_typed_even_when_empty():
    for widths in ([], [1]):
        node = [*bump(), widths]
        assert code(case(node)) == "ill-typed"


def test_bind_and_forget_widths():
    ty = parse_type(
        "Result<Bytes<N>,E>",
        {"E": {"kind": "error", "cases": ["X"]}},
        widths=frozenset("N"),
    )
    assert str(bind_widths(ty, {"N": 4})) == "Result<Bytes<4>,E>"
    assert str(forget_widths(ty)) == "Result<Bytes,E>"
    assert forget_widths(Ty("U8")) == Ty("U8")


# -- B3: bytes literals ----------------------------------------------------------------------
TAKES4 = {"f": {"params": [["x", "Bytes<4>"]], "ret": "Bool", "requires": ["ir.core"]}}
TAKES2 = {"f": {"params": [["x", "Bytes<2>"]], "ret": "Bool", "requires": ["ir.core"]}}
STAND = {"same": {"params": [["x", "Bytes<4>"]], "body": ["lit", True]}}


def test_a_bytes_literal_has_the_static_type_of_its_length():
    ok = ["subject_op", "f", [hexlit("00112233")]]
    models = {"same": {"params": [["x", "Bytes<4>"]], "body": ["lit", True]}}
    got = outcome_ir(
        case(
            ok, ops=TAKES4, models=models, model_order=["same"], subjects={"f": "same"}
        ),
        STANDARD,
    )
    assert got["value"] is True
    assert code(case(["subject_op", "f", [hexlit("0011")]], ops=TAKES4)) == "ill-typed"
    assert (
        code(case(["subject_op", "f", [hexlit("00112233")]], ops=TAKES2)) == "ill-typed"
    )


def test_the_node_keeps_the_written_type_for_reporting():
    program = Program()
    compiled = program.compile(hexlit("0a0b"), {})
    assert str(compiled.ty) == "Bytes<2>" and str(compiled.written) == "Bytes"


@pytest.mark.parametrize(
    "literal",
    [
        {"type": "Bytes<2>", "value": "0a0b"},
        {"type": "Bool", "value": True},
    ],
)
def test_a_literal_written_with_the_wrong_type_is_ill_typed(literal):
    assert code(case(["lit", literal])) == "ill-typed"


# -- [D64]: levels by reach ------------------------------------------------------------------
FIXED_MODEL = {"params": [["a", "U8"]], "body": ["bit_and", var("a"), u8(1)]}


def test_a_model_nothing_reaches_may_need_a_missing_level():
    got = outcome_ir(
        bare_case(["lit", True], models={"fx": FIXED_MODEL}, model_order=["fx"]),
        STANDARD,
    )
    assert got["value"] is True


def test_a_model_the_expression_reaches_is_level_missing():
    ir = ["model_call", "fx", [var("a")]]
    assert (
        code(case(ir, models={"fx": FIXED_MODEL}, model_order=["fx"]))
        == "level-missing"
    )


def test_a_model_reached_through_another_model_is_level_missing():
    models = {
        "fx": FIXED_MODEL,
        "outer": {"params": [["a", "U8"]], "body": ["model_call", "fx", [var("a")]]},
    }
    ir = ["model_call", "outer", [var("a")]]
    assert code(case(ir, models=models, model_order=["fx", "outer"])) == "level-missing"
    # and the failed compile is not remembered as a success
    program = Program(
        models=models, model_order=["fx", "outer"], levels=frozenset({"ir.core"})
    )
    for _ in range(2):
        with pytest.raises(Refusal) as caught:
            program.compile(ir, {"a": Ty("U8")})
        assert caught.value.code == "level-missing"


def test_an_unknown_tag_in_a_model_nothing_reaches_is_still_a_corpus_error():
    models = {"bad": {"params": [], "body": ["frobnicate"]}}
    assert (
        code(bare_case(["lit", True], models=models, model_order=["bad"]))
        == "unknown-tag"
    )
    # while a tag of a defined level is only a level question
    ok = {"fx": {"params": [], "body": ["bit_and", u8(1), u8(1)]}}
    assert code(bare_case(["lit", True], models=ok, model_order=["fx"])) is None


def test_the_unknown_tag_wins_over_a_level_in_one_body():
    body = ["and", ["eq", ["bit_and", u8(1), u8(1)], u8(1)], ["frobnicate"]]
    models = {"m": {"params": [], "body": body}}
    assert code(case(["lit", True], models=models, model_order=["m"])) == "unknown-tag"


def test_a_chain_of_65_models_is_depth_even_when_nothing_calls_it():
    models = {"m0": {"params": [], "body": ["lit", True]}}
    for i in range(1, 65):
        models[f"m{i}"] = {"params": [], "body": ["model_call", f"m{i - 1}", []]}
    order = [f"m{i}" for i in range(65)]
    assert code(bare_case(["lit", True], models=models, model_order=order)) == "depth"
    del models["m64"]
    assert code(bare_case(["lit", True], models=models, model_order=order[:64])) is None


def test_a_model_body_over_the_cost_bound_is_refused_even_when_not_live():
    body = ["lit", True]
    for _ in range(11):  # 2^12 nodes: a balanced tree of `and`
        body = ["and", body, body]
    heavy = {"m": {"params": [], "body": body}}
    assert code(bare_case(["lit", True], models=heavy, model_order=["m"])) == "ir-cost"
    assert (
        code(
            bare_case(["lit", True], models=heavy, model_order=["m"])
            | {"max_ir_cost": 65536}
        )
        is None
    )


def test_model_order_is_a_permutation_of_the_models():
    models = {"a": INC["inc"], "b": INC["inc"]}
    for order in (["a"], ["a", "a", "b"], ["a", "b", "ghost"], ["b", "a", "a"]):
        with pytest.raises(Refusal) as caught:
            Program(models=models, model_order=order)
        assert caught.value.code == "model-order"
    with pytest.raises(Refusal):
        Program(models=models, model_order=[1, 2])
    Program(models=models, model_order=["b", "a"])


def test_a_model_calling_a_later_model_is_refused_wherever_it_stands():
    models = {
        "a": {"params": [], "body": ["model_call", "b", []]},
        "b": {"params": [], "body": ["lit", True]},
    }
    assert (
        code(case(["lit", True], models=models, model_order=["a", "b"]))
        == "model-order"
    )


# -- F4 / R71: the class of an overflow -------------------------------------------------------
BIG = ["mul", ["lit", {"type": "Nat", "value": "18446744073709551615"}]] * 1
HUGE = [
    "mul",
    [
        "mul",
        ["lit", {"type": "Nat", "value": "18446744073709551615"}],
        ["lit", {"type": "Nat", "value": "18446744073709551615"}],
    ],
    [
        "mul",
        ["lit", {"type": "Nat", "value": "18446744073709551615"}],
        ["lit", {"type": "Nat", "value": "18446744073709551615"}],
    ],
]
HUGE_EQ = ["eq", HUGE, ["lit", {"type": "Nat", "value": "0"}]]
SUBJECT_IS_5 = ["eq", bump("a"), u8(6)]


def overflow_class(ir):
    got = outcome_ir(case(ir), STANDARD)
    assert got["error"] == "ir-overflow"
    return got["class"]


def test_a_pure_overflow_outside_any_guard_is_a_corpus_error():
    assert overflow_class(HUGE_EQ) == "corpus"
    assert overflow_class(["and", ["lit", True], HUGE_EQ]) == "corpus"  # a pure guard
    assert overflow_class(["if", var("t"), HUGE_EQ, ["lit", True]]) == "corpus"


@pytest.mark.parametrize(
    "ir",
    [
        ["if", SUBJECT_IS_5, HUGE_EQ, ["lit", True]],
        ["and", SUBJECT_IS_5, HUGE_EQ],
        ["or", ["not", SUBJECT_IS_5], HUGE_EQ],
        ["implies", SUBJECT_IS_5, HUGE_EQ],
        ["and", SUBJECT_IS_5, ["if", var("t"), HUGE_EQ, ["lit", True]]],
    ],
)
def test_a_pure_node_under_a_guard_that_calls_the_subject_is_a_case_failure(ir):
    # The stand-in makes bump(5) = 6, so SUBJECT_IS_5 holds and the guarded node runs.
    assert overflow_class(ir) == "case"


def test_the_guarded_node_that_does_not_run_does_not_overflow():
    ir = ["if", ["not", SUBJECT_IS_5], HUGE_EQ, ["lit", True]]
    got = outcome_ir(case(ir), STANDARD)
    assert got == {"type": "Bool", "value": True, "calls": 1}


def test_an_overflow_in_a_model_called_under_an_impure_guard_is_a_case_failure():
    models = {
        "inc": INC["inc"],
        "boom": {"params": [], "body": HUGE_EQ},
    }
    ir = ["if", ["not", SUBJECT_IS_5], ["model_call", "boom", []], ["lit", True]]
    got = outcome_ir(case(ir, models=models, model_order=["inc", "boom"]), STANDARD)
    assert got == {"type": "Bool", "value": True, "calls": 1}
    ir = ["if", SUBJECT_IS_5, ["model_call", "boom", []], ["lit", True]]
    got = outcome_ir(case(ir, models=models, model_order=["inc", "boom"]), STANDARD)
    assert got == {"error": "ir-overflow", "class": "case"}
    # the same call under a pure guard is the corpus's fault
    ir = ["if", var("t"), ["model_call", "boom", []], ["lit", True]]
    got = outcome_ir(case(ir, models=models, model_order=["inc", "boom"]), STANDARD)
    assert got == {"error": "ir-overflow", "class": "corpus"}


# -- F5 / F6: the oracle block and fault rows --------------------------------------------------
def build_ok(*, oracle=None, result_type="Bool", **edits):
    from test_v2_chunks import bare

    binders = [("b", "U8", {"d": "u8_all", "size": 256})]
    corpus, row = bare(
        binders, expected=["model_call", "m", [var("b")]], result_type="U8"
    )
    row = row | edits
    if oracle is not None:
        row["oracle"] = oracle
    if result_type != "Bool":
        row["result_type"] = result_type
    return corpus, row


def test_the_oracle_block_is_closed_and_checked():
    corpus, row = build_ok(result_type="U8")
    C.build_suite(corpus, row)
    good = row["oracle"]
    for oracle in (
        good | {"extra": 1},
        {"kind": "reference", "expected": good["expected"]},
        {"kind": "predicate", "subject": good["subject"]},
        {
            "kind": "reference",
            "subject": ["subject_op", "nope", []],
            "expected": good["expected"],
        },
        {"kind": "reference", "subject": ["bad"], "expected": good["expected"]},
        "reference",
    ):
        with pytest.raises(Refusal) as caught:
            C.build_suite(corpus, row | {"oracle": oracle})
        assert caught.value.code == "corpus", oracle


def test_the_expected_side_must_produce_the_result_type():
    corpus, row = build_ok(result_type="U8")
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus, row | {"result_type": "Bool"})
    assert caught.value.code == "corpus"


def fault_row(**edits):
    row = {
        "fault": "f",
        "op": "op",
        "kind": "pre",
        "rewrites": [{"param": "a", "expr": ["var", "b"]}],
        "requires": ["ir.core"],
    }
    return row | edits


OP2 = {"params": [["a", "U8"], ["b", "U8"]], "ret": "U8", "requires": ["ir.core"]}


@pytest.mark.parametrize(
    "row",
    [
        fault_row(rewrites=[{"param": "zzz", "expr": ["var", "b"]}]),
        fault_row(
            rewrites=[
                {"param": "a", "expr": ["var", "b"]},
                {"param": "a", "expr": ["var", "b"]},
            ]
        ),
        fault_row(rewrites=[]),
        fault_row(rewrites=[{"param": "a", "expr": ["var", "b"], "x": 1}]),
        fault_row(rewrites=["a"]),
        fault_row(extra=1),
        fault_row(op=3),
        fault_row(requires="ir.core"),
        {
            "fault": "f",
            "kind": "post",
            "expr": ["var", "result"],
            "requires": ["ir.core"],
        },
        {
            "fault": "f",
            "op": "op",
            "kind": "post",
            "expr": ["var", "result"],
            "requires": ["ir.core"],
            "z": 0,
        },
        {"fault": "f", "op": "op", "kind": "wobble", "requires": []},
        "nope",
    ],
)
def test_a_malformed_fault_row_is_a_kit_error_never_a_crash(row):
    # R126: the row's own schema is the case's, so a break is a kit error.
    with pytest.raises(KitError):
        compile_fault(row, OP2, {})


@pytest.mark.parametrize(
    "row",
    [
        fault_row(rewrites=[{"param": "a", "expr": ["lit", True]}]),
        {
            "fault": "f",
            "op": "op",
            "kind": "post",
            "expr": ["lit", True],
            "requires": ["ir.core"],
        },
    ],
)
def test_a_well_formed_row_of_the_wrong_type_is_ill_typed(row):
    with pytest.raises(Refusal) as caught:
        compile_fault(row, OP2, {})
    assert caught.value.code == "ill-typed"


def test_a_fault_rewrites_a_named_width_parameter_as_plain_bytes():
    op = {
        "params": [["p", "Bytes<N>"]],
        "ret": "Bool",
        "width_params": ["N"],
        "requires": ["ir.core"],
    }
    row = fault_row(
        rewrites=[
            {
                "param": "p",
                "expr": [
                    "concat",
                    ["var", "p"],
                    ["lit", {"type": "Bytes", "value": "00"}],
                ],
            }
        ]
    )
    compiled = compile_fault(row, op, {})
    assert compiled.rewrite_args([b"\x01"]) == [b"\x01\x00"]


def test_a_nat_fault_may_produce_an_int_that_is_checked_when_written():
    op = {"params": [["a", "Nat"]], "ret": "Nat", "requires": ["ir.core"]}
    row = {
        "fault": "f",
        "op": "op",
        "kind": "post",
        "expr": ["add", ["var", "result"], ["lit", {"type": "Nat", "value": "1"}]],
        "requires": ["ir.core"],
    }
    assert compile_fault(row, op, {}).rewrite_result([1], 2) == 3


# -- F7: the chunk file is hostile --------------------------------------------------------------
def sealed_suite():
    from test_v2_chunks import A_AND_B, REFINED, sealed

    corpus, row, data = sealed(A_AND_B, **REFINED)
    return C.build_suite(corpus, row), data


def refuses(suite, data):
    import hashlib

    suite.row["chunks_sha256"] = hashlib.sha256(data).hexdigest()
    with pytest.raises(Refusal) as caught:
        C.check_chunk_file(data, suite)
    assert caught.value.code == "chunk-file"


def test_a_chunk_file_over_2_mib_is_refused_before_it_is_read():
    suite, _data = sealed_suite()
    with pytest.raises(Refusal) as caught:
        C.check_chunk_file(b"0" * ((2 << 20) + 1), suite)
    assert caught.value.code == "chunk-file"


@pytest.mark.parametrize(
    "line",
    [
        "0 " + "9" * 21 + " 0 0 " + "0" * 64 + " " + "0" * 64,  # a 21-digit count
        "0 ٣ 0 0 " + "0" * 64 + " " + "0" * 64,  # a non-ASCII digit
        "0 -1 0 0 " + "0" * 64 + " " + "0" * 64,
        "0 01 0 0 " + "0" * 64 + " " + "0" * 64,
        "0 1 0 0 " + "0" * 63 + " " + "0" * 64,
        "0 1 0 0 " + "G" * 64 + " " + "0" * 64,
        "0  1 0 0 " + "0" * 64 + " " + "0" * 64,  # a doubled space
        "0 1 0 0 " + "0" * 64,  # the expected column is missing
        "0 1 0 0 " + "0" * 64 + " " + "0" * 64 + " 5",
        "",
        "9" * 1_000_000,
    ],
)
def test_a_hostile_line_is_chunk_file_and_never_a_value_error(line):
    suite, _data = sealed_suite()
    refuses(suite, (line + "\n").encode() * suite.m)


def test_non_ascii_bytes_and_a_missing_final_lf_are_chunk_file():
    suite, data = sealed_suite()
    refuses(suite, data.replace(b"\n", b"\xc3\xa9\n", 1))
    refuses(suite, data[:-1])
    refuses(suite, data + b"\n")


# -- F14: the recursion limit is not raised around the loop --------------------------------------
def test_a_shallow_expression_runs_under_the_ordinary_limit():
    calls = []

    def fn(env):
        calls.append(sys.getrecursionlimit())
        return 1

    before = sys.getrecursionlimit()
    assert guarded(fn, 3) is fn
    deep = guarded(fn, 10_000)
    assert deep is not fn and deep({}) == 1
    assert calls[-1] >= 60_000 and sys.getrecursionlimit() == before


def test_enumerating_a_chunk_leaves_the_limit_alone():
    suite, _data = sealed_suite()
    before = sys.getrecursionlimit()
    C.run_chunk(suite, 0)
    assert sys.getrecursionlimit() == before


# -- R72, R74, R75 and the rest -----------------------------------------------------------------
def test_a_take_count_of_a_fixed_width_type_is_ill_typed():
    for tag in ("take", "drop"):
        assert code(case([tag, hexlit("0a"), u8(1)])) == "ill-typed"


def test_suites_have_one_to_eight_binders_and_a_bounded_product():
    from test_v2_chunks import bare

    one = ("a", "U8", {"d": "u8_all", "size": 256})
    corpus, row = bare([one], body=["lit", True], ops={}, models={}, order=[])
    for binders in ([one] * 9, [one] * 8 + [one]):
        with pytest.raises(Refusal) as caught:
            C.build_suite(
                corpus,
                row
                | {"binders": [[n, {"type": t, "domain": d}] for n, t, d in binders]},
            )
        assert caught.value.code == "corpus"


def test_the_corpus_part_and_row_are_checked_for_kind_and_booleans():
    from test_v2_chunks import bare

    one = ("a", "U8", {"d": "u8_all", "size": 256})
    corpus, row = bare([one], body=["lit", True], ops={}, models={}, order=[])
    for edit in (
        {"kind": "other"},
        {"product": True},
        {"chunks": True},
        {"binders": "x"},
    ):
        with pytest.raises(Refusal):
            C.build_suite(corpus, row | edit)
    two = [("a", "U8", {"d": "u8_all", "size": 256})] * 2
    corpus2, row2 = bare(two, body=["lit", True], ops={}, models={}, order=[])
    with pytest.raises(Refusal):
        C.build_suite(corpus2, row2)  # two binders with one name


def test_kit_hex_and_the_domain_window_schema():
    assert kit_hex("0a", "x") == b"\n"
    with pytest.raises(KitError):
        kit_hex("zz", "x")
    with pytest.raises(KitError):
        kit_hex(5, "x")
    base = {"descriptor": {"d": "bool", "size": 2}, "type": "Bool"}
    assert outcome_domain(base | {"index": 1}, STANDARD)["size"] == 2
    assert outcome_domain(base | {"range": [2, 1]}, STANDARD) == {"error": "index"}
    with pytest.raises(KitError):
        outcome_domain(base | {"range": [1]}, STANDARD)
    with pytest.raises(KitError):
        outcome_domain(base | {"index": -1}, STANDARD)


def test_an_overflow_in_a_model_entered_through_a_subject_argument_is_a_case_failure():
    """R71 (extended): the generator walks such a call's arguments, never its body."""
    models = {
        "inc": INC["inc"],
        "boom": {"params": [["x", "U8"]], "body": HUGE_EQ},
        "outer": {"params": [["x", "U8"]], "body": ["model_call", "boom", [var("x")]]},
    }
    order = ["inc", "boom", "outer"]
    ir = ["model_call", "outer", [bump()]]
    got = outcome_ir(case(ir, models=models, model_order=order), STANDARD)
    assert got == {"error": "ir-overflow", "class": "case"}
    pure = ["model_call", "outer", [var("a")]]
    got = outcome_ir(case(pure, models=models, model_order=order), STANDARD)
    assert got == {"error": "ir-overflow", "class": "corpus"}


def test_a_model_that_needs_a_missing_level_is_classified_not_compiled():
    core = frozenset({"ir.core"})
    models = {"fx": FIXED_MODEL, "ok": INC["inc"]}
    program = Program(models=models, model_order=["fx", "ok"], levels=core)
    assert list(program.not_live_models) == ["fx"] and "ok" in program.models
    # the same models are all live at the levels this lane implements (S10e)
    assert Program(models=models, model_order=["fx", "ok"]).not_live_models == {}
    # a model calling a not-live model is itself not live
    models["outer"] = {
        "params": [["a", "U8"]],
        "body": ["model_call", "fx", [var("a")]],
    }
    program = Program(models=models, model_order=["fx", "ok", "outer"], levels=core)
    assert sorted(program.not_live_models) == ["fx", "outer"]


def test_a_fault_may_change_a_length_but_not_the_kind_of_value():
    op = {"params": [["a", "U8"]], "ret": "Bytes<32>", "requires": ["ir.core"]}
    drop = [
        "take",
        ["var", "result"],
        ["sub", ["len", ["var", "result"]], ["lit", {"type": "Nat", "value": "1"}]],
    ]
    row = {
        "fault": "d",
        "op": "op",
        "kind": "post",
        "expr": drop,
        "requires": ["ir.core"],
    }
    compile_fault(row, op, {})
    with pytest.raises(Refusal) as caught:
        compile_fault(row | {"expr": ["lit", True]}, op, {})
    assert caught.value.code == "ill-typed"


def test_a_literal_written_with_a_tuple_type_is_a_level_before_it_is_a_bad_literal():
    got = outcome_ir(case(["lit", {"type": "(U8,U8)", "value": ["1", "2"]}]), STANDARD)
    assert got == {"error": "level-missing"}


def test_a_stand_in_for_an_op_with_widths_is_compared_with_plain_bytes():
    from coding_adventures_warrant_corpus.v2.ir import SubjectTable

    ops = {
        "f": {
            "params": [["a", "Bytes<N>"]],
            "ret": "Bool",
            "width_params": ["N"],
            "requires": ["ir.core"],
        }
    }
    good = {"s": {"params": [["a", "Bytes"]], "body": ["lit", True]}}
    bad = {"s": {"params": [["a", "U8"]], "body": ["lit", True]}}
    for models, fine in ((good, True), (bad, False)):
        program = Program(ops=ops, models=models, model_order=["s"])
        # R100: only a called op's stand-in is compared, so the tree calls it.
        program.compile(
            ["subject_op", "f", [["var", "a"]], [3]], {"a": parse_type("Bytes<3>", {})}
        )
        if fine:
            SubjectTable(program, {"f": "s"})
        else:
            with pytest.raises(KitError):
                SubjectTable(program, {"f": "s"})
