"""CV2-F17 / VER04r 16.23: [D77] and R119 to R126, the oracle's share.

* [D77], R119: ``v2-fault`` returns ``unknown-tag``, ``depth`` and ``model-order``, in section 11's order.
* R120: an ``env`` type the reader refuses is ``ill-typed`` in ``v2-ir``; a missing or non-text type is a kit error.
* R122: the signature of every op is read right after the table in ``v2-ir``.
* R123: a consumer's ``types`` table reserves ``V0_BUILTINS`` plus the six v1 type names, and nothing else.
* R124: an unreached model that needs a level outside ``levels`` is not compiled.
* R125: the ``v2-chunk`` reader reads ``corpus.ops``, ``corpus.models`` and ``model_order``.
* R126: a malformed fault row, or an undefined level name in ``fault.requires``, is a kit error.
"""

from __future__ import annotations

import copy
import pathlib

import pytest

from coding_adventures_warrant_oracle import conformance as C

OP = {"params": [["a", "Bytes"]], "ret": "Bytes", "requires": ["ir.core"]}
POST = {"fault": "p", "op": "echo", "kind": "post", "requires": ["ir.core"], "expr": ["var", "a"]}
A = ["var", "a"]
ERR = {"E": {"kind": "error", "cases": ["A", "B"]}}


def fault_case(fault=POST, types=None, op=OP, **kw):
    case = {"fault": fault, "op": op, "types": types or {}, "levels": ["ir.core"], "args": ["0a"], "result": "0b"}
    case.update(kw)
    return case


def post(expr, **kw):
    return {**POST, "expr": expr, **kw}


def nest(n):
    """``n`` nested ``concat`` nodes over ``["var", "a"]``: a tree ``n + 1`` levels deep."""
    expr = A
    for _ in range(n):
        expr = ["concat", expr, A]
    return expr


def ir_case(ir=("lit", True), **kw):
    case = {"levels": ["ir.core"], "ir": list(ir), "env": [], "types": {}, "models": {}, "model_order": [],
            "ops": {}, "subjects": {}}
    case.update(kw)
    return case


def model(params, body):
    return {"params": [list(p) for p in params], "body": body}


# ------------------------------------------------------------------------- [D77], R119: the three codes


def test_the_closed_set_holds_the_three_codes():
    assert {"unknown-tag", "depth", "model-order"} <= C.ERROR_CODES["v2-fault"]


def test_an_unknown_tag_in_a_post_row_is_unknown_tag():
    assert C.derive_fault(fault_case(post(["bogus", A]))) == {"error": "unknown-tag"}


def test_an_unknown_tag_in_a_pre_rewrite_is_unknown_tag():
    pre = {"fault": "p", "op": "echo", "kind": "pre", "requires": ["ir.core"],
           "rewrites": [{"param": "a", "expr": ["bogus", A]}]}
    assert C.derive_fault(fault_case(pre)) == {"error": "unknown-tag"}


def test_a_reserved_name_is_an_unknown_tag_as_in_v2_ir():
    # ``list`` and the ``ir.seq`` names are tags of no level the tables define (``static_refusal`` says the same)
    assert C.derive_fault(fault_case(post(["list", A]))) == {"error": "unknown-tag"}


def test_a_match_is_not_an_unknown_tag():
    arms = [[["ok", ["bind", "x"]], A], [["err", ["bind", "e"]], A]]
    case = fault_case(post(["match", ["ok", A], arms], requires=["ir.core", "ir.tuple"]), levels=["ir.core", "ir.tuple"])
    assert C.derive_fault(case) != {"error": "unknown-tag"}


def test_a_depth_of_64_is_accepted_and_65_is_depth():
    assert C.too_deep(nest(63)) is False and C.too_deep(nest(64)) is True
    assert C.derive_fault(fault_case(post(nest(63)))) == {"result": "0a" * 64}  # 64 levels: accepted
    assert C.derive_fault(fault_case(post(nest(64)))) == {"error": "depth"}  # 65 levels: refused
    assert C.derive_fault(fault_case(post(nest(500)))) == {"error": "depth"}


def test_the_depth_agrees_with_the_compilers_boundary():
    # the same tree in a v2-ir case: the compiler and ``too_deep`` draw the line in one place
    assert C.derive_ir(ir_case(nest(63), env=[["a", {"type": "Bytes", "value": "0a"}]]))["type"] == "Bytes"
    assert C.derive_ir(ir_case(nest(64), env=[["a", {"type": "Bytes", "value": "0a"}]])) == {"error": "depth"}


def test_a_model_call_is_model_order():
    assert C.derive_fault(fault_case(post(["model_call", "m", [A]]))) == {"error": "model-order"}


def test_each_code_comes_before_the_typing_of_the_row():
    ill = ["not", A]  # ill-typed beside the defect
    assert C.derive_fault(fault_case(post(ill))) == {"error": "ill-typed"}
    assert C.derive_fault(fault_case(post(["concat", ill, ["bogus"]]))) == {"error": "unknown-tag"}
    assert C.derive_fault(fault_case(post(["concat", ill, ["model_call", "m", []]]))) == {"error": "model-order"}
    assert C.derive_fault(fault_case(post(["concat", nest(70), ill]))) == {"error": "depth"}


def test_each_code_comes_before_subject_forbidden():
    subject = ["subject_op", "echo", [A]]
    assert C.derive_fault(fault_case(post(subject))) == {"error": "subject-forbidden"}
    assert C.derive_fault(fault_case(post(["concat", subject, ["bogus"]]))) == {"error": "unknown-tag"}
    assert C.derive_fault(fault_case(post(["concat", subject, ["model_call", "m", []]]))) == {"error": "model-order"}
    assert C.derive_fault(fault_case(post(["concat", subject, nest(70)]))) == {"error": "depth"}


def test_level_missing_comes_before_each_code():
    for expr in (["bogus", A], nest(70), ["model_call", "m", [A]]):
        assert C.derive_fault(fault_case(post(expr), levels=[])) == {"error": "level-missing"}


def test_the_table_and_the_op_row_come_before_each_code():
    bad = {"R": {"kind": "record", "fields": [["a", "Nope"]]}}
    assert C.derive_fault(fault_case(post(["bogus", A]), types=bad)) == {"error": "ill-typed"}
    assert C.derive_fault(fault_case(post(nest(70)), types=bad)) == {"error": "ill-typed"}
    bad_op = {**OP, "params": [["a", "Int"]]}
    assert C.derive_fault(fault_case(post(["bogus", A]), op=bad_op)) == {"error": "ill-typed"}
    assert C.derive_fault(fault_case(post(["bogus", A]), op=bad_op, levels=[])) == {"error": "ill-typed"}


def test_an_unknown_tag_adds_no_level_to_the_computed_requires():
    # the row's ``requires`` is checked against the levels of the tags the oracle knows (R119 (d))
    assert C.fault_levels(post(["bogus", A]) | {"kind": "post"}, C.op_sig("echo", OP, {})) == {"ir.core"}
    wrong = post(["bogus", A], requires=["ir.core", "ir.fixed"])
    with pytest.raises(C.KitError, match="requires"):
        C.derive_fault(fault_case(wrong))


def test_requires_of_a_v2_fault_case():
    assert C.requires_of("v2-fault", fault_case(post(["bogus", A])), {"error": "unknown-tag"}) == ["ir.core"]
    wide = fault_case(post(["bogus", A]), levels=["ir.core", "ir.fixed"])
    assert C.requires_of("v2-fault", wide, {"error": "unknown-tag"}) == ["ir.core", "ir.fixed"]  # the case's levels
    assert C.requires_of("v2-fault", wide, {"error": "depth"}) == ["ir.core"]  # the row's requires
    assert C.requires_of("v2-fault", wide, {"error": "model-order"}) == ["ir.core"]


def test_the_three_vectors_of_r119_e_build_as_cases():
    for slug, expr, code in (
        ("unknown-tag", ["bogus", A], "unknown-tag"),
        ("depth", nest(64), "depth"),
        ("model-order", ["model_call", "m", [A]], "model-order"),
    ):
        row = C.Row(slug, f"a {code} in a fault expression", fault_case(post(expr)), {"error": code})
        (case,) = C.build_category("v2-fault", [row])
        assert case["expect"] == {"error": code} and case["requires"] == ["ir.core"]


# ------------------------------------------------------------------------- R120: an env type


@pytest.mark.parametrize("text", ["Int", "Option<Int>", "Nope", "Result<U8,Q>", "Bytes<07>", "Bytes<5000>", "U8<", "",
                                  "E", "List<Int>"])
def test_an_env_type_the_reader_refuses_is_ill_typed(text):
    types = {"Q": {"kind": "record", "fields": [["a", "U8"]]}, **ERR}
    case = ir_case(env=[["x", {"type": text, "value": "1"}]], types=types)
    assert C.derive_ir(case) == {"error": "ill-typed"}


def test_an_env_type_refusal_is_ill_typed_in_compile_ir_case():
    with pytest.raises(C.Refused) as exc:
        C.compile_ir_case(ir_case(env=[["x", {"type": "Int", "value": "1"}]]))
    assert exc.value.code == "ill-typed"


def test_an_env_entry_that_breaks_the_schema_is_a_kit_error():
    for entry in (["x", {"value": "1"}], ["x", {"type": 3, "value": "1"}], ["x", {"type": "U8"}], ["x"], "x",
                  [3, {"type": "U8", "value": "1"}], ["x", 7]):
        with pytest.raises(C.KitError):
            C.derive_ir(ir_case(env=[entry]))


def test_a_value_that_does_not_decode_is_not_ill_typed():
    # ``value`` is outside the closed set of ``v2-ir``: ``finish_case`` refuses it, so it is never a vector (R120 (b))
    assert C.derive_ir(ir_case(env=[["x", {"type": "U8", "value": "999"}]])) == {"error": "value"}
    assert "value" not in C.ERROR_CODES["v2-ir"]


def test_a_good_env_entry_still_derives():
    out = C.derive_ir(ir_case(["var", "x"], env=[["x", {"type": "U8", "value": "7"}]]))
    assert out["type"] == "U8" and out["value"] == "7"


# ------------------------------------------------------------------------- R122: ops before everything else


BAD_ROWS = {
    "int-param": {"params": [["p", "Int"]], "ret": "Bool"},
    "int-ret": {"params": [], "ret": "Int"},
    "error-type": {"params": [["p", "E"]], "ret": "Bool"},
    "undeclared": {"params": [["p", "Nope"]], "ret": "Bool"},
    "unparseable": {"params": [["p", "U8<"]], "ret": "Bool"},
}


@pytest.mark.parametrize("name", sorted(BAD_ROWS))
def test_an_unused_bad_op_row_comes_before_every_tree_defect(name):
    ops = {"f": BAD_ROWS[name]}
    models = {"m": model([("p", "Bool")], ["lit", True])}
    defects = {
        "unknown-tag": ir_case(["bogus"], ops=ops, types=ERR),
        "level-missing": ir_case(["lit", True], ops=ops, types=ERR, levels=[]),
        "model-order": ir_case(["model_call", "missing", [["lit", True]]], ops=ops, types=ERR),
        "depth": ir_case(nest(70), ops=ops, types=ERR),
        "subject-forbidden": ir_case(["subject_op", "f", []], ops=ops, types=ERR, role="fault"),
        "model-in-the-way": ir_case(["lit", True], ops=ops, types=ERR, models=models, model_order=["m"]),
    }
    for label, case in defects.items():
        assert C.derive_ir(case) == {"error": "ill-typed"}, label


def test_the_bound_and_the_table_still_come_before_the_op_rows():
    ops = {"f": BAD_ROWS["int-param"]}
    assert C.derive_ir(ir_case(ops=ops, max_ir_cost=1)) == {"error": "ir-cost"}
    assert C.derive_ir(ir_case(ops=ops, types={"R": {"kind": "record", "fields": [["a", "R"]]}})) == {"error": "ill-typed"}


def test_an_unreached_op_row_that_needs_a_level_is_accepted():
    ops = {"f": {"params": [["p", "Option<U8>"]], "ret": "Bool"}}
    assert C.derive_ir(ir_case(ops=ops)) == {"type": "Bool", "value": True, "calls": 0}


def test_a_model_signature_is_left_open():
    # R122 (c): the oracle gives the tree's code for a model parameter that names an undeclared type
    models = {"m": model([("p", "Nope")], ["lit", True])}
    assert C.derive_ir(ir_case(["bogus"], models=models, model_order=["m"])) == {"error": "unknown-tag"}
    assert C.derive_ir(ir_case(models=models, model_order=["m"])) == {"error": "ill-typed"}


# ------------------------------------------------------------------------- R123: the table's reserved set


@pytest.mark.parametrize("name", ["U16", "U32", "I8", "I16", "I32", "I64", "Bool", "Int", "Nat", "U8", "U64", "Bytes",
                                  "List", "Option", "Result", "len", "at", "concat", "fixed", "div", "Some", "None",
                                  "Ok", "Err"])
def test_the_table_reserves_v0_builtins_and_the_six_v1_types(name):
    assert name in C.TABLE_RESERVED
    for row in ({"kind": "error", "cases": ["A"]}, {"kind": "record", "fields": [["a", "U8"]]}):
        with pytest.raises(C.Refused):
            C.check_type_table({name: row}, "corpus")


@pytest.mark.parametrize("name", ["rem", "div_trunc", "checked_add", "take", "drop", "imin", "imax", "slice", "count",
                                  "fold", "rotl"])
def test_the_table_does_not_reserve_the_v1_functions_or_level_names(name):
    C.check_type_table({name: {"kind": "error", "cases": ["A", "B"]}}, "corpus")
    C.check_type_table({"V": {"kind": "variant", "ctors": [[name, []], ["Other", []]]}}, "corpus")


@pytest.mark.parametrize("name", ["U16", "I64", "Some", "Bool"])
def test_a_constructor_name_is_checked_with_the_same_set(name):
    row = {"kind": "variant", "ctors": [[name, []], ["Other", []]]}
    with pytest.raises(C.Refused):
        C.check_type_table({"V": row}, "ill-typed")


def test_lowering_keeps_its_version_scoped_set():
    from coding_adventures_warrant_oracle import lower

    assert "rem" in lower._DECLARED_RESERVED_V1 and "rem" not in C.TABLE_RESERVED


# ------------------------------------------------------------------------- R124: an unreached model that needs a level


def uncalled(params, body=("lit", True), **kw):
    return ir_case(models={"m": model(params, list(body))}, model_order=["m"], **kw)


@pytest.mark.parametrize(
    "params, body",
    [
        ([("p", "Option<U8>")], ("lit", True)),
        ([("p", "Option<Int>")], ("lit", True)),
        ([("p", "List<U8>")], ("lit", True)),
        ([("p", "List<Int>")], ("lit", True)),
        ([("p", "(U8,Bool)")], ("lit", True)),
        ([("p", "Bool")], ("some", ["lit", True])),
        ([("p", "Bool")], ("rotl", ["lit", {"type": "U8", "value": "1"}], ["lit", {"type": "U8", "value": "1"}])),
    ],
)
def test_an_uncalled_model_that_needs_a_missing_level_is_not_compiled(params, body):
    assert C.derive_ir(uncalled(params, body)) == {"type": "Bool", "value": True, "calls": 0}


def test_an_uncalled_model_that_needs_the_level_it_has_is_compiled():
    case = uncalled([("p", "Option<U8>")], levels=["ir.core", "ir.data"])
    assert C.derive_ir(case) == {"type": "Bool", "value": True, "calls": 0}
    case = uncalled([("p", "Bool")], ("lit", {"type": "U8", "value": "1"}), levels=["ir.core", "ir.data"])
    assert C.derive_ir(case)["type"] == "Bool"


def test_an_uncalled_model_that_needs_no_missing_level_is_still_compiled():
    # a body that does not type is ``ill-typed`` whether or not anything calls it
    assert C.derive_ir(uncalled([("p", "Bool")], ("not", ["lit", {"type": "U8", "value": "1"}]))) == {"error": "ill-typed"}


def test_a_called_model_that_needs_a_missing_level_is_level_missing():
    case = ir_case(["model_call", "m", [["lit", True]]], models={"m": model([("p", "Option<U8>")], ["lit", True])},
                   model_order=["m"])
    assert C.derive_ir(case) == {"error": "level-missing"}


def test_a_model_needs_the_levels_of_the_models_it_calls():
    models = {
        "low": model([("p", "Option<U8>")], ["lit", True]),
        "high": model([("p", "Bool")], ["model_call", "low", [["lit", None]]]),
    }
    case = ir_case(models=models, model_order=["low", "high"])
    assert C.derive_ir(case) == {"type": "Bool", "value": True, "calls": 0}


def test_a_stand_in_of_subjects_is_compiled():
    ops = {"f": {"params": [["p", "Option<U8>"]], "ret": "Bool"}}
    models = {"m": model([("p", "Option<U8>")], ["lit", True])}
    case = ir_case(ops=ops, models=models, model_order=["m"], subjects={"f": "m"})
    assert C.derive_ir(case) == {"error": "level-missing"}


def test_the_tag_name_checks_still_apply_to_a_skipped_model():
    opt = [("p", "Option<U8>")]
    assert C.derive_ir(uncalled(opt, ["bogus"])) == {"error": "unknown-tag"}
    assert C.derive_ir(uncalled(opt, ["subject_op", "f", []], ops={"f": {"params": [], "ret": "Bool"}})) == {
        "error": "subject-forbidden"
    }
    assert C.derive_ir(uncalled(opt, ["model_call", "m", []])) == {"error": "model-order"}
    assert C.derive_ir(uncalled(opt, nest(70))) == {"error": "depth"}
    assert C.derive_ir(uncalled([("p", "Nope")])) == {"error": "ill-typed"}  # its parameters are still read


def chain(n, param="Option<U8>"):
    models = {"m0": model([("p", param)], ["lit", True])}
    order = ["m0"]
    for i in range(1, n):
        models[f"m{i}"] = model([("p", param)], ["model_call", f"m{i - 1}", [["var", "p"]]])
        order.append(f"m{i}")
    return ir_case(models=models, model_order=order)


def test_the_chain_depth_of_a_skipped_model_is_still_checked():
    assert C.derive_ir(chain(64))["type"] == "Bool"
    assert C.derive_ir(chain(65)) == {"error": "depth"}
    assert C.derive_ir(chain(65, "Bool")) == {"error": "depth"}  # compiled models agree


def test_the_derivation_of_a_reached_set_is_unchanged():
    trees, models, ops = C.reach(uncalled([("p", "Option<U8>")]))
    assert models == [] and ops == []
    assert C.reached_levels(uncalled([("p", "Option<U8>")]), trees, models, ops) == {"ir.core"}


# ------------------------------------------------------------------------- R125: the v2-chunk reader


@pytest.fixture(scope="module")
def small_kit():
    here = pathlib.Path(__file__).resolve().parent / "fixtures" / "conformance-v2"
    return C.build_kit((here / "kit-small.warrant").read_bytes(), (here / "kit-small.profile").read_bytes())


def chunk_parts(kit):
    return copy.deepcopy(kit.corpus_part()), copy.deepcopy(kit.row("ragged"))


def refused(corpus, row):
    with pytest.raises(C.Refused) as exc:
        C.check_corpus_and_row(corpus, row, 0)
    assert exc.value.code == "corpus"


def test_the_untouched_corpus_part_is_accepted(small_kit):
    corpus, row = chunk_parts(small_kit)
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17


@pytest.mark.parametrize("text", ["Int", "Nope", "E", "Result<U8,Q>", "U8<", "Bytes<07>", 3])
@pytest.mark.parametrize("where", ["param", "ret"])
def test_an_op_type_the_reader_refuses_is_corpus(small_kit, text, where):
    corpus, row = chunk_parts(small_kit)
    corpus["types"] = {**corpus["types"], "E": {"kind": "error", "cases": ["A", "B"]},
                       "Q": {"kind": "record", "fields": [["a", "U8"]]}}
    op = {"params": [["p", "Bool"]], "ret": "Bool"}
    if where == "param":
        op["params"] = [["p", text]]
    else:
        op["ret"] = text
    corpus["ops"] = {"f": op}
    refused(corpus, row)


@pytest.mark.parametrize("text", ["E", "Nope", "U8<", "Result<U8,Q>"])
def test_a_model_parameter_the_reader_refuses_is_corpus(small_kit, text):
    corpus, row = chunk_parts(small_kit)
    corpus["types"] = {**corpus["types"], "E": {"kind": "error", "cases": ["A", "B"]},
                       "Q": {"kind": "record", "fields": [["a", "U8"]]}}
    corpus["models"] = {"m": model([("p", text)], ["lit", True])}
    corpus["model_order"] = ["m"]
    refused(corpus, row)


@pytest.mark.parametrize("text", ["Int", "Option<Int>", "(Int,Bool)", "List<Int>", "Result<Int,E>"])
def test_an_int_model_parameter_is_accepted(small_kit, text):
    corpus, row = chunk_parts(small_kit)
    corpus["types"] = {**corpus["types"], "E": {"kind": "error", "cases": ["A", "B"]}}
    corpus["models"] = {"m": model([("p", text)], ["lit", True])}
    corpus["model_order"] = ["m"]
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17


def test_a_good_op_with_a_named_width_is_accepted(small_kit):
    corpus, row = chunk_parts(small_kit)
    corpus["ops"] = {"f": {"params": [["p", "Bytes<N>"]], "ret": "Bytes<N>", "width_params": ["N"]}}
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17


def test_the_other_defects_of_ops_models_and_order_are_corpus(small_kit):
    base = {"f": {"params": [["p", "Bool"]], "ret": "Bool"}}
    model_m = {"m": model([("p", "Bool")], ["lit", True])}
    cases = [
        {"ops": []},
        {"ops": None},
        {"models": []},
        {"ops": {"f": 3}},
        {"ops": {"f": {"params": [["p", "Bool"]]}}},
        {"ops": {"f": {"params": 5, "ret": "Bool"}}},
        {"ops": {"f": {"params": [["p", "Bool"]], "ret": "Bool", "width_params": 7}}},
        {"models": {"m": 3}, "model_order": ["m"]},
        {"models": {"m": {"params": [["p", "Bool"]]}}, "model_order": ["m"]},
        {"models": {"m": model([("p", "Bool")], ["bogus"])}, "model_order": ["m"]},
        {"models": {"m": model([("p", "Bool")], ["concat", ["lit", True], ["list"]])}, "model_order": ["m"]},
        {"models": model_m, "model_order": ["missing"]},
        {"models": model_m, "model_order": "m"},
        {"models": model_m, "model_order": [3]},
        {"models": {}, "model_order": ["m"]},
    ]
    for patch in cases:
        corpus, row = chunk_parts(small_kit)
        corpus.update({"ops": base, **patch} if "ops" not in patch else patch)
        refused(corpus, row)


def test_a_model_the_suite_never_calls_is_accepted(small_kit):
    corpus, row = chunk_parts(small_kit)
    corpus["models"] = {"m": model([("p", "Bool")], ["lit", True])}
    corpus["model_order"] = ["m"]
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17


def test_derive_chunk_reports_the_corpus_code_for_a_bad_op(small_kit):
    corpus, row = chunk_parts(small_kit)
    corpus["ops"] = {"f": {"params": [["p", "Int"]], "ret": "Bool"}}
    case = {"corpus": corpus, "suite": row, "chunk_file_hex": small_kit.chunk_file("ragged").hex(), "chunk": 0}
    assert C.derive_chunk(case, small_kit) == {"error": "corpus"}


# ------------------------------------------------------------------------- R126: the fault row's schema


def raises_kit_error(fault, **kw):
    with pytest.raises(C.KitError, match="R126"):
        C.derive_fault(fault_case(fault, **kw))


PRE = {"fault": "p", "op": "echo", "kind": "pre", "requires": ["ir.core"], "rewrites": [{"param": "a", "expr": A}]}


def test_a_well_formed_fault_row_is_untouched():
    assert C.derive_fault(fault_case(POST)) == {"result": "0a"}
    assert C.derive_fault(fault_case(PRE)) == {"args": ["0a"]}
    C.check_fault_row(POST, OP)
    C.check_fault_row(PRE, {"not": "an op row"})  # a malformed op row is ``op_sig``'s to name


@pytest.mark.parametrize(
    "mutate",
    [
        lambda f: f.__setitem__("extra", 1),
        lambda f: f.__setitem__("kind", "swap"),
        lambda f: f.__setitem__("kind", ["post"]),
        lambda f: f.__delitem__("expr"),
        lambda f: f.__delitem__("fault"),
        lambda f: f.__delitem__("op"),
        lambda f: f.__delitem__("requires"),
        lambda f: f.__delitem__("kind"),
        lambda f: f.__setitem__("requires", "ir.core"),
        lambda f: f.__setitem__("requires", [3]),
        lambda f: f.__setitem__("requires", ["ir.bogus"]),
        lambda f: f.__setitem__("op", 3),
        lambda f: f.__setitem__("op", "not a name"),
        lambda f: f.__setitem__("fault", 3),
        lambda f: f.__setitem__("rewrites", []),  # the key of the other kind
    ],
)
def test_a_malformed_post_row_is_a_kit_error(mutate):
    fault = copy.deepcopy(POST)
    mutate(fault)
    raises_kit_error(fault)


@pytest.mark.parametrize(
    "mutate",
    [
        lambda f: f.__setitem__("rewrites", []),
        lambda f: f.__setitem__("rewrites", "x"),
        lambda f: f.__setitem__("rewrites", [3]),
        lambda f: f.__setitem__("rewrites", [{"param": "a"}]),
        lambda f: f.__setitem__("rewrites", [{"param": "a", "expr": A, "x": 1}]),
        lambda f: f.__setitem__("rewrites", [{"param": 3, "expr": A}]),
        lambda f: f.__setitem__("rewrites", [{"param": "zz", "expr": A}]),
        lambda f: f.__setitem__("rewrites", [{"param": "a", "expr": A}, {"param": "a", "expr": A}]),
        lambda f: f.__setitem__("expr", A),
    ],
)
def test_a_malformed_pre_row_is_a_kit_error(mutate):
    fault = copy.deepcopy(PRE)
    mutate(fault)
    raises_kit_error(fault)


def test_a_fault_that_is_not_an_object_is_a_kit_error():
    raises_kit_error([POST])
    raises_kit_error("post")


def test_a_replace_row_is_a_kit_error_without_expr():
    raises_kit_error({"fault": "p", "op": "echo", "kind": "replace", "requires": ["ir.core"]})


def test_the_schema_is_read_before_the_table():
    bad = {"R": {"kind": "record", "fields": [["a", "R"]]}}
    assert C.derive_fault(fault_case(POST, types=bad)) == {"error": "ill-typed"}  # a bad table alone is a vector
    raises_kit_error({**POST, "extra": 1}, types=bad)  # with a malformed row it is a kit error, never a vector
    raises_kit_error({**POST, "requires": ["ir.bogus"]}, types=bad)


def test_every_defined_level_name_is_a_good_requires_name():
    for level in ("ir.core", "ir.fixed", "ir.tuple", "ir.data", "ir.list", "ir.seq"):
        C.check_fault_row({**POST, "requires": [level]}, OP)
