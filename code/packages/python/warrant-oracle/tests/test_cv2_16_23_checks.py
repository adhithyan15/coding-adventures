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


def nest(n, name="a"):
    """``n`` nested ``concat`` nodes over ``["var", name]``: a tree ``n + 1`` levels deep."""
    leaf = ["var", name]
    expr = leaf
    for _ in range(n):
        expr = ["concat", expr, leaf]
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


def test_the_tag_list_is_the_level_ir_list_never_an_unknown_tag():
    # section 16.16 R57: refused as the level ``ir.list``, so ``level-missing`` where ``levels`` lacks it
    assert C.tag_level_of("list") == "ir.list" and C.tag_level_of("bogus") is None and C.tag_level_of("fold") == "ir.seq"
    assert C.derive_fault(fault_case(post(["list", A]))) == {"error": "level-missing"}
    assert C.derive_fault(fault_case(post(["concat", ["list", A], ["bogus"]]))) == {"error": "level-missing"}
    # a row that states ``ir.list`` is level-missing too, and so is one that omits it
    assert C.derive_fault(fault_case(post(["list", A], requires=["ir.core", "ir.list"]))) == {"error": "level-missing"}
    assert C.derive_ir(ir_case(["list", ["lit", True]])) == {"error": "level-missing"}
    assert C.derive_ir(ir_case(["concat", ["bogus"], ["list"]])) == {"error": "unknown-tag"}
    # an uncalled model body holding ``list`` is accepted by a chunk corpus reader (it needs a level the oracle lacks)
    assert C.known_levels([["list", ["lit", True]]]) == {"ir.core", "ir.list"}
    assert C.static_refusal(ir_case(models={"m": model([], ["list"])}, model_order=["m"], levels=["ir.core"])) is None


def test_the_tag_list_in_an_uncalled_chunk_model_is_accepted(small_kit):
    corpus, row = chunk_parts(small_kit)
    corpus["models"] = {"m": model([("p", "Bool")], ["concat", ["lit", True], ["list"]])}
    corpus["model_order"] = ["m"]
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17


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
    # ``value`` is outside the closed set of ``v2-ir``: the case breaks its own schema, a kit error (R120 (b), R45)
    with pytest.raises(C.KitError, match="env value"):
        C.derive_ir(ir_case(env=[["x", {"type": "U8", "value": "999"}]]))
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


def test_a_stand_in_of_an_unreached_op_counts_for_nothing():
    # R100: never run, so it neither causes ``level-missing`` nor a kit error, and it adds nothing to ``requires``
    ops = {"f": {"params": [["p", "Option<U8>"]], "ret": "Bool"}}
    models = {"m": model([("p", "Option<U8>")], ["lit", True])}
    case = ir_case(ops=ops, models=models, model_order=["m"], subjects={"f": "m"})
    assert C.derive_ir(case) == {"type": "Bool", "value": True, "calls": 0}
    assert C.requires_of("v2-ir", case, {"type": "Bool"}) == ["ir.core"]


def stand_in_case(levels, body):
    ops = {"f": {"params": [["p", "Bool"]], "ret": "Bool"}}
    models = {"m": model([("p", "Bool")], body)}
    return ir_case(["subject_op", "f", [["lit", True]]], ops=ops, models=models, model_order=["m"],
                   subjects={"f": "m"}, levels=levels)


def test_a_stand_in_of_a_reached_op_that_needs_a_missing_level_is_a_kit_error():
    # R100: never ``level-missing``; the kit build fails
    with pytest.raises(C.KitError, match="R100"):
        C.derive_ir(stand_in_case(["ir.core"], ["lit", {"type": "U8", "value": "1"}]) | {"models": {"m": model([("p", "Option<U8>")], ["lit", True])}})
    with pytest.raises(C.KitError, match="R100"):
        C.derive_ir(stand_in_case(["ir.core"], ["not", ["some", ["lit", True]]]))


def test_a_stand_in_of_a_reached_op_counts_for_requires():
    case = stand_in_case(["ir.core", "ir.fixed"], ["bit", ["lit", {"type": "U8", "value": "1"}],
                                                   ["lit", {"type": "Nat", "value": "1"}]])
    assert C.derive_ir(case)["type"] == "Bool"
    assert C.requires_of("v2-ir", case, {"type": "Bool"}) == ["ir.core", "ir.fixed"]
    plain = stand_in_case(["ir.core", "ir.fixed"], ["lit", True])
    assert C.requires_of("v2-ir", plain, {"type": "Bool"}) == ["ir.core"]


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
        {"models": {"m": model([("p", "Bool")], ["subject_op", "f", [["var", "p"]]])}, "model_order": ["m"]},
        {"models": {"m": model([("p", "Bool")], ["model_call", "ghost", []])}, "model_order": ["m"]},
        {"models": {"m": model([("p", "Bool")], ["not", ["lit", {"type": "U8", "value": "1"}]])}, "model_order": ["m"]},
        {"models": {"m": model([("p", "Bytes")], nest(70, "p"))}, "model_order": ["m"]},
        {"models": model_m, "model_order": ["m", "m"]},
        {"models": {**model_m, "n": model([("p", "Bool")], ["lit", True])}, "model_order": ["m"]},
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


def test_a_model_that_needs_a_missing_level_gets_only_the_tier_two_checks_in_a_chunk_corpus(small_kit):
    corpus, row = chunk_parts(small_kit)
    # needs ir.list (a level the oracle lacks): its ill-typed body is not compiled, so it is accepted
    ill = ["not", ["lit", {"type": "U8", "value": "1"}]]
    corpus["models"] = {"m": model([("p", "List<U8>")], ill)}
    corpus["model_order"] = ["m"]
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17
    # ... but the tag-name checks still apply
    corpus["ops"] = {"f": {"params": [["p", "Bool"]], "ret": "Bool"}}
    corpus["models"] = {"m": model([("p", "List<U8>")], ["subject_op", "f", [["lit", True]]])}
    refused(corpus, row)
    corpus["models"] = {"m": model([("p", "List<U8>")], nest(70, "p"))}
    refused(corpus, row)


def test_a_chain_of_65_models_is_corpus_in_a_chunk_corpus(small_kit):
    corpus, row = chunk_parts(small_kit)
    models = {"m0": model([("p", "Bool")], ["lit", True])}
    order = ["m0"]
    for i in range(1, 65):
        models[f"m{i}"] = model([("p", "Bool")], ["model_call", f"m{i - 1}", [["var", "p"]]])
        order.append(f"m{i}")
    corpus["models"], corpus["model_order"] = models, order
    refused(corpus, row)
    del models["m64"]
    order.pop()
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17


# ------------------------------------------------------------------------- N3: the implicit ir.core


def test_an_omitted_ir_core_is_still_needed_by_a_tree_that_holds_a_core_tag():
    fault = post(A, requires=[])
    assert C.derive_fault(fault_case(fault, levels=[])) == {"error": "level-missing"}
    assert C.derive_fault(fault_case(fault, levels=["ir.fixed"])) == {"error": "level-missing"}
    assert C.derive_fault(fault_case(fault, levels=["ir.core"])) == {"result": "0a"}
    pre = {**PRE, "requires": []}
    assert C.derive_fault(fault_case(pre, levels=[])) == {"error": "level-missing"}
    # an unknown tag beside it: level-missing still comes first (order 2 before order 3)
    assert C.derive_fault(fault_case(post(["bogus", A], requires=[]), levels=[])) == {"error": "level-missing"}
    assert C.derive_fault(fault_case(post(["bogus", A], requires=[]))) == {"error": "unknown-tag"}


# ------------------------------------------------------------------------- N5: a malformed op row


MALFORMED_OPS = [{"params": [["a"]], "ret": "Bool"}, {"params": [["a", "Bool", "x"]], "ret": "Bool"},
                 {"params": [[3, "Bool"]], "ret": "Bool"}, {"params": "x", "ret": "Bool"}, {"ret": "Bool"},
                 {"params": []}, 5, [], {"params": [], "ret": "Bool", "width_params": "N"},
                 {"params": [], "ret": "Bool", "width_params": [3]}]


@pytest.mark.parametrize("row", MALFORMED_OPS)
def test_a_malformed_op_row_is_ill_typed_in_v2_ir(row):
    assert C.derive_ir(ir_case(ops={"f": row})) == {"error": "ill-typed"}


@pytest.mark.parametrize("row", MALFORMED_OPS)
def test_a_malformed_op_row_is_a_kit_error_in_v2_fault(row):
    with pytest.raises(C.KitError, match="op row"):
        C.derive_fault(fault_case(POST, op=row))


@pytest.mark.parametrize("row", MALFORMED_OPS)
def test_a_malformed_op_row_is_corpus_in_a_chunk_corpus(small_kit, row):
    corpus, chunk_row = chunk_parts(small_kit)
    corpus["ops"] = {"f": row}
    refused(corpus, chunk_row)


def test_an_in_level_body_is_refused_at_65_deep_and_accepted_at_64(small_kit):
    corpus, row = chunk_parts(small_kit)
    corpus["model_order"] = ["m"]
    corpus["models"] = {"m": model([("p", "Bytes")], nest(63, "p"))}  # 64 levels, ``p`` bound
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17
    corpus["models"] = {"m": model([("p", "Bytes")], nest(64, "p"))}  # 65 levels
    refused(corpus, row)


# ------------------------------------------------------------------------- F1 to F5 of the re-review


def test_a_stand_in_holding_the_tag_list_is_a_kit_error_not_level_missing():
    for body, models in (
        (["list", ["lit", True]], None),
        (["model_call", "low", [["var", "p"]]], {"low": model([("p", "Bool")], ["list", ["lit", True]])}),
    ):
        case = stand_in_case(["ir.core"], body)
        if models:
            case["models"] = {**models, "m": case["models"]["m"]}
            case["model_order"] = ["low", "m"]
        with pytest.raises(C.KitError, match="R100"):
            C.derive_ir(case)
        assert "ir.list" in C.requires_of("v2-ir", case, {"type": "Bool"})  # no bare OracleError


def beyond(body, params=(("p", "Bool"),), **kw):
    return {"models": {"m": model(list(params), body)}, "model_order": ["m"], **kw}


def chunk_with(kit, patch, requires=None):
    corpus, row = chunk_parts(kit)
    corpus.update(patch)
    if requires is not None:
        row["requires"] = requires
    return corpus, row


U8LIT = ["lit", {"type": "U8", "value": "1"}]
BEYOND_BAD = {
    "ill-typed-fixed-body": beyond(["not", ["bit", U8LIT, ["lit", True]]]),
    "option-param-core-body-ill-typed": beyond(["not", U8LIT], params=(("p", "Option<U8>"),)),
    "some-body": beyond(["not", ["some", ["lit", True]]]),
    "calls-an-ill-typed-fixed-model": {
        "models": {"low": model([("p", "Bool")], ["not", ["bit", U8LIT, ["lit", True]]]),
                   "m": model([("p", "Bool")], ["model_call", "low", [["var", "p"]]])},
        "model_order": ["low", "m"]},
    "and-with-fixed-and-unbound": beyond(["and", ["bit", U8LIT, ["lit", {"type": "Nat", "value": "1"}]], ["var", "q"]]),
}


@pytest.mark.parametrize("name", sorted(BEYOND_BAD))
def test_a_model_beyond_the_suite_requires_that_fails_is_a_kit_error(small_kit, name):
    # R101: neither ``corpus`` (a lane without the level accepts) nor a silent accept (a lane with it refuses)
    corpus, row = chunk_with(small_kit, BEYOND_BAD[name], requires=["ir.core"])
    with pytest.raises((C.KitError, C.Refused)) as exc:
        C.check_corpus_and_row(corpus, row, 0)
    assert isinstance(exc.value, C.KitError), (name, exc.value)


def test_a_tuple_model_beyond_the_suite_requires_over_the_cost_bound_is_a_kit_error(small_kit):
    body = ["lit", True]
    for _ in range(12):  # a balanced tree of 2**13 - 1 nodes: over the 2,000 bound, 13 levels deep
        body = ["and", body, body]
    corpus, row = chunk_with(small_kit, beyond(body, params=(("p", "(U8,Bool)"),)), requires=["ir.core"])
    with pytest.raises(C.KitError, match="R101"):
        C.check_corpus_and_row(corpus, row, 0)
    row["requires"] = ["ir.core", "ir.tuple"]
    refused(corpus, row)  # the suite needs tuples: the same model is ``corpus``


def test_the_same_models_are_corpus_when_the_suite_requires_their_level(small_kit):
    corpus, row = chunk_with(small_kit, BEYOND_BAD["ill-typed-fixed-body"], requires=["ir.core", "ir.fixed"])
    refused(corpus, row)
    corpus, row = chunk_with(small_kit, BEYOND_BAD["some-body"], requires=["ir.core", "ir.data"])
    refused(corpus, row)
    corpus, row = chunk_with(small_kit, BEYOND_BAD["option-param-core-body-ill-typed"], requires=["ir.core", "ir.data"])
    refused(corpus, row)


def test_a_good_model_beyond_the_suite_requires_is_accepted(small_kit):
    corpus, row = chunk_with(small_kit, beyond(["eq", ["bit", U8LIT, ["lit", {"type": "Nat", "value": "1"}]], ["lit", True]]),
                             requires=["ir.core"])
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17


def test_an_undeclared_err_case_in_an_uncalled_in_level_model_is_corpus(small_kit):
    body = ["is_err", ["err", "NOPE"]]
    corpus, row = chunk_with(small_kit, beyond(body))
    refused(corpus, row)
    corpus["types"] = {**corpus["types"], "E": {"kind": "error", "cases": ["NOPE", "B"]}}
    assert C.check_corpus_and_row(corpus, row, 0)[0] == 17
    assert C.derive_ir(ir_case(["is_err", ["err", "NOPE"]])) == {"error": "ill-typed"}  # the oracle's v2-ir agrees


def test_core_is_always_needed_by_a_fault_row():
    assert C.derive_fault(fault_case(post(["bogus"], requires=[]), levels=[])) == {"error": "level-missing"}
    fixed = post(["max_value", "U8"], requires=["ir.fixed"])
    case = fault_case(fixed, levels=["ir.fixed"], op={"params": [["a", "U8"]], "ret": "U8"}, args=["1"], result="2")
    assert C.derive_fault(case) == {"error": "level-missing"}
    assert C.derive_fault({**case, "levels": ["ir.core", "ir.fixed"]}) == {"result": "255"}


@pytest.mark.parametrize("row", [{"params": [["a", 3]], "ret": "Bool"}, {"params": [], "ret": 3},
                                 {"params": [], "ret": None}])
def test_a_non_text_op_type_is_a_kit_error_in_v2_fault(row):
    with pytest.raises(C.KitError, match="op row"):
        C.derive_fault(fault_case(POST, op=row))
    assert C.derive_ir(ir_case(ops={"f": row})) == {"error": "ill-typed"}
