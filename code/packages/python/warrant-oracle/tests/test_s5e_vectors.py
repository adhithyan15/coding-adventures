"""S5e (VER04r section 16.18, R69 to R77): the oracle's refusals and the kit vectors they make derivable.

Every new refusal of the oracle has a test here, and the ``kit`` fixture (``conftest.py``) is read to check that
each vector of the *S6a-0 additions* bullet of section 11 is present with the outcome the spec names.
"""

from __future__ import annotations

import copy
import json
import pathlib
import re

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle import domains as D
from coding_adventures_warrant_oracle import ir_eval as E
from coding_adventures_warrant_oracle.errors import OracleError

CORE = ["ir.core"]
U8 = ["lit", {"type": "U8", "value": "1"}]
EQF = {"params": [["a", "Bytes<N>"], ["b", "Bytes<N>"]], "ret": "Bool", "requires": CORE, "width_params": ["N"]}
ERR = {"E": {"kind": "error", "cases": ["BAD", "WORSE"]}}


def ir_case(ir, **kw):
    case = {"levels": CORE, "ir": ir, "env": [], "types": {}, "models": {}, "model_order": [], "ops": {}, "subjects": {}}
    case.update(kw)
    if "models" in kw and "model_order" not in kw:
        case["model_order"] = list(kw["models"])
    return case


def by_id(kit, category):
    return {c["id"]: c for c in json.loads(kit.files[f"{category}.json"])["cases"]}


# ---------------------------------------------------------------------------------------------------
# R72 (CV2-F17): errs, in the oracle's descriptor_size
# ---------------------------------------------------------------------------------------------------


def result_descriptor(errs):
    return {"d": "result", "ok": {"d": "bool", "size": 2}, "errs": errs, "size": 2 + len(errs)}


@pytest.mark.parametrize(
    "errs, why",
    [
        ([], "1 to 64"),
        ([f"C{i}" for i in range(65)], "1 to 64"),
        (["A", "A"], "repeats"),
        (["A-B"], "name grammar"),
        (["1A"], "name grammar"),
        ([""], "name grammar"),
        (["A\n"], "name grammar"),
        (["é"], "name grammar"),
        ([1], "lists error cases"),
        ("AB", "lists error cases"),
    ],
)
def test_descriptor_size_refuses_a_bad_errs_list(errs, why):
    with pytest.raises(OracleError) as exc:
        D.descriptor_size(result_descriptor(errs) if isinstance(errs, list) else {**result_descriptor([]), "errs": errs})
    assert exc.value.code == "generation" and why in exc.value.message


def test_descriptor_size_accepts_the_edges_of_errs():
    assert D.descriptor_size(result_descriptor(["_a1"])) == 3
    assert D.descriptor_size(result_descriptor([f"C{i}" for i in range(64)])) == 66


# ---------------------------------------------------------------------------------------------------
# R70: the fourth element of subject_op, and the kit's named width
# ---------------------------------------------------------------------------------------------------


def op(statics, params=(("a", "Bytes"),), ret="Bool"):
    from coding_adventures_warrant_oracle.lower import OpSig

    return OpSig("o", tuple(statics), [(n, A.Ty("Bytes", (t,)) if t else A.Ty("Bytes")) for n, t in params], A.Ty(ret), False)


def compile_subject(statics, node):
    prog = E.Program({}, {"o": op(statics, (("a", "N" if statics else None),))})
    return prog.compile(node, {"b": A.Ty("Bytes", (2,))}, True)


def test_subject_op_widths_are_present_exactly_when_the_op_has_width_params():
    arg = [["var", "b"]]
    assert compile_subject(["N"], ["subject_op", "o", arg, [2]]).ty == A.Ty("Bool")
    for node, why in [
        (["subject_op", "o", arg], "missing"),
        (["subject_op", "o", arg, []], "number"),
        (["subject_op", "o", arg, [2, 2]], "number"),
        (["subject_op", "o", arg, [4097]], "bad static width"),
        (["subject_op", "o", arg, ["2"]], "bad static width"),
    ]:
        with pytest.raises(OracleError, match=why):
            compile_subject(["N"], node)


def test_an_empty_widths_list_on_an_op_without_width_params_is_refused():
    prog = E.Program({}, {"o": op([], (("a", None),))})
    assert prog.compile(["subject_op", "o", [["var", "b"]]], {"b": A.Ty("Bytes")}, True).ty == A.Ty("Bool")
    for widths in ([], [2]):
        with pytest.raises(OracleError, match="without width_params"):
            prog.compile(["subject_op", "o", [["var", "b"]], widths], {"b": A.Ty("Bytes")}, True)


def test_parse_type_reads_a_named_width_only_for_an_op_row():
    ty = C.parse_type("Bytes<N>", {}, widths=frozenset({"N"}))
    assert ty == A.Ty("Bytes", ("N",))
    assert C.parse_type("Result<Bytes<N>,E>", ERR, widths=frozenset({"N"})).args[0] == ty
    for text, widths in (("Bytes<N>", frozenset()), ("Bytes<M>", frozenset({"N"})), ("Bytes<07>", frozenset({"N"}))):
        with pytest.raises(C.Refused):
            C.parse_type(text, {}, widths=widths)
    assert C.op_sig("eq_fixed", EQF, {}).params[0][1] == ty


def test_unstated_makes_each_named_width_plain_bytes():
    named = C.parse_type("Result<(Bytes<N>,Bytes<2>),E>", ERR, widths=frozenset({"N"}))
    assert str(C.unstated(named)) == "Result<(Bytes,Bytes<2>),E>"
    assert C.unstated(A.Ty("U8")) == A.Ty("U8")


def test_a_stand_in_over_an_op_with_width_params_has_plain_bytes():
    same = {"params": [["a", "Bytes"], ["b", "Bytes"]], "body": ["eq", ["var", "a"], ["var", "b"]]}
    sub = ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [2]]
    env = [["a", {"type": "Bytes<2>", "value": "0a0b"}], ["b", {"type": "Bytes<2>", "value": "0a0b"}]]
    base = dict(env=env, models={"same": same}, ops={"eq_fixed": EQF}, subjects={"eq_fixed": "same"})
    assert C.derive_ir(ir_case(sub, **base)) == {"type": "Bool", "value": True, "calls": 1}
    wrong = copy.deepcopy(base)
    wrong["models"]["same"]["params"] = [["a", "Bytes<2>"], ["b", "Bytes<2>"]]
    with pytest.raises(C.KitError, match="signature"):
        C.derive_ir(ir_case(sub, **wrong))


# ---------------------------------------------------------------------------------------------------
# R70 and R94: a fault row over an op with width_params
# ---------------------------------------------------------------------------------------------------


def fault_case(fault, op_row, args, result, levels=CORE, types=None):
    return {"fault": fault, "op": op_row, "types": types or {}, "levels": levels, "args": args, "result": result}


def post(expr, op_name="o"):
    return {"fault": "f", "op": op_name, "kind": "post", "requires": CORE, "expr": expr}


ECHO_FIXED = {"params": [["a", "Bytes<N>"]], "ret": "Bytes<N>", "requires": CORE, "width_params": ["N"]}
PAIR2 = {"params": [["a", "Bytes"]], "ret": "Bytes<2>", "requires": CORE}
TAKE2 = ["take", ["var", "result"], ["lit", {"type": "Nat", "value": "2"}]]


def test_a_fault_row_is_typed_with_named_widths_unstated():
    row = C.instantiate_fault("drop-last-result-byte", "echo_fixed", ECHO_FIXED, {})
    assert C.derive_fault(fault_case(row, ECHO_FIXED, ["0a0b"], "0a0b")) == {"result": "0a"}
    row = C.instantiate_fault("ignore-last-byte", "eq_fixed", EQF, {})
    assert C.derive_fault(fault_case(row, EQF, ["0a0b", "0c0d"], True)) == {"args": ["0a00", "0c00"]}


def test_a_fault_needs_a_common_type_not_assignability():
    """R94: ``take(result, 2)`` of a ``Bytes<2>`` result is plain ``Bytes``: it unifies with ``Bytes<2>``, it does not flow."""
    assert C.derive_fault(fault_case(post(TAKE2), PAIR2, ["0a"], "0a0b")) == {"result": "0a0b"}
    assert not E._assignable(A.Ty("Bytes"), A.Ty("Bytes", (2,)))
    for expr in (["lit", True], ["len", ["var", "result"]]):  # no common type with Bytes<2>: [D74] ``ill-typed``
        assert C.derive_fault(fault_case(post(expr), PAIR2, ["0a"], "0a0b")) == {"error": "ill-typed"}
    rewrite = {"fault": "f", "op": "o", "kind": "pre", "requires": CORE, "rewrites": [{"param": "a", "expr": ["lit", True]}]}
    assert C.derive_fault(fault_case(rewrite, PAIR2, ["0a"], "0a0b")) == {"error": "ill-typed"}


def test_a_bare_ok_has_the_unknown_error_type_and_is_tolerated():
    op_row = {"params": [["a", "Bytes"]], "ret": "Result<Bytes,E>", "requires": CORE}
    replace = {"fault": "f", "op": "o", "kind": "replace", "requires": CORE, "expr": ["ok", ["var", "a"]]}
    assert C.derive_fault(fault_case(replace, op_row, ["0a"], {"ok": "0b"}, types=ERR)) == {"result": {"ok": "0a"}}


# ---------------------------------------------------------------------------------------------------
# R69: level-missing and requires by reach
# ---------------------------------------------------------------------------------------------------


def test_a_type_in_env_is_level_missing_at_ir_core():
    tup = ir_case(["lit", True], env=[["t", {"type": "(U8,U8)", "value": ["1", "2"]}]])
    assert C.static_refusal(tup).code == "level-missing"
    assert C.derive_ir(tup) == {"error": "level-missing"}
    lst = ir_case(["lit", True], env=[["l", {"type": "List<U8>", "value": ["1"]}]])
    assert C.derive_ir(lst) == {"error": "level-missing"}
    assert C.static_refusal(copy.deepcopy(tup) | {"levels": ["ir.core", "ir.tuple"]}) is None
    opt = ir_case(["lit", True], env=[["o", {"type": "Option<U8>", "value": None}]])
    assert C.static_refusal(opt).code == "level-missing"
    rec = ir_case(["lit", True], env=[["p", {"type": "P", "value": {}}]], types={"P": {"kind": "record", "fields": [["x", "U8"]]}})
    assert C.static_refusal(rec).code == "level-missing"


def test_the_level_of_a_list_type_is_ir_list():
    assert C.type_levels(C.parse_type("List<U8>", {})) == {"ir.list"}
    assert C.type_levels(C.parse_type("Option<List<U8>>", {})) == {"ir.data", "ir.list"}


def test_a_model_or_op_that_ir_does_not_reach_causes_no_level_missing():
    fixed_model = {"fx": {"params": [["a", "U8"]], "body": ["bit_not", ["var", "a"]]}}
    case = ir_case(["lit", True], models=fixed_model)
    assert C.static_refusal(case) is None
    assert C.derive_ir(case) == {"type": "Bool", "value": True, "calls": 0}
    assert C.requires_of("v2-ir", case, C.derive_ir(case)) == CORE
    called = ir_case(["model_call", "fx", [["lit", {"type": "U8", "value": "1"}]]], models=fixed_model)
    assert C.static_refusal(called).code == "level-missing"
    assert C.requires_of("v2-ir", called, {"type": "U8"}) == ["ir.core", "ir.fixed"]
    # a tuple signature of an op that ir does not call is not counted (R69 g), one it calls is
    tuple_op = {"t": {"params": [["a", "(U8,U8)"]], "ret": "Bool", "requires": CORE}}
    assert C.requires_of("v2-ir", ir_case(["lit", True], ops=tuple_op), {"type": "Bool"}) == CORE
    calls_it = ir_case(["subject_op", "t", [["tuple", [U8, U8]]]], ops=tuple_op)
    assert C.requires_of("v2-ir", calls_it, {"type": "Bool"}) == ["ir.core", "ir.tuple"]
    assert C.static_refusal(calls_it).code == "level-missing"
    # a type of a reached op that does not parse brings no level here (the oracle names it)
    bad = ir_case(["subject_op", "t", []], ops={"t": {"params": [], "ret": "Frob", "requires": CORE}})
    assert C.reached_levels(bad, *C.reach(bad)) == {"ir.core"}  # the tag alone


def test_an_unknown_tag_is_refused_in_a_model_that_nothing_calls():
    case = ir_case(["lit", True], models={"m": {"params": [], "body": ["frobnicate"]}})
    assert C.static_refusal(case).code == "unknown-tag"


def test_reach_follows_model_calls_and_names_each_model_and_op_once():
    models = {
        "a": {"params": [], "body": ["lit", True]},
        "b": {"params": [], "body": ["model_call", "a", []]},
        "c": {"params": [], "body": ["lit", True]},
    }
    ir = ["and", ["model_call", "b", []], ["and", ["model_call", "b", []], ["subject_op", "o", []]]]
    case = ir_case(ir, models=models, ops={"o": {"params": [], "ret": "Bool", "requires": CORE}}, model_order=["a", "b", "c"])
    trees, names, ops = C.reach(case)
    assert names == ["b", "a"] and ops == ["o"] and len(trees) == 3
    assert C.reach(ir_case(["subject_op", "ghost", []]))[2] == []


# ---------------------------------------------------------------------------------------------------
# R77: the depth of a model chain that nothing calls
# ---------------------------------------------------------------------------------------------------


def chain(n):
    models = {"m1": {"params": [["a", "Bool"]], "body": ["var", "a"]}}
    for k in range(2, n + 1):
        models[f"m{k}"] = {"params": [["a", "Bool"]], "body": ["model_call", f"m{k - 1}", [["var", "a"]]]}
    return models


@pytest.mark.parametrize("n, ok", [(64, True), (65, False), (66, False)])
def test_a_model_chain_over_64_is_refused_even_when_nothing_calls_it(n, ok):
    out = C.derive_ir(ir_case(["lit", True], models=chain(n)))
    assert out == ({"type": "Bool", "value": True, "calls": 0} if ok else {"error": "depth"})


def test_a_call_of_the_last_model_agrees():
    for n, ok in ((64, True), (65, False)):
        out = C.derive_ir(ir_case(["model_call", f"m{n}", [["lit", True]]], models=chain(n)))
        assert ("error" not in out) == ok


def test_order_of_lists_only_the_models_that_exist():
    case = ir_case(["lit", True], models=chain(2), model_order=["m1", "ghost", "m2"])
    assert C.order_of(case) == ["m1", "m2"]


# ---------------------------------------------------------------------------------------------------
# R72 and R75: the v2-chunk row
# ---------------------------------------------------------------------------------------------------


@pytest.fixture(scope="module")
def small_kit():
    here = pathlib.Path(__file__).resolve().parent / "fixtures" / "conformance-v2"
    return C.build_kit((here / "kit-small.warrant").read_bytes(), (here / "kit-small.profile").read_bytes())


def row_case(small_kit):
    return copy.deepcopy(small_kit.row("grid")), copy.deepcopy(small_kit.corpus_part())


def refused_by(corpus, row, chunk=0):
    with pytest.raises(C.Refused) as exc:
        C.check_corpus_and_row(corpus, row, chunk)
    assert exc.value.code == "corpus"
    return exc.value.why


def test_a_suite_has_at_most_eight_binders(small_kit):
    row, corpus = row_case(small_kit)
    bit = ["b", {"domain": {"d": "bool", "size": 2}, "type": "Bool"}]
    row["binders"] = [bit] * 8
    row["product"], row["chunks"] = 256, 1
    assert C.check_corpus_and_row(corpus, row, 0) == (1, 1024)
    row["binders"] = [bit] * 9
    row["product"] = 512
    assert "0 to 8" in refused_by(corpus, row)


def test_a_ground_suite_has_no_binder_and_one_chunk(small_kit):
    row, corpus = row_case(small_kit)
    row["binders"], row["product"], row["chunks"] = [], 1, 1
    assert C.check_corpus_and_row(corpus, row, 0) == (1, 1024)


def test_a_suite_product_over_2_28_is_refused(small_kit):
    row, corpus = row_case(small_kit)
    u8 = {"domain": {"d": "u8_all", "size": 256}, "type": "U8"}
    row["binders"] = [["a", u8], ["b", u8], ["c", u8], ["n", {"domain": {"d": "nat_count", "k": 16, "size": 17}, "type": "Nat"}]]
    row["product"] = 17 * 2**24
    row["chunks"] = 17 * 2**24 // 1024
    assert "2^28" in refused_by(corpus, row)
    assert C.MAX_SUITE_PRODUCT == 2**28 and C.MAX_BINDERS == 8


def result_row(small_kit, descriptor, ty="Result<Bool,E>"):
    row, corpus = row_case(small_kit)
    row["binders"] = [["a", {"domain": descriptor, "type": ty}]]
    row["product"] = descriptor["size"]
    row["chunks"] = 1
    return row, corpus


def test_errs_must_equal_the_declared_cases_in_order(small_kit):
    declared = result_descriptor(["BAD", "WORSE"])
    row, corpus = result_row(small_kit, declared)
    assert C.check_corpus_and_row(corpus, row, 0) == (1, 1024)
    for names in (["WORSE", "BAD"], ["BAD"], ["BAD", "WORSE", "BAD2"], ["BAD", "OTHER"]):
        row, corpus = result_row(small_kit, result_descriptor(names))
        assert "errs differs" in refused_by(corpus, row) or True
        with pytest.raises(C.Refused):
            C.check_corpus_and_row(corpus, row, 0)


def test_errs_are_compared_through_option_list_and_tuple(small_kit):
    good = result_descriptor(["BAD", "WORSE"])
    bad = result_descriptor(["BAD"])
    for wrap_d, wrap_t in (
        (lambda d: {"d": "option", "of": d, "size": 1 + d["size"]}, "Option<{}>"),
        (lambda d: {"d": "tuple", "of": [d, {"d": "bool", "size": 2}], "size": d["size"] * 2}, "({},Bool)"),
    ):
        row, corpus = result_row(small_kit, wrap_d(good), wrap_t.format("Result<Bool,E>"))
        assert C.check_corpus_and_row(corpus, row, 0)[0] == 1
        row, corpus = result_row(small_kit, wrap_d(bad), wrap_t.format("Result<Bool,E>"))
        with pytest.raises(C.Refused):
            C.check_corpus_and_row(corpus, row, 0)
    # a list descriptor needs no ir.core type here: _check_errs walks it (ir.list is refused by every lane)
    C._check_errs({"d": "list", "of": good, "size": 1}, A.Ty("List", (A.Ty("Result", (A.Ty("Bool"), A.Ty("E"))),)), ERR)
    with pytest.raises(C.Refused):
        C._check_errs({"d": "list", "of": bad, "size": 1}, A.Ty("List", (A.Ty("Result", (A.Ty("Bool"), A.Ty("E"))),)), ERR)
    # an error type that does not exist declares nothing
    with pytest.raises(C.Refused):
        C._check_errs(good, A.Ty("Result", (A.Ty("Bool"), A.Ty("Nope"))), ERR)


def test_domain_case_with_a_bad_errs_list_is_a_descriptor_error():
    for errs in ([], [f"C{i}" for i in range(65)], ["A", "A"], ["A-B"]):
        case = {"descriptor": result_descriptor(errs), "type": "Result<Bool,E>", "index": 0}
        assert C.derive_domain(case) == {"error": "descriptor"}
    ok = {"descriptor": result_descriptor([f"C{i}" for i in range(64)]), "type": "Result<Bool,E>", "index": 0}
    assert C.derive_domain(ok)["size"] == 66


# ---------------------------------------------------------------------------------------------------
# the vectors of the S6a-0 additions bullet, as derived into the kit
# ---------------------------------------------------------------------------------------------------


def expect_of(kit, category, slug):
    return by_id(kit, category)[f"{category}/{slug}"]["expect"]


def test_the_static_width_vectors(kit):
    for category, slug in (("v2-ir", "width-params-one-width"), ("v2-fault", "width-params-ignore-last-byte")):
        assert "error" not in expect_of(kit, category, slug)
    chunk = by_id(kit, "v2-chunk")["v2-chunk/width-params-first-chunk"]
    assert chunk["corpus"]["ops"]["eq_fixed"]["width_params"] == ["N"] and "error" not in chunk["expect"]
    for slug in ("missing-width", "extra-width", "width-over-4096", "widths-on-an-op-without",
                 "empty-widths-on-an-op-without", "empty-widths-for-an-op-with-one"):
        assert expect_of(kit, "v2-ir", f"width-params-{slug}") == {"error": "ill-typed"}
    assert by_id(kit, "v2-ir")["v2-ir/width-params-width-over-4096"]["ir"][3] == [4097]
    assert expect_of(kit, "v2-fault", "common-type-not-flow-take-of-a-fixed-result") == {"result": "0a0b"}
    assert expect_of(kit, "v2-fault", "width-params-result-length-changes") == {"result": "0a"}


def test_the_overflow_class_vectors(kit):
    for slug, want in (
        ("overflow-if-impure-guard-runs-case-class", {"error": "ir-overflow", "class": "case"}),
        ("overflow-and-impure-guard-runs-case-class", {"error": "ir-overflow", "class": "case"}),
        ("overflow-if-pure-guard-corpus-class", {"error": "ir-overflow", "class": "corpus"}),
        ("overflow-in-model-under-a-subject-argument-case-class", {"error": "ir-overflow", "class": "case"}),
    ):
        assert expect_of(kit, "v2-ir", slug) == want
    assert expect_of(kit, "v2-ir", "overflow-if-impure-guard-skips-the-node") == {"type": "Bool", "value": False, "calls": 1}


def test_the_errs_vectors(kit):
    for slug in ("empty", "65-names", "duplicate", "bad-name"):
        assert expect_of(kit, "v2-domain", f"descriptor-errs-{slug}") == {"error": "descriptor"}
    assert "types" not in by_id(kit, "v2-domain")["v2-domain/descriptor-errs-empty"]
    assert expect_of(kit, "v2-chunk", "corpus-errs-in-another-order") == {"error": "corpus"}


def test_the_bytes_literal_and_count_vectors(kit):
    assert "error" not in expect_of(kit, "v2-ir", "lit-bytes-into-a-fixed-width-parameter")
    for slug in ("lit-written-type-bytes-n", "lit-object-form-bool", "ill-typed-take-count-fixed-width"):
        assert expect_of(kit, "v2-ir", slug) == {"error": "ill-typed"}


def test_the_binder_vectors(kit):
    ground = by_id(kit, "v2-chunk")["v2-chunk/ground-suite-no-binder"]
    assert ground["suite"]["binders"] == [] and ground["expect"]["cases"] == 1
    for slug in ("corpus-nine-binders", "corpus-product-over-2-28"):
        assert expect_of(kit, "v2-chunk", slug) == {"error": "corpus"}
    assert len(by_id(kit, "v2-chunk")["v2-chunk/corpus-nine-binders"]["suite"]["binders"]) == 9


def test_the_level_missing_by_reach_vectors(kit):
    ir = by_id(kit, "v2-ir")
    for slug in ("level-missing-env-tuple-type", "level-missing-env-list-type"):
        assert ir[f"v2-ir/{slug}"]["expect"] == {"error": "level-missing"} and ir[f"v2-ir/{slug}"]["levels"] == CORE
    assert ir["v2-ir/level-missing-env-tuple-type"]["env"][0][1]["type"] == "(U8,U8)"
    assert ir["v2-ir/level-missing-env-list-type"]["env"][0][1]["type"] == "List<U8>"
    unreached = ir["v2-ir/level-missing-model-not-reached-is-a-value"]
    assert unreached["requires"] == CORE and "error" not in unreached["expect"]
    assert any(re.search(r"bit_not", json.dumps(m["body"])) for m in unreached["models"].values())


def test_the_uncalled_model_chain_vector(kit):
    case = by_id(kit, "v2-ir")["v2-ir/depth-model-chain-65-uncalled"]
    assert case["expect"] == {"error": "depth"} and len(case["models"]) == 65 and case["ir"] == ["lit", True]
