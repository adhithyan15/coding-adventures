"""VER04r section 16.22: [D74], [D75] and R110 to R117 in the Python lane.

* [D74], R110: a ``v2-fault`` case's bad ``types`` table is ``ill-typed``, read
  whole after the row's own shape and before ``level-missing``,
  ``subject-forbidden`` and the typing of the row.
* [D75], R111: a *model parameter* may be ``Int`` (also under ``Option``,
  ``Result``, tuples and ``List``); an op row, a binder, a field and a value may
  not.  R112: a model's result may be ``Int``: nothing here changes it.
* R115: the order of the checks inside ``v2-ir``.
* R116: a ``list`` descriptor does not hide a model's unknown tag.
* R117: an error type is not the type of a field, a constructor component or any
  other value.
"""

from __future__ import annotations

import copy

import pytest
from test_v2_chunks import MODELS, OPS, bare
from test_v2_ir import BUMP, case, u8, var
from test_v2_lane_checks import ERR, fault_case, fault_row

from coding_adventures_warrant_corpus.v2 import STANDARD, Program
from coding_adventures_warrant_corpus.v2 import chunks as C
from coding_adventures_warrant_corpus.v2 import ir as ir_module
from coding_adventures_warrant_corpus.v2.errors import Refusal
from coding_adventures_warrant_corpus.v2.faults import compile_fault
from coding_adventures_warrant_corpus.v2.kit import (
    outcome_fault,
    outcome_ir,
    outcome_value,
)
from coding_adventures_warrant_corpus.v2.wtypes import (
    check_types_table,
    parse_type,
)

LEAF = {"d": "u8_all", "size": 256}
INT3 = ["lit", {"type": "Int", "value": "3"}]
SUBJECT_CALL = ["subject_op", "bump", [["var", "result"]]]


ERR_TABLE = {"E": ERR}


def u8_nat(n):
    return ["lit", {"type": "Nat", "value": str(n)}]


def nest(depth):
    node = ["lit", True]
    for _ in range(depth):
        node = ["not", node]
    return node


def code_of(thunk):
    with pytest.raises(Refusal) as caught:
        thunk()
    return caught.value.code


# ===========================================================================
# [D74] / R110: `ill-typed` in v2-fault
# ===========================================================================
# R110 (d): one case per table defect, each on a row that is otherwise accepted.
FAULT_TABLE_DEFECTS = {
    "a record with a field of its own type": {
        "R": {"kind": "record", "fields": [["me", "R"]]}
    },
    "a record holding itself under List": {
        "R": {"kind": "record", "fields": [["me", "List<R>"]]}
    },
    "two records that hold each other under Option": {
        "A": {"kind": "record", "fields": [["b", "Option<B>"]]},
        "B": {"kind": "record", "fields": [["a", "Option<A>"]]},
    },
    "a repeated field name": {
        "P": {"kind": "record", "fields": [["a", "U8"], ["a", "Bool"]]}
    },
    "a repeated constructor name": {
        "V": {"kind": "variant", "ctors": [["A", []], ["A", ["U8"]]]}
    },
    "a field of an undeclared type": {
        "P": {"kind": "record", "fields": [["a", "Ghost"]]}
    },
    "a field of an error type": {
        "E": ERR,
        "P": {"kind": "record", "fields": [["a", "E"]]},
    },
}


def test_the_fault_row_the_defects_ride_on_is_accepted():
    assert outcome_fault(fault_case(), STANDARD) == {"result": "2"}


@pytest.mark.parametrize(
    "table", FAULT_TABLE_DEFECTS.values(), ids=list(FAULT_TABLE_DEFECTS)
)
def test_a_fault_case_with_a_bad_table_is_ill_typed_not_corpus(table):
    got = outcome_fault(fault_case(types=table), STANDARD)
    assert got == {"error": "ill-typed"}
    assert code_of(lambda: check_types_table(table, "ill-typed")) == "ill-typed"


@pytest.mark.parametrize("kind", ["pre", "post", "replace"])
def test_every_kind_of_fault_reads_the_table(kind):
    table = FAULT_TABLE_DEFECTS["a repeated field name"]
    row = fault_row()
    if kind == "pre":
        row = {
            "fault": "f",
            "op": "bump",
            "kind": "pre",
            "rewrites": [{"param": "a", "expr": ["var", "a"]}],
            "requires": ["ir.core"],
        }
    else:
        row = row | {"kind": kind}
    assert outcome_fault(fault_case(fault=row, types=table), STANDARD) == {
        "error": "ill-typed"
    }


def test_compile_fault_refuses_a_bad_table_as_ill_typed():
    op = {"params": [["a", "U8"]], "ret": "U8", "requires": ["ir.core"]}
    table = FAULT_TABLE_DEFECTS["a field of an undeclared type"]
    assert code_of(lambda: compile_fault(fault_row(), op, table)) == "ill-typed"


BAD = FAULT_TABLE_DEFECTS["a repeated field name"]


@pytest.mark.parametrize(
    ("what", "edits"),
    [
        ("a row that needs a level the lane lacks", {"requires": ["ir.fixed"]}),
        ("an expression of a missing level", {"expr": ["some", ["var", "result"]]}),
        ("a subject call", {"expr": SUBJECT_CALL}),
        ("an ill-typed tree", {"expr": ["add", ["var", "result"], ["lit", True]]}),
        ("a row with no common type", {"expr": ["lit", True]}),
    ],
)
def test_the_table_comes_before_levels_subject_and_typing(what, edits):
    got = outcome_fault(fault_case(fault=fault_row(**edits), types=BAD), STANDARD)
    assert got == {"error": "ill-typed"}, what


@pytest.mark.parametrize(
    ("what", "edits", "want"),
    [
        # (2) level-missing, then (3) subject-forbidden, then (4) the typing
        ("requires", {"requires": ["ir.fixed"]}, "level-missing"),
        (
            "requires and a subject call",
            {"requires": ["ir.fixed"], "expr": SUBJECT_CALL},
            "level-missing",
        ),
        (
            "a tag of a missing level and a subject call",
            {"expr": ["and", ["some", ["lit", True]], SUBJECT_CALL]},
            "level-missing",
        ),
        ("a subject call", {"expr": SUBJECT_CALL}, "subject-forbidden"),
        (
            "an ill-typed tree",
            {"expr": ["add", ["var", "result"], ["lit", True]]},
            "ill-typed",
        ),
        ("a row with no common type", {"expr": ["lit", True]}, "ill-typed"),
        ("an unbound variable", {"expr": ["var", "nope"]}, "ill-typed"),
    ],
)
def test_the_rest_of_the_order_of_r110_b_is_unchanged(what, edits, want):
    got = outcome_fault(fault_case(fault=fault_row(**edits)), STANDARD)
    assert got == {"error": want}, what


def test_a_fault_expression_ill_typed_as_a_tree_is_ill_typed_as_in_v2_ir():
    # R110 (a)'s reading: the refused `r67` row is of this kind.
    bad_tree = ["add", ["var", "result"], ["lit", {"type": "U16", "value": "1"}]]
    tree = outcome_fault(fault_case(fault=fault_row(expr=bad_tree)), STANDARD)
    assert tree == {"error": "ill-typed"}
    # the same tree, as an expression of v2-ir
    ir = ["add", u8(1), ["lit", {"type": "U16", "value": "1"}]]
    assert outcome_ir(case(ir, env=[]), STANDARD) == {"error": "ill-typed"}


# ===========================================================================
# [D75] / R111: Int as a model parameter
# ===========================================================================
def model_case(param_type, arg, *, body=None, **fields):
    models = {
        "m": {
            "params": [["x", param_type]],
            "body": ["lit", True] if body is None else body,
        }
    }
    ir = ["model_call", "m", [arg]]
    return case(ir, env=[], models=models, model_order=["m"], subjects={}, **fields)


def test_a_model_may_take_an_int_parameter_and_compare_it_with_an_int_literal():
    # R111 (d), the suggested vector: a Bool result by choice.
    got = outcome_ir(model_case("Int", INT3, body=["eq", var("x"), INT3]), STANDARD)
    assert got == {"type": "Bool", "value": True, "calls": 0}
    other = ["lit", {"type": "Int", "value": "4"}]
    got = outcome_ir(model_case("Int", INT3, body=["eq", var("x"), other]), STANDARD)
    assert got == {"type": "Bool", "value": False, "calls": 0}


def test_a_nat_flows_into_an_int_parameter():
    nat = ["lit", {"type": "Nat", "value": "3"}]
    got = outcome_ir(model_case("Int", nat, body=["eq", var("x"), INT3]), STANDARD)
    assert got == {"type": "Bool", "value": True, "calls": 0}


def test_a_negative_int_reaches_the_model():
    minus = ["sub", ["lit", {"type": "Nat", "value": "0"}], u8_nat(2)]
    body = ["eq", var("x"), ["lit", {"type": "Int", "value": "-2"}]]
    got = outcome_ir(model_case("Int", minus, body=body), STANDARD)
    assert got["value"] is True


def test_an_int_result_stays_accepted_r112():
    models = {"m": {"params": [], "body": ["sub", u8_nat(1), u8_nat(2)]}}
    ir = ["eq", ["model_call", "m", []], ["lit", {"type": "Int", "value": "-1"}]]
    got = outcome_ir(
        case(ir, env=[], models=models, model_order=["m"], subjects={}), STANDARD
    )
    assert got == {"type": "Bool", "value": True, "calls": 0}


def test_int_under_result_is_accepted_at_ir_core():
    models = {"m": {"params": [["x", "Result<Int,E>"]], "body": ["lit", True]}}
    Program(types=ERR_TABLE, models=models, model_order=["m"])
    got = outcome_ir(
        model_case("Result<Int,E>", ["ok", INT3], types=ERR_TABLE), STANDARD
    )
    assert got == {"type": "Bool", "value": True, "calls": 0}


@pytest.mark.parametrize(
    "text", ["Option<Int>", "(Int,Bool)", "List<Int>", "Option<(Int,List<Int>)>"]
)
def test_int_under_option_tuple_or_list_is_not_a_type_error_in_a_model(text):
    # At `ir.core` such a type is a missing level, not `ill-typed` as it was: the
    # lane records the model not-live and a call to it is `level-missing`.
    models = {"m": {"params": [["x", text]], "body": ["lit", True]}}
    program = Program(models=models, model_order=["m"], levels=frozenset({"ir.core"}))
    assert "m" in program.not_live_models
    got = outcome_ir(
        case(["lit", True], env=[], models=models, model_order=["m"], subjects={}),
        STANDARD,
    )
    assert got == {"type": "Bool", "value": True, "calls": 0}
    assert code_of(lambda: program.model("m")) == "level-missing"


@pytest.mark.parametrize(
    "text", ["Option<Int>", "(Int,Bool)", "List<Int>", "Option<(Int,List<Int>)>"]
)
def test_int_under_option_tuple_or_list_is_accepted_where_the_level_is(
    text, monkeypatch
):
    monkeypatch.setattr(ir_module, "IMPLEMENTED_LEVELS", ir_module.DEFINED_LEVELS)
    models = {"m": {"params": [["x", text]], "body": ["lit", True]}}
    program = Program(models=models, model_order=["m"], levels=ir_module.DEFINED_LEVELS)
    assert program.not_live_models == {}
    assert str(program.model("m").params[0][1]) == text


@pytest.mark.parametrize("text", ["Option<Ghost>", "Option<Intt>", "Int<U8>"])
def test_a_bad_model_parameter_type_is_still_ill_typed(text):
    models = {"m": {"params": [["x", text]], "body": ["lit", True]}}
    got = outcome_ir(
        case(["lit", True], env=[], models=models, model_order=["m"], subjects={}),
        STANDARD,
    )
    assert got == {"error": "ill-typed"}


def test_a_model_parameter_of_type_int_is_accepted_in_a_corpus_part():
    corpus, row = bare([("b", "U8", LEAF)])
    models = corpus["models"] | {
        "wide": {"params": [["s", "Int"], ["n", "Nat"]], "body": ["lit", True]}
    }
    corpus = corpus | {"models": models, "model_order": ["m", "wide"]}
    assert C.build_suite(corpus, row).m == 1


@pytest.mark.parametrize(
    "ops",
    [
        {"f": {"params": [["a", "Int"]], "ret": "U8", "requires": ["ir.core"]}},
        {"f": {"params": [["a", "U8"]], "ret": "Int", "requires": ["ir.core"]}},
    ],
    ids=["op parameter", "op result"],
)
def test_int_is_refused_in_an_op_row(ops):
    got = outcome_ir(case(["lit", True], ops=ops), STANDARD)
    assert got == {"error": "ill-typed"}
    corpus, row = bare([("b", "U8", LEAF)])
    assert code_of(lambda: C.build_suite(corpus | {"ops": ops}, row)) == "corpus"


def test_int_is_still_refused_as_a_binder_a_field_and_a_value():
    # a binder
    corpus, row = bare([("b", "U8", LEAF)])
    row = copy.deepcopy(row)
    row["binders"][0][1]["type"] = "Int"
    assert code_of(lambda: C.build_suite(corpus, row)) == "corpus"
    # a field, as a type and so as a value
    table = {"P": {"kind": "record", "fields": [["a", "Int"]]}}
    reject = outcome_value(
        {"type": "P", "types": table, "input_hex": b'{"a":"1"}'.hex()}, STANDARD
    )
    assert reject == {"outcome": "reject", "error": "type"}
    # a value
    reject = outcome_value(
        {"type": "Int", "types": {}, "input_hex": b'"1"'.hex()}, STANDARD
    )
    assert reject == {"outcome": "reject", "error": "type"}
    assert code_of(lambda: parse_type("Option<Int>", {})) == "type"


def test_an_env_entry_of_type_int_is_ill_typed():
    # R111 (c) as amended by section 16.23 R120: a vector now (see
    # test_v2_erratum_16_23.py), no longer a kit error.
    got = case(["lit", True], env=[["x", {"type": "Int", "value": "1"}]])
    assert outcome_ir(got, STANDARD) == {"error": "ill-typed"}


# ===========================================================================
# R115: the order of the checks inside v2-ir
# ===========================================================================
BAD_TABLE = {"R": {"kind": "record", "fields": [["me", "R"]]}}
CHAIN = {"m0": {"params": [], "body": ["lit", True]}} | {
    f"m{n}": {"params": [], "body": ["model_call", f"m{n - 1}", []]}
    for n in range(1, 65)
}
INC_MODELS = {"inc": {"params": [["a", "U8"]], "body": ["add", var("a"), u8(1)]}}


@pytest.mark.parametrize(
    ("what", "edits"),
    [
        ("an unknown tag", {"ir": ["bogus"]}),
        ("a tag of a missing level", {"ir": ["some", u8(1)]}),
        ("a subject call under role admit", {"ir": SUBJECT_CALL, "role": "admit"}),
        ("a model_call to a missing model", {"ir": ["model_call", "ghost", []]}),
        ("a model_order that is not the models", {"model_order": []}),
        (
            "a depth of 65",
            {"ir": nest(65)},
        ),
        ("an unbound variable", {"ir": var("zz")}),
        ("an op that is missing", {"ir": ["subject_op", "ghost", []]}),
        (
            "an impure guard",
            {
                "ir": [
                    "and",
                    ["eq", ["subject_op", "bump", [var("a")]], u8(1)],
                    ["lit", True],
                ]
            },
        ),
        ("a model chain of 65", {"models": CHAIN, "model_order": list(CHAIN)}),
    ],
)
def test_the_table_comes_before_every_other_check_of_v2_ir(what, edits):
    fields = dict(edits)
    ir = fields.pop("ir", ["lit", True])
    got = outcome_ir(case(ir, **fields, types=BAD_TABLE), STANDARD)
    assert got == {"error": "ill-typed"}, what


def test_a_table_comes_before_a_model_body_over_the_cost_bound():
    body = ["lit", True]
    for _ in range(11):  # 2^12 - 1 nodes on the worst path, 11 deep
        body = ["and", body, body]
    models = {"heavy": {"params": [], "body": body}}
    plain = outcome_ir(
        case(["lit", True], models=models, model_order=["heavy"]), STANDARD
    )
    assert plain == {"error": "ir-cost"}
    both = outcome_ir(
        case(["lit", True], models=models, model_order=["heavy"], types=BAD_TABLE),
        STANDARD,
    )
    assert both == {"error": "ill-typed"}


@pytest.mark.parametrize("bound", [1999, 65537, "2000", None, True])
def test_the_bound_on_max_ir_cost_comes_before_the_table(bound):
    # R115 (b): the lanes' order, pinned for the oracle to follow.
    got = outcome_ir(case(["lit", True], types=BAD_TABLE, max_ir_cost=bound), STANDARD)
    assert got == {"error": "ir-cost"}
    # and with a good table the bound alone is the same refusal
    assert outcome_ir(case(["lit", True], max_ir_cost=bound), STANDARD) == {
        "error": "ir-cost"
    }


def test_a_bad_table_with_a_bound_that_holds_is_the_tables_refusal():
    got = outcome_ir(case(["lit", True], types=BAD_TABLE, max_ir_cost=2000), STANDARD)
    assert got == {"error": "ill-typed"}


# ===========================================================================
# R116: a list descriptor does not hide the models
# ===========================================================================
LIST_BINDER = [
    "l",
    {
        "type": "List<U8>",
        "domain": {"d": "list", "of": LEAF, "size": 15},
    },
]


def list_suite(models):
    corpus, row = bare([("b", "U8", LEAF)])
    # `ir.list` is a defined level the lane lacks: the row says so, as a real one does
    row = copy.deepcopy(row) | {
        "binders": [LIST_BINDER],
        "requires": ["ir.core", "ir.list"],
    }
    corpus = corpus | {"models": models, "model_order": list(models)}
    return corpus, row


def test_a_list_binder_alone_is_level_missing():
    corpus, row = list_suite(copy.deepcopy(MODELS))
    assert code_of(lambda: C.build_suite(corpus, row)) == "level-missing"


@pytest.mark.parametrize(
    "models",
    [
        {"m": {"params": [], "body": ["bogus_tag"]}},
        {"m": {"params": [], "body": ["subject_op", "f", []]}},
        {"m": {"params": [], "body": ["model_call", "ghost", []]}},
        {"m": {"params": [], "body": nest(65)}},
        {
            "m": {"params": [], "body": ["lit", True]},
            "n": {"params": [], "body": ["model_call", "m", []]},
        },
    ],
    ids=["unknown tag", "subject call", "missing callee", "depth", "good chain"],
)
def test_a_list_binder_does_not_hide_a_model_defect(models):
    corpus, row = list_suite(models)
    want = "level-missing" if "n" in models else "corpus"
    assert code_of(lambda: C.build_suite(corpus, row)) == want


def test_an_unknown_tag_in_a_model_is_corpus_with_and_without_the_list_binder():
    models = {"m": {"params": [], "body": ["bogus_tag"]}}
    corpus, row = list_suite(models)
    with_list = code_of(lambda: C.build_suite(corpus, row))
    plain, plain_row = bare([("b", "U8", LEAF)])
    plain = plain | {"models": models, "model_order": ["m"]}
    without = code_of(lambda: C.build_suite(plain, plain_row))
    assert with_list == without == "corpus"


# ===========================================================================
# R117: an error type is not a value type
# ===========================================================================
@pytest.mark.parametrize(
    "text", ["E", "Option<E>", "List<E>", "(U8,E)", "Result<E,E>", "Result<U8,R>"]
)
def test_the_type_readers_refuse_an_error_name_as_a_value_type(text):
    types = {"E": ERR, "R": {"kind": "record", "fields": [["a", "U8"]]}}
    assert code_of(lambda: parse_type(text, types)) == "type"
    assert code_of(lambda: parse_type(text, types, allow_int=True)) == "type"


def test_an_error_name_is_a_type_only_inside_result():
    types = {"E": ERR}
    assert str(parse_type("Result<U8,E>", types)) == "Result<U8,E>"
    assert str(parse_type("Option<Result<(U8,Bool),E>>", types)) == (
        "Option<Result<(U8,Bool),E>>"
    )


def test_every_context_refuses_an_error_typed_value():
    types = {"E": ERR}
    # v2-value: the type of the value
    got = outcome_value(
        {"type": "E", "types": types, "input_hex": b"1".hex()}, STANDARD
    )
    assert got == {"outcome": "reject", "error": "type"}
    # v2-ir: an op parameter, a model parameter, an env entry
    ops = {"f": {"params": [["a", "E"]], "ret": "U8", "requires": ["ir.core"]}}
    got = outcome_ir(case(["lit", True], types=types, ops=ops), STANDARD)
    assert got == {"error": "ill-typed"}
    models = {"m": {"params": [["x", "E"]], "body": ["lit", True]}}
    got = outcome_ir(
        case(["lit", True], types=types, models=models, model_order=["m"]), STANDARD
    )
    assert got == {"error": "ill-typed"}
    # an env entry (R117 b/c: the code of the context, not a kit error)
    for text in ("E", "Option<E>", "(U8,E)"):
        got = outcome_ir(
            case(["lit", True], types=types, env=[["x", {"type": text, "value": 1}]]),
            STANDARD,
        )
        assert got == {"error": "ill-typed"}, text
    # a corpus: a binder, an op, a model
    corpus, row = bare([("b", "U8", LEAF)], types=types)
    row = copy.deepcopy(row)
    row["binders"][0][1]["type"] = "E"
    assert code_of(lambda: C.build_suite(corpus, row)) == "corpus"
    corpus, row = bare([("b", "U8", LEAF)], types=types)
    assert code_of(lambda: C.build_suite(corpus | {"ops": ops}, row)) == "corpus"
    models = {"m": {"params": [["x", "E"]], "body": ["lit", True]}}
    corpus = corpus | {"models": models}
    assert code_of(lambda: C.build_suite(corpus, row)) == "corpus"


def test_a_fault_over_an_error_typed_field_is_ill_typed():
    table = FAULT_TABLE_DEFECTS["a field of an error type"]
    assert outcome_fault(fault_case(types=table), STANDARD) == {"error": "ill-typed"}


def test_the_shared_op_table_still_builds():
    # a sanity check that the helpers these tests lean on are the real ones
    assert OPS["f"]["ret"] == "U8" and BUMP["bump"]["ret"] == "U8"


def test_names_misplaced_error_type_reads_only_what_is_a_type_text():
    from coding_adventures_warrant_corpus.v2.wtypes import names_misplaced_error_type

    types = {"E": ERR}
    assert names_misplaced_error_type("Option<E>", types)
    assert not names_misplaced_error_type("Result<U8,E>", types)
    assert not names_misplaced_error_type("U8 U8", types)
    assert not names_misplaced_error_type(7, types)
