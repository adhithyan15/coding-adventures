"""CV2-F17 / VER04r 16.22: [D74] ``ill-typed`` in ``v2-fault``, [D75] ``Int`` model parameter, R113, R115 (b), R117.

* [D74], R110: a ``v2-fault`` case's ``types`` is read whole and first (``ill-typed``), then ``level-missing``, then
  ``subject-forbidden``, then the typing of the row (``ill-typed``); ``instantiate_fault`` no longer crashes on an
  undeclared field.
* [D75], R111: a model parameter may be ``Int`` (also under ``Option``, ``Result``, a tuple, ``List``); an op row, an
  ``env`` entry, a field and a value still refuse it.
* R113: ``bindings_v2.read_corpus`` reads ``corpus.types`` whole (a kit error: ``v2-binding`` has no table code).
* R115 (b): in ``v2-ir`` the bound on ``max_ir_cost`` comes before the table.
* R117: an error type is not a value type; every member of a table is parsed.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle import bindings_v2 as BV
from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

ERR = {"E": {"kind": "error", "cases": ["A", "B"]}}
OP = {"params": [["a", "Bytes"]], "ret": "Bytes", "requires": ["ir.core"]}
POST = {"fault": "p", "op": "echo", "kind": "post", "requires": ["ir.core"], "expr": ["var", "a"]}
TRUE = ["lit", True]
LIT = ["lit", {"type": "Int", "value": "3"}]


def fault_case(fault=POST, types=None, **kw):
    case = {"fault": fault, "op": OP, "types": types or {}, "levels": ["ir.core"], "args": ["0a"], "result": "0b"}
    case.update(kw)
    return case


def ir_case(ir, **kw):
    case = {"levels": ["ir.core"], "ir": ir, "env": [], "types": {}, "models": {}, "model_order": [], "ops": {},
            "subjects": {}}
    case.update(kw)
    return case


def rec(*fields):
    return {"kind": "record", "fields": [list(f) for f in fields]}


def var(*ctors):
    return {"kind": "variant", "ctors": [[n, list(t)] for n, t in ctors]}


def refusal(types, code):
    with pytest.raises(C.Refused) as exc:
        C.check_type_table(types, code)
    assert exc.value.code == code


# ------------------------------------------------------------------ the table (R108, R117, R110 a)

BAD_TABLES = {
    "self": {"R": rec(("a", "R"))},
    "list-of-self": {"R": rec(("a", "List<R>"))},
    "option-pair": {"A": rec(("b", "Option<B>")), "B": rec(("a", "A"))},
    "repeated-field": {"R": rec(("a", "U8"), ("a", "Bool"))},
    "repeated-ctor": {"V": var(("X", []), ("X", ["U8"]))},
    "undeclared": {"R": rec(("a", "Nope"))},
    "error-field": {**ERR, "R": rec(("e", "E"))},
    "error-under-option": {**ERR, "R": rec(("e", "Option<E>"))},
    "error-in-tuple": {**ERR, "R": rec(("e", "(E,Bool)"))},
    "error-in-ctor": {**ERR, "V": var(("X", []), ("Y", ["E"]))},
    "result-q-record": {"Q": rec(("a", "U8")), "R": rec(("r", "Result<U8,Q>"))},
    "int-field": {"R": rec(("a", "Int"))},
    "int-under-option": {"R": rec(("a", "Option<Int>"))},
    "malformed-text": {"R": rec(("a", "Option<U8"))},
    "non-text": {"R": rec(("a", 7))},
    "over-deep": {"R": rec(("a", "Option<" * 17 + "U8" + ">" * 17))},
    "name-grammar": {"R": rec(("1a", "U8"))},
    "row-not-object": {"R": 3},
    "row-kind": {"R": {"kind": "alias"}},
    "no-fields": {"R": rec()},
    "one-ctor": {"V": var(("X", []))},
    "nine-fields": {"V": var(("X", []), ("Y", ["U8"] * 9))},
    "empty-error": {"E": {"kind": "error", "cases": []}},
    "repeated-case": {"E": {"kind": "error", "cases": ["A", "A"]}},
    "malformed-row": {"R": {"kind": "record"}},
    "unhashable-name": {"R": {"kind": "record", "fields": [[["x"], "U8"]]}},
    "bad-field-pair": {"R": {"kind": "record", "fields": [["a", "U8", 1]]}},
    "ctor-fields-not-list": {"V": {"kind": "variant", "ctors": [["X", []], ["Y", "U8"]]}},
}


@pytest.mark.parametrize("name", sorted(BAD_TABLES))
def test_a_bad_table_is_refused_whole_with_the_contexts_code(name):
    for code in ("ill-typed", "corpus", "type"):
        refusal(BAD_TABLES[name], code)


def test_a_table_that_is_not_an_object_is_refused():
    refusal([], "ill-typed")


def test_result_with_an_error_second_argument_is_fine():
    C.check_type_table({**ERR, "R": rec(("r", "Result<U8,E>"), ("s", "Option<Result<(U8,Bool),E>>"))}, "x")
    C.check_type_table({}, "x")


def test_the_walk_survives_a_deep_chain_of_records():
    types = {f"R{i}": rec(("a", f"R{i + 1}")) for i in range(3000)}
    types["R3000"] = rec(("a", "U8"))
    C.check_type_table(types, "ill-typed")
    types["R3000"] = rec(("a", "R0"))
    refusal(types, "ill-typed")


def test_an_error_name_is_not_a_value_type_in_the_type_reader():
    with pytest.raises(C.Refused) as exc:
        C.parse_type("E", ERR)
    assert exc.value.code == "type"
    with pytest.raises(C.Refused):
        C.parse_type("Option<E>", ERR)
    assert C.parse_type("Result<U8,E>", ERR).head == "Result"
    with pytest.raises(C.Refused):
        C.parse_type("Result<U8,E>", {"E": 3})  # a malformed row is a refusal, not an AttributeError


def test_lowering_refuses_a_record_field_of_an_error_type():
    src = b"warrant t v1\n  error E = A | B\n  record R(e: E)\n  op f(r: R) -> Bool\n"
    with pytest.raises(OracleError) as exc:
        lower_document(parse_source(src))
    assert exc.value.code == "type" and "not a value type" in exc.value.message


# ------------------------------------------------------------------ [D74], R110: v2-fault


def test_v2_fault_has_ill_typed_in_its_closed_set():
    assert C.ERROR_CODES["v2-fault"] == {"ill-typed", "level-missing", "subject-forbidden"}


@pytest.mark.parametrize("name", sorted(BAD_TABLES))
def test_a_fault_case_with_a_bad_table_is_ill_typed(name):
    assert C.derive_fault(fault_case(types=BAD_TABLES[name])) == {"error": "ill-typed"}


def test_the_table_comes_before_level_and_subject_and_typing():
    bad = BAD_TABLES["self"]
    swap = {"params": [["a", "U8"], ["b", "U8"]], "ret": "U8", "requires": ["ir.core"]}
    needs_level = C.instantiate_fault("xor-lsb", "swap", swap, {})
    assert needs_level["requires"] == ["ir.fixed"]
    wide = {**fault_case(needs_level, bad), "op": swap, "args": ["3", "4"], "result": "7"}
    assert C.derive_fault(wide) == {"error": "ill-typed"}
    assert C.derive_fault({**wide, "types": {}}) == {"error": "level-missing"}
    subject = {**POST, "expr": ["subject_op", "echo", [["var", "a"]], [], []]}
    assert C.derive_fault(fault_case(subject, types=bad)) == {"error": "ill-typed"}
    # then level-missing, then subject-forbidden, then the typing, on a good table
    assert C.derive_fault(fault_case(subject)) == {"error": "subject-forbidden"}
    assert C.derive_fault(fault_case({**POST, "expr": ["not", ["var", "a"]]})) == {"error": "ill-typed"}


def test_the_table_is_checked_even_when_the_row_does_not_reach_it():
    assert C.derive_fault(fault_case(types={"R": rec(("a", "Nope"))})) == {"error": "ill-typed"}


def test_a_row_with_no_common_type_is_ill_typed():
    # R94: an expression of another type than the result it replaces, and a pre rewrite of another type
    assert C.derive_fault(fault_case({**POST, "kind": "replace", "expr": TRUE})) == {"error": "ill-typed"}
    pre = {"fault": "p", "op": "echo", "kind": "pre", "requires": ["ir.core"],
           "rewrites": [{"param": "a", "expr": TRUE}]}
    assert C.derive_fault(fault_case(pre)) == {"error": "ill-typed"}


def test_an_op_row_that_names_int_or_an_undeclared_type_is_ill_typed_in_a_fault_case():
    for ty in ("Int", "Nope"):
        op = {"params": [["a", ty]], "ret": "Bytes", "requires": ["ir.core"]}
        assert C.derive_fault({**fault_case(), "op": op}) == {"error": "ill-typed"}


def test_instantiate_fault_survives_an_undeclared_field():
    types = {**ERR, "R": rec(("a", "Nope")), "P": rec(("x", "U8"), ("y", "Bool"))}
    op = {"params": [["a", "U8"]], "ret": "Result<P,E>", "requires": ["ir.core"]}
    row = C.instantiate_fault("field-zero", "f", op, types)
    assert row["fault"] == "field-zero" and row["requires"] == ["ir.data", "ir.tuple"]
    case = {**fault_case(row, types), "op": op, "args": ["1"], "result": {"ok": ["1", True]},
            "levels": ["ir.core", "ir.data", "ir.tuple"]}
    assert C.derive_fault(case) == {"error": "ill-typed"}
    cyc = {**types, "P": rec(("x", "U8"), ("p", "P"))}  # a record holding itself: still written, then refused
    row = C.instantiate_fault("field-zero", "f", op, cyc)
    assert C.derive_fault({**case, "fault": row, "types": cyc}) == {"error": "ill-typed"}


# ------------------------------------------------------------------ [D75], R111: Int model parameter


def test_a_model_parameter_may_be_int():
    models = {"m": {"params": [["p", "Int"]], "body": ["eq", ["var", "p"], LIT]}}
    built = C.derive_ir(ir_case(["model_call", "m", [LIT]], models=models, model_order=["m"]))
    assert built["type"] == "Bool" and built["value"] is True


@pytest.mark.parametrize("text,levels", [("Int", ["ir.core"]), ("Result<Int,E>", ["ir.core"]),
                                         ("(Int,Bool)", ["ir.core", "ir.tuple"]),
                                         ("Option<Int>", ["ir.core", "ir.data"])])
def test_int_is_read_through_option_result_tuple_and_list_in_a_model_parameter(text, levels):
    models = {"m": {"params": [["p", text]], "body": TRUE}}
    built = C.compile_ir_case(ir_case(TRUE, models=models, model_order=["m"], levels=levels, types=ERR))
    assert built.prog.models["m"].params[0][1].head in ("Int", "Result", "()", "List", "Option")


def test_the_reader_accepts_int_under_a_list_in_a_model_parameter():
    models = {"m": {"params": [["p", "List<Int>"]], "body": TRUE}}
    # the oracle's evaluator holds no ``ir.list`` yet, but the reader does not refuse the parameter as ``type``
    with pytest.raises(C.KitError, match="reserved level"):
        C.derive_ir(ir_case(TRUE, models=models, model_order=["m"], levels=["ir.core", "ir.list"]))


def test_a_reached_int_model_parameter_brings_its_wrappers_level():
    models = {"m": {"params": [["p", "Option<Int>"]], "body": TRUE}}
    case = ir_case(["model_call", "m", [["none"]]], models=models, model_order=["m"])
    assert C.derive_ir(case) == {"error": "level-missing"}


def test_int_stays_refused_in_an_op_row_an_env_entry_a_field_and_a_value():
    for ty in ("Int", "Option<Int>"):
        op = {"params": [["a", ty]], "ret": "Bool", "requires": ["ir.core"]}
        assert C.derive_ir(ir_case(LIT, ops={"f": op})) == {"error": "ill-typed"}
    # an env entry: ``type``, outside the closed set of ``v2-ir``, so the kit build fails (R111 (c))
    assert C.derive_ir(ir_case(["var", "x"], env=[["x", {"type": "Int", "value": "3"}]])) == {"error": "type"}
    assert C.derive_ir(ir_case(LIT, types={"R": rec(("a", "Int"))})) == {"error": "ill-typed"}
    for text in ("Int", "Option<Int>"):
        with pytest.raises(C.Refused):
            C.parse_type(text, {})


# ------------------------------------------------------------------ R113: v2-binding corpus.types


def binding_corpus(types):
    return {"types": types, "ops": {"f": {"params": [["a", "Bool"]], "ret": "Bool"}}}


@pytest.mark.parametrize("name", ["self", "list-of-self", "option-pair", "repeated-field", "repeated-ctor",
                                  "undeclared", "error-field", "error-under-option", "result-q-record"])
def test_a_binding_corpus_with_a_bad_table_is_a_kit_error(name):
    with pytest.raises(BV.BindingKitError):
        BV.read_corpus(binding_corpus(BAD_TABLES[name]))


def test_a_binding_corpus_with_a_good_table_is_read():
    assert BV.read_corpus(binding_corpus({**ERR, "R": rec(("a", "U8"), ("r", "Result<U8,E>"))})).types


# ------------------------------------------------------------------ R115 (b): the bound before the table


def test_the_cost_bound_comes_before_the_table_in_v2_ir():
    bad = BAD_TABLES["repeated-field"]
    assert C.derive_ir(ir_case(["bogus"], types=bad)) == {"error": "ill-typed"}
    for cost in (1999, 65_537, "x", True):
        assert C.derive_ir(ir_case(["bogus"], types=bad, max_ir_cost=cost)) == {"error": "ir-cost"}
    assert C.derive_ir(ir_case(["bogus"], types=bad, max_ir_cost=2000)) == {"error": "ill-typed"}
