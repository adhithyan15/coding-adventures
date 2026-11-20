"""Slice S10a: ``requires`` inference and the typed IR compiler's cost (evaluation: S10c-1).

* ``ir_levels`` is the sorted union of the levels of every tag of an expression and of the call
  closure of every ``model_call`` (VER04r section 16.10 R9); a tuple *type* adds ``ir.tuple``.
* A ``Program`` types and costs the 35 built-ins (``1 +`` the operands, section 4.2 rule 6).  Until
  slice S10c-1 it refused to *evaluate* them (the default one by level, an ``analyze=True`` one when
  the closure ran); since S10c-1 every ``Program`` evaluates them and ``analyze`` changes nothing.
"""

from __future__ import annotations

import pytest
from conftest import adopter_bytes

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import levels as L
from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.corpus_v2 import derive
from coding_adventures_warrant_oracle.emit import emit
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import ModelDef, Program
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.suites import SuitePlan

GEN = {"name": "test", "version": "0"}


def lower(text: str | bytes):
    return lower_document(parse_source(text))


def lit(ty: str, value: int) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


V = lambda n: ["var", n]  # noqa: E731
U8 = A.Ty("U8")


# -- ir_levels ----------------------------------------------------------------------------


def test_levels_of_core_and_fixed_tags():
    assert L.ir_levels(["add", V("a"), V("b")]) == ["ir.core"]
    assert L.ir_levels(["bit_not", V("a")]) == ["ir.core", "ir.fixed"]
    # a lone static-type built-in has no core tag at all: the union is exactly its level
    assert L.ir_levels(["max_value", "U8"]) == ["ir.fixed"]
    assert L.ir_levels(["tuple", [V("a"), V("b")]]) == ["ir.core", "ir.tuple"]
    assert L.ir_levels(["project", V("a"), 0]) == ["ir.core", "ir.tuple"]
    assert L.ir_levels(["some", V("a")]) == ["ir.core", "ir.data"]
    assert L.ir_levels(["fold", V("a")]) == ["ir.core", "ir.seq"]
    assert L.ir_levels([]) == []


def test_levels_of_a_list_of_trees_is_the_union():
    trees = [["lit", True], ["eq", ["rotl", V("a"), lit("Nat", 1)], V("a")]]
    assert L.ir_levels(trees) == ["ir.core", "ir.fixed"]


def test_levels_include_the_call_closure_once_per_model():
    models = {
        "leaf": ["rotl", V("a"), lit("Nat", 3)],
        "mid": ["model_call", "leaf", [V("a")]],
        "top": ["model_call", "mid", [["model_call", "mid", [V("a")]]]],
        "pure": ["add", V("a"), lit("Int", 1)],
    }
    assert L.ir_levels(["model_call", "pure", [V("x")]], models) == ["ir.core"]
    assert L.ir_levels(["model_call", "top", [V("x")]], models) == ["ir.core", "ir.fixed"]
    # the call is inside an argument of a subject call, and the closure is still followed
    nested = ["subject_op", "f", [["model_call", "leaf", [V("x")]]]]
    assert L.ir_levels(nested, models) == ["ir.core", "ir.fixed"]


def test_levels_refuse_what_they_cannot_classify():
    with pytest.raises(OracleError) as err:
        L.ir_levels(["frobnicate", V("a")])
    assert err.value.code == "ir"
    with pytest.raises(OracleError) as err:
        L.ir_levels(["model_call", "nowhere", []])
    assert err.value.code == "ir"
    # S10t-1: an IR ``match`` is ``ir.tuple`` (the lowerer only lowers a match on a Result)
    assert L.ir_levels(["match", V("a"), [[["wild"], V("b")]]]) == ["ir.core", "ir.tuple"]
    assert L.tag_level("match") is None and L.tag_level("nope") is None


# -- requires on lowered units -------------------------------------------------------------

DOC = """warrant r v1
  op f(a: U32) -> U32
  op g(a: U32 | bit(a, 0)) -> U32
  op h(a: U32) -> U32
  model rot(a: U32) -> U32 = rotl(a, 7)
  model twice(a: U32) -> U32 = rot(rot(a))
  model plain(a: U32) -> U32 = a + 1u32
  model plain2(a: U32) -> U32 = plain(a)
  claim direct:
    forall a: U32. f(a) == bit_not(a)
  claim via_model:
    forall a: U32. f(a) == twice(a)
  claim via_model_core:
    forall a: U32. f(a) == plain2(a)
  claim via_admit:
    forall a: U32 | popcount(a) > 3. f(a) == a
  claim op_with_admit:
    forall a: U32. g(a) == a
  example ex_core: h(1u32) == 2u32
  example ex_fixed: h(1u32) == rotl(1u32, 1)
"""


def test_requires_is_the_sorted_union_over_tags_and_the_model_closure():
    spec = lower(DOC)
    got = {s.claim: s.requires for s in spec.suites}
    both = ("ir.core", "ir.fixed")
    assert got == {
        "direct": both,
        "via_model": both,  # no built-in in the claim itself: the closure of twice -> rot
        "via_model_core": ("ir.core",),
        "via_admit": both,  # only the refinement (admit) uses a built-in
        "op_with_admit": ("ir.core",),  # the *op's* admit is the op's requires, not the claim's
    }
    assert {n: o.requires for n, o in spec.ops.items()} == {
        "f": ("ir.core",),
        "g": both,
        "h": ("ir.core",),
    }
    assert spec.models["rot"].requires == both and spec.models["plain2"].requires == ("ir.core",)
    ex = {e.name: e.requires for e in spec.examples}
    assert ex == {"ex_core": ("ir.core",), "ex_fixed": both}
    # since S10c-1 evaluation refuses no unit for needing ir.fixed
    assert all(s.unsupported is None for s in spec.suites)
    assert spec.ops["g"].unsupported is None and spec.ops["f"].unsupported is None


def test_a_tuple_type_adds_ir_tuple_even_without_a_tuple_tag():
    # R9: the rule is on the type.  The oracle's composite domains already enumerate tuple
    # binders, so the unit is not refused: its requires says what a consumer must implement.
    spec = lower(
        "warrant t v1\n  error E = A\n  op g(t: (Bool, Bool)) -> Bool\n"
        "  op k(a: U8) -> Result<(U8, Nat), E>\n"
        "  op plain(a: U8) -> U8\n"
        "  claim tup:\n    forall t: (Bool, Bool). g(t) == true\n"
        "  claim ret:\n    forall a: U8. k(a) == k(a)\n"
        "  claim none:\n    forall a: U8. plain(a) == a\n"
    )
    assert {s.claim: s.requires for s in spec.suites} == {
        "tup": ("ir.core", "ir.tuple"),
        "ret": ("ir.core", "ir.tuple"),  # the type of the sub-expression k(a)
        "none": ("ir.core",),
    }
    assert spec.ops["g"].requires == ("ir.core", "ir.tuple")  # the op's own parameter
    assert spec.ops["k"].requires == ("ir.core", "ir.tuple")  # the op's own result
    assert spec.ops["plain"].requires == ("ir.core",)
    assert all(s.unsupported is None for s in spec.suites)


def test_tuple_tags_and_match_on_a_result_lower_and_are_evaluable_since_s10t1():
    for body, params in (
        ("(a, a) == (a, a)", "a: U8"),
        ("a.0 == 1", "a: (Nat, Nat)"),
    ):
        spec = lower(f"warrant t v1\n  model m({params}) -> Bool = {body}\n")
        assert spec.models["m"].unsupported is None
        assert spec.models["m"].requires == ("ir.core", "ir.tuple")
    spec = lower(
        "warrant t v1\n  error E = A\n  op k(a: U8) -> Result<U8, E>\n"
        "  claim c:\n    forall a: U8. match k(a) with Ok(x) -> true | Err(E.A) -> false\n"
    )
    assert spec.suites[0].unsupported is None  # match on a Result (D35) is ir.tuple, evaluated
    assert spec.suites[0].requires == ("ir.core", "ir.tuple")
    spec = lower(
        "warrant t v1\n  model m(a: U8) -> Bool = match a with x -> true\n"
    )
    assert spec.models["m"].unsupported.level == "ir.data"  # match on anything else


def test_a_claim_calling_an_unlowerable_model_inherits_its_level():
    spec = lower(
        "warrant t v1\n  op f(a: U8) -> U8\n  model g(a: U8) -> Bool = [a] == [a]\n"
        "  claim c:\n    forall a: U8. g(a) == (f(a) == a)\n"
    )
    assert spec.suites[0].unsupported.level == "ir.list"  # the reserved level (S11a)


# -- the typed compiler: cost ---------------------------------------------------------------


def prog(**models: tuple) -> Program:
    return Program(
        {n: ModelDef(n, p, r, ir) for n, (p, r, ir) in models.items()}, {}, analyze=True
    )


def compiled(ir: list, tenv: dict | None = None, **kw):
    return Program({}, {}, analyze=True, **kw).compile(ir, tenv or {})


@pytest.mark.parametrize(
    "ir,tenv,cost,ty",
    [
        (["bit_and", V("a"), V("b")], {"a": U8, "b": U8}, 3, "U8"),
        (["bit_not", V("a")], {"a": U8}, 2, "U8"),
        (["shl", V("a"), lit("Nat", 3)], {"a": U8}, 3, "U8"),
        (["popcount", V("a")], {"a": U8}, 2, "Nat"),
        (["bit", V("a"), lit("Nat", 1)], {"a": U8}, 3, "Bool"),
        (["max_value", "U64"], {}, 1, "U64"),  # a static type is not an operand
        (["min_value", "I8"], {}, 1, "I8"),
        (["widen", "U16", V("a")], {"a": U8}, 2, "U16"),
        (["wrap", "U8", lit("Int", 300)], {}, 2, "U8"),
        (["to_int", V("a")], {"a": U8}, 2, "Int"),
        (["be_bytes", V("a")], {"a": A.Ty("U32")}, 2, "Bytes<4>"),
        (["from_be", "U32", ["be_bytes", V("a")]], {"a": A.Ty("U32")}, 3, "U32"),
        # unwrap_or evaluates both operands: the cost is the sum, no short circuit
        (
            ["unwrap_or", ["checked_add", V("a"), V("b")], ["add", V("a"), V("b")]],
            {"a": U8, "b": U8},
            1 + (1 + 2) + (1 + 2),
            "U8",
        ),
        (["unwrap_or", ["div", V("n"), V("m")], lit("Int", 0)], {"n": A.Ty("Nat"), "m": A.Ty("Nat")}, 5, "Int"),
        (["imax", V("n"), lit("Int", 1)], {"n": A.Ty("Nat")}, 3, "Int"),
    ],
)
def test_every_new_tag_costs_one_plus_its_operands(ir, tenv, cost, ty):
    c = compiled(ir, tenv)
    assert c.cost == cost and str(c.ty) == ty


def test_cost_follows_the_worst_path_through_if_and_models():
    inner = ["if", ["bit", V("a"), lit("Nat", 0)], ["bit_not", V("a")], V("a")]
    c = compiled(inner, {"a": U8})
    assert c.cost == 1 + 3 + max(2, 1)
    p = prog(m=([("a", U8)], U8, ["rotl", V("a"), lit("Nat", 1)]))
    call = ["model_call", "m", [["bit_not", V("x")]]]
    assert p.compile(call, {"x": U8}).cost == 1 + 2 + 3  # 1 + arguments + the model's body


def test_the_cost_bound_still_applies():
    ir = lit("U8", 0)
    for _ in range(70):
        ir = ["bit_not", ir]
    deep = Program({}, {}, analyze=True)
    with pytest.raises(OracleError) as err:
        deep.compile(ir, {})
    assert err.value.code == "limit"  # nesting
    wide = ["unwrap_or", ["div", V("a"), V("b")], lit("Int", 0)]
    with pytest.raises(OracleError) as err:
        Program({}, {}, analyze=True, max_cost=4).compile(wide, {"a": A.Ty("Nat"), "b": A.Ty("Nat")})
    assert err.value.code == "limit" and "ir-cost" in err.value.message


# -- the typed compiler: shapes and types -----------------------------------------------------


@pytest.mark.parametrize(
    "ir,tenv",
    [
        (["bit_and", V("a")], {"a": U8}),  # arity
        (["bit_and", V("a"), V("a"), V("a")], {"a": U8}),
        (["max_value"], {}),  # missing static type
        (["max_value", "Nat"], {}),
        (["max_value", 8], {}),
        (["max_value", "U8", "U8"], {}),
        (["widen", V("a")], {"a": U8}),
        (["widen", "U8", V("a")], {"a": A.Ty("U16")}),  # range not inside
        (["bit_xor", V("a"), V("b")], {"a": U8, "b": A.Ty("U16")}),
        (["sar", V("a"), lit("Nat", 1)], {"a": U8}),  # sar on unsigned
        (["shl", V("a"), lit("U8", 1)], {"a": U8}),  # count not a Nat
        (["shl", V("a"), V("n")], {"a": U8, "n": A.Ty("Int")}),
        (["popcount", V("n")], {"n": A.Ty("Nat")}),
        (["to_nat", V("a")], {"a": A.Ty("I8")}),
        (["to_int", V("n")], {"n": A.Ty("Nat")}),
        (["from_be", "U16", V("b")], {"b": A.Ty("Bytes", (1,))}),
        (["from_be", "U16", V("b")], {"b": A.Ty("Bytes")}),
        (["unwrap_or", ["div", V("a"), V("b")], lit("U8", 0)], {"a": A.Ty("I8"), "b": A.Ty("I8")}),
        (["unwrap_or", ["div_trunc", V("a"), V("b")], lit("U8", 0)], {"a": U8, "b": U8}),
        (["unwrap_or", ["checked_add", V("a"), V("a")], lit("Int", 0)], {"a": U8}),
        (["unwrap_or", V("a"), lit("U8", 0)], {"a": U8}),  # not an Option
        (["imin", V("a"), V("a")], {"a": U8}),
        (["at", V("n"), lit("Nat", 0)], {"n": A.Ty("Nat")}),
        (["at", V("b"), lit("U8", 0)], {"b": A.Ty("Bytes")}),
        (["wrap", "U8", ["lit", True]], {}),
    ],
)
def test_the_typed_compiler_refuses_ill_typed_ir_fixed(ir, tenv):
    with pytest.raises(OracleError) as err:
        compiled(ir, tenv)
    assert err.value.code == "ir", err.value


def test_the_typed_compiler_enforces_the_option_consumer_rule():
    opt = ["div", V("a"), V("b")]
    tenv = {"a": U8, "b": U8}
    for bad in (
        ["eq", opt, opt],
        ["if", ["lit", True], opt, opt],
        ["and", opt, ["lit", True]],
        ["unwrap_or", V("a"), opt],  # the second operand
        ["unwrap_or", ["div", opt, V("b")], lit("U8", 0)],  # nested producer
        ["unwrap_or", ["if", ["lit", True], opt, opt], lit("U8", 0)],  # not directly a producer
        opt,  # a top-level Option
    ):
        with pytest.raises(OracleError) as err:
            compiled(bad, tenv)
        assert err.value.code == "level" and err.value.level == "ir.data", bad
    ok = compiled(["unwrap_or", opt, lit("U8", 0)], tenv)
    assert str(ok.ty) == "U8"


def test_the_typed_compiler_refuses_an_option_in_models_ops_and_variables():
    option_u8 = A.Ty("Option", (U8,))
    body = ["div", V("a"), V("a")]
    p = prog(m=([("a", U8)], option_u8, body))
    with pytest.raises(OracleError) as err:
        p.model("m")
    assert err.value.level == "ir.data"
    p = prog(m=([("a", option_u8)], U8, lit("U8", 0)))
    with pytest.raises(OracleError) as err:
        p.model("m")
    assert err.value.level == "ir.data"
    p = prog(m=([("a", U8)], U8, ["unwrap_or", ["div", V("a"), V("a")], lit("U8", 0)]))
    assert p.model("m").cost == 1 + 3 + 1
    with pytest.raises(OracleError) as err:
        compiled(V("a"), {"a": option_u8})
    assert err.value.level == "ir.data"
    arg_opt = ["model_call", "m", [body]]
    with pytest.raises(OracleError) as err:
        p.compile(arg_opt, {"a": U8})
    assert err.value.level == "ir.data"


def test_a_malformed_tuple_tag_is_an_ir_error_not_a_level():
    for tag in ("tuple", "project"):
        with pytest.raises(OracleError) as err:
            compiled([tag, V("a")], {"a": U8})
        assert err.value.code == "ir"


# -- ir.fixed is evaluated since S10c-1 -------------------------------------------------------------


def test_every_program_evaluates_every_ir_fixed_tag_and_analyze_changes_nothing():
    for analyze in (False, True):
        c = Program({}, {}, analyze=analyze).compile(["bit_not", V("a")], {"a": U8})
        assert c.fn({"a": 1}) == 254
    # the 35 names are all compiled (a malformed operand list is an ir error, never a level one)
    for tag in sorted(L.FIXED_TAGS):
        with pytest.raises(OracleError) as err:
            Program({}, {}).compile([tag, ["lit", True]], {})
        assert err.value.code == "ir", tag


FIXED_DOC = (
    "warrant e v1\n  op f(a: U8) -> U8\n  model nb(a: U8) -> U8 = bit_not(a)\n"
    "  claim live:\n    forall a: U8. f(a) == a + 1u8\n"
    "  claim fixed:\n    forall a: U8. f(a) == nb(a)\n"
)


def test_emit_enumerates_the_ir_fixed_suites():
    tree = emit(FIXED_DOC.encode())
    assert tree["suites"]["live"]["status"] == "ok"
    assert tree["suites"]["fixed"]["status"] == "ok"
    assert tree["suites"]["fixed"]["cases"] == 256
    assert tree["summary"]["not_live_by_level"] == {}
    assert tree["levels_implemented"] == ["ir.core", "ir.fixed", "ir.tuple"]


def test_a_suite_plan_and_derive_accept_an_ir_fixed_document():
    spec = lower(FIXED_DOC)
    prog = build_program(spec)
    assert "nb" in prog.models  # the default program holds it now: it can evaluate it
    plan = SuitePlan(spec, prog, spec.suites[1])
    assert plan.product == 256
    out = derive(FIXED_DOC.encode(), b"", spec_ref="s.warrant", generator=GEN)
    assert out.corpus["levels"] == ["ir.core", "ir.fixed"]
    assert [s["requires"] for s in out.corpus["suites"]] == [["ir.core"], ["ir.core", "ir.fixed"]]


def test_derive_writes_computed_requires_that_equal_the_constant_for_core_documents():
    doc = (
        "warrant d v1\n  op f(a: U8) -> U8\n"
        "  claim c:\n    forall a: U8. f(a) == a + 1u8\n"
    )
    out = derive(doc.encode(), b"", spec_ref="s.warrant", generator=GEN)
    assert out.corpus["suites"][0]["requires"] == ["ir.core"]
    assert out.corpus["ops"]["f"]["requires"] == ["ir.core"]
    assert out.corpus["levels"] == ["ir.core"]


def test_derive_counts_a_tuple_type_in_requires_and_levels():
    doc = (
        "warrant d v1\n  op g(t: (Bool, Bool)) -> Bool\n"
        "  claim c:\n    forall t: (Bool, Bool). g(t) == true\n"
    )
    out = derive(doc.encode(), b"", spec_ref="s.warrant", generator=GEN)
    assert out.corpus["suites"][0]["requires"] == ["ir.core", "ir.tuple"]
    assert out.corpus["ops"]["g"]["requires"] == ["ir.core", "ir.tuple"]
    assert out.corpus["levels"] == ["ir.core", "ir.tuple"]


# -- the two typers agree on the adopters ---------------------------------------------------------


@pytest.mark.parametrize(
    "name", ["lzw", "x25519", "hash-functions", "logic-gates", "arithmetic", "wasm-leb128"]
)
def test_lowerer_and_typed_compiler_agree_on_every_lowered_unit_of_the_adopters(name):
    """The source typer (lower.py) and the IR typer (ir_eval.py) are two implementations of
    Appendix A.1: every model and claim the lowerer accepted type-checks in the compiler, and
    the compiler's result type is assignable to the declared one."""
    spec = lower(adopter_bytes(name) if name != "hash-functions" else _hash_functions())
    prog = build_program(spec, analyze=True, analyze_data=True, max_cost=65_536)
    checked = 0
    for n, m in spec.models.items():
        if m.ir is None:
            continue
        prog.model(n)
        checked += 1
    for s in spec.suites:
        if not s.requires:
            continue
        tenv = dict(s.binders)
        prog.compile(s.admit, tenv, allow_subject=False)
        prog.compile(s.body, tenv, allow_subject=True)
        checked += 1
    assert checked > 0


def _hash_functions() -> bytes:
    from conftest import SPECS

    return (SPECS / "hash-functions.warrant").read_bytes()


def test_remaining_ir_fixed_refusals_of_the_typed_compiler():
    with pytest.raises(OracleError) as err:  # div of two Bools: neither fixed nor Int/Nat
        compiled(["unwrap_or", ["div", ["lit", True], ["lit", True]], lit("Int", 0)])
    assert err.value.code == "ir"
    with pytest.raises(OracleError) as err:  # a list is the reserved ir.list (R27)
        compiled(
            ["unwrap_or", ["at", V("xs"), lit("Nat", 0)], lit("U8", 0)],
            {"xs": A.Ty("List", (U8,))},
        )
    assert err.value.code == "level" and err.value.level == "ir.list"


def test_an_op_refinement_must_be_bool_and_a_type_error_in_it_is_not_swallowed():
    with pytest.raises(OracleError) as err:
        lower("warrant o v1\n  op f(a: U8 | a + 1u8) -> U8\n")
    assert err.value.code == "type" and "Bool" in err.value.message
    with pytest.raises(OracleError) as err:
        lower("warrant o v1\n  op f(a: U8 | nope) -> U8\n")
    assert err.value.code == "type"
