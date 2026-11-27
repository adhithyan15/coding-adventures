"""Parser: golden tests over every adopter's ``.warrant`` file, WX-G syntax, hostile input."""

from __future__ import annotations

from collections import Counter

import pytest
from conftest import ADOPTERS, adopter_bytes

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.parser import MAX_NESTING, parse_source

# (module, document version, declarations, declaration census).  Written down from
# a first reading and then cross-checked against a line scan of each file below.
GOLDEN = {
    "ct-compare": (
        "ct_compare",
        "v0",
        47,
        {
            "AssumeDecl": 2,
            "BoundDecl": 4,
            "ClaimDecl": 9,
            "ErrorDecl": 1,
            "ExampleDecl": 12,
            "InputDecl": 9,
            "LabelsDecl": 4,
            "ModelDecl": 2,
            "OpDecl": 4,
        },
    ),
    "gf256": (
        "gf256",
        "v0",
        128,
        {
            "AssumeDecl": 2,
            "BoundDecl": 6,
            "ClaimDecl": 38,
            "ErrorDecl": 1,
            "ExampleDecl": 35,
            "InputDecl": 11,
            "LabelsDecl": 6,
            "ModelDecl": 23,
            "OpDecl": 6,
        },
    ),
    "wasm-leb128": (
        "wasm_leb128",
        "v1",
        165,
        {
            "AssumeDecl": 2,
            "BoundDecl": 6,
            "ClaimDecl": 44,
            "ErrorDecl": 1,
            "ExampleDecl": 39,
            "InputDecl": 12,
            "LabelsDecl": 6,
            "ModelDecl": 49,
            "OpDecl": 6,
        },
    ),
    "logic-gates": (
        "logic_gates",
        "v1",
        340,
        {
            "AssumeDecl": 2,
            "BoundDecl": 22,
            "ClaimDecl": 88,
            "ErrorDecl": 1,
            "ExampleDecl": 83,
            "InputDecl": 41,
            "LabelsDecl": 22,
            "ModelDecl": 59,
            "OpDecl": 22,
        },
    ),
    "lzw": (
        "lzw",
        "v1",
        89,
        {
            "AssumeDecl": 2,
            "BoundDecl": 2,
            "ClaimDecl": 21,
            "ErrorDecl": 1,
            "ExampleDecl": 27,
            "InputDecl": 2,
            "LabelsDecl": 2,
            "ModelDecl": 30,
            "OpDecl": 2,
        },
    ),
    "arithmetic": (
        "arithmetic",
        "v1",
        219,  # 218 before VER04r D76 split the 9-binder claim in two
        {
            "AssumeDecl": 2,
            "BoundDecl": 5,
            "ClaimDecl": 64,
            "ErrorDecl": 1,
            "ExampleDecl": 53,
            "InputDecl": 12,
            "LabelsDecl": 5,
            "ModelDecl": 69,
            "OpDecl": 5,
            "RecordDecl": 2,
            "VariantDecl": 1,
        },
    ),
    "x25519": (
        "x25519",
        "v1",
        26,
        {
            "ClaimDecl": 8,
            "ErrorDecl": 1,
            "ExampleDecl": 9,
            "LabelsDecl": 3,
            "ModelDecl": 2,
            "OpDecl": 3,
        },
    ),
}
KEYWORD = {
    "claim": "ClaimDecl",
    "example": "ExampleDecl",
    "model": "ModelDecl",
    "op": "OpDecl",
    "error": "ErrorDecl",
    "labels": "LabelsDecl",
    "input": "InputDecl",
    "bound": "BoundDecl",
    "assume": "AssumeDecl",
    "record": "RecordDecl",
    "variant": "VariantDecl",
}


def line_census(raw: bytes) -> Counter:
    """Count declarations by a naive line scan: an independent cross-check."""
    out: Counter = Counter()
    for line in raw.decode().splitlines():
        if line.startswith("  ") and not line.startswith("   "):
            word = line.split()[0] if line.split() else ""
            if word in KEYWORD:
                out[KEYWORD[word]] += 1
    return out


def test_golden_table_covers_every_adopter():
    assert sorted(GOLDEN) == sorted(ADOPTERS)


@pytest.mark.parametrize("name", ADOPTERS)
def test_adopter_parses_with_the_golden_shape(name):
    raw = adopter_bytes(name)
    doc = parse_source(raw)
    module, version, count, census = GOLDEN[name]
    assert (doc.name, doc.version, len(doc.decls)) == (module, version, count)
    assert dict(Counter(type(d).__name__ for d in doc.decls)) == census
    assert Counter(type(d).__name__ for d in doc.decls) == line_census(raw)


def test_ct_compare_signatures_and_a_claim_in_detail():
    doc = parse_source(adopter_bytes("ct-compare"))
    ops = {d.name: d for d in doc.decls if isinstance(d, A.OpDecl)}
    assert ops["eq_fixed"].statics == ("N",)
    assert str(ops["eq_fixed"].params[0].ty) == "Bytes<N>"
    assert str(ops["select_bytes"].ret) == "Result<Bytes,SelectError>"
    assert ops["select_bytes"].fresh and not ops["eq"].fresh
    claims = {d.name: d for d in doc.decls if isinstance(d, A.ClaimDecl)}
    body = claims["select_picks"].body
    assert isinstance(body, A.Quant) and body.kind == "forall"
    assert [b.name for b in body.binders] == ["a", "b", "choice"]
    assert body.binders[1].refinement is not None  # b: Bytes | len(a) == len(b)
    assert isinstance(body.body, A.Binary) and body.body.op == "=="


def test_input_rejects_and_ensures_rows():
    doc = parse_source(adopter_bytes("ct-compare"))
    inputs = [d for d in doc.decls if isinstance(d, A.InputDecl) and d.rejects]
    assert {(d.op, d.param) for d in inputs} == {("select_bytes", "a"), ("select_bytes", "b")}
    assert inputs[0].rejects[0][1] == ("SelectError", "CT_SELECT_LENGTH_MISMATCH")
    assert inputs[0].ensures == ("no_panic", "terminates")


def test_v1_constructs_in_arithmetic():
    doc = parse_source(adopter_bytes("arithmetic"))
    kinds = {type(d).__name__ for d in doc.decls}
    assert {"RecordDecl", "VariantDecl"} <= kinds
    models = {d.name: d for d in doc.decls if isinstance(d, A.ModelDecl)}
    assert isinstance(models["is_not"].body, A.Match)
    arms = models["is_not"].body.arms
    # a bare capitalised name is a "bind" pattern: only the checker knows it is a constructor
    assert arms[0][0].kind == "bind" and arms[0][0].name == ("Not",)
    assert arms[1][0].kind == "wild"
    variant = next(d for d in doc.decls if isinstance(d, A.VariantDecl))
    assert [c for c, _ in variant.ctors] == ["Add", "Sub", "And", "Or", "Xor", "Not"]


def test_tuples_projection_and_static_builtins_in_wasm():
    doc = parse_source(adopter_bytes("wasm-leb128"))
    models = {d.name: d for d in doc.decls if isinstance(d, A.ModelDecl)}
    assert str(models["du1"].ret) == "Result<((U64,Nat)),Leb128Error>".replace("((", "(").replace(
        "))", ")"
    )
    low7 = models["low7"].body
    assert isinstance(low7, A.Call) and low7.name == ("widen",) and low7.statics == ("U64",)
    mv = models["sx"].body
    assert isinstance(mv, A.Call) and mv.statics == ("I64",)


def test_precedence_and_associativity():
    doc = parse_source(
        b"warrant p\n  model m(a: Nat, b: Nat) -> Bool = a + b * 2 == 3 and not true or false => true\n"
    )
    body = doc.decls[0].body
    # => binds loosest, then or, then and, then ==, then +, then *
    assert body.op == "=>"
    assert body.left.op == "or"
    assert body.left.left.op == "and"
    eq = body.left.left.left
    assert eq.op == "==" and eq.left.op == "+" and eq.left.right.op == "*"
    assert isinstance(body.left.left.right, A.Unary)


def test_implication_is_right_associative():
    doc = parse_source(b"warrant p\n  model m(a: Bool) -> Bool = a => a => a\n")
    body = doc.decls[0].body
    assert body.op == "=>" and body.right.op == "=>"


def test_comparisons_do_not_chain():
    with pytest.raises(OracleError) as err:
        parse_source(b"warrant p\n  model m(a: Nat) -> Bool = 1 < a < 3\n")
    assert err.value.code == "parse"


def test_subtraction_of_a_negative():
    doc = parse_source(b"warrant p\n  model m(a: Int) -> Int = a - -1\n")
    body = doc.decls[0].body
    assert body.op == "-" and isinstance(body.right, A.Unary) and body.right.op == "neg"


def test_static_application_versus_comparison():
    # ``f<2>(x)`` is an application only for a callable with static parameters.
    doc = parse_source(
        b"warrant p\n  op f<N: Nat>(a: Bytes<N>) -> Bool\n  model m(a: Nat, b: Nat) -> Bool = a < b\n"
    )
    assert isinstance(doc.decls[1].body, A.Binary)
    doc = parse_source(
        b'warrant p\n  op f<N: Nat>(a: Bytes<N>) -> Bool\n  example e:\n    f<2>(b"0000") == true\n'
    )
    call = doc.decls[1].body.left
    assert isinstance(call, A.Call) and call.statics == (2,)


def test_example_body_may_be_inline():
    doc = parse_source(b"warrant p\n  example e: 1 == 1\n")
    assert isinstance(doc.decls[0], A.ExampleDecl)


def test_lists_if_and_alias():
    doc = parse_source(
        b"warrant p\n  model Word = Bytes\n  model m(a: Nat) -> List<Nat> = if a == 0 then [] else [a, a]\n"
    )
    assert isinstance(doc.decls[0], A.AliasDecl)
    body = doc.decls[1].body
    assert isinstance(body, A.If) and body.then == A.ListLit(())


def test_tuple_type_and_literal():
    doc = parse_source(b"warrant p v1\n  model m(a: Nat) -> (Nat, Bool) = (a, true)\n")
    assert str(doc.decls[0].ret) == "(Nat,Bool)"
    assert isinstance(doc.decls[0].body, A.TupleLit)


def test_postfix_projection_forms():
    doc = parse_source(
        b"warrant p v1\n  model m(t: (Nat, Bool)) -> Nat = t.0\n  model n(a: Nat) -> Nat = f(a).1\n"
    )
    assert doc.decls[0].body == A.Name(("t", "0"))
    assert doc.decls[1].body == A.Proj(A.Call(("f",), (), (A.Name(("a",)),)), "1")


def test_match_patterns():
    doc = parse_source(
        b"warrant p v1\n  model m(o: Option<Nat>) -> Nat = match o with Some((x, y)) -> x | None -> 0 | _ -> 1\n"
    )
    arms = doc.decls[0].body.arms
    assert arms[0][0].kind == "ctor" and arms[0][0].items[0].kind == "tuple"
    assert arms[1][0].kind == "bind"  # a bare name: the checker decides ctor vs binder
    assert arms[2][0].kind == "wild"


def test_bool_pattern_and_qualified_pattern():
    doc = parse_source(
        b"warrant p v1\n  model m(b: Bool) -> Nat = match b with true -> 1 | false -> 0\n"
    )
    assert [a[0].kind for a in doc.decls[0].body.arms] == ["bool", "bool"]
    doc = parse_source(
        b"warrant p v1\n  model m(r: Result<Nat, E>) -> Nat = match r with Ok(x) -> x | Err(E.BAD) -> 0\n  error E = BAD\n"
    )
    assert doc.decls[0].body.arms[1][0].items[0].name == ("E", "BAD")


# -- WX-G: inout parameters and ensures blocks (VER04k section 4) -----------------

ZEROIZE_LIKE = b"""warrant zeroize v1
  model zeros(n: Nat) -> Bytes = b""
  op zeroize_bytes(buf: inout Bytes) -> Bytes
  op zeroize_u64(x: inout U64) -> U64
  ensures zeroize_bytes:
    len(result) == len(buf)
    result == zeros(len(buf))
  ensures zeroize_u64:
    result == 0u64
  claim wipes:
    forall buf: Bytes. zeroize_bytes(buf) == zeros(len(buf))
"""


def test_wx_g_inout_and_ensures_parse():
    doc = parse_source(ZEROIZE_LIKE)
    ops = [d for d in doc.decls if isinstance(d, A.OpDecl)]
    assert all(p.inout for o in ops for p in o.params)
    ens = [d for d in doc.decls if isinstance(d, A.EnsuresDecl)]
    assert [(e.op, len(e.rows)) for e in ens] == [("zeroize_bytes", 2), ("zeroize_u64", 1)]


def test_zeroize_spec_parses_when_present(specs_dir):
    path = specs_dir / "zeroize.warrant"
    if not path.exists():  # lands with WX-G (PR 19369)
        pytest.skip("zeroize.warrant is not on this branch yet")
    doc = parse_source(path.read_bytes())
    assert doc.name == "zeroize" and any(isinstance(d, A.EnsuresDecl) for d in doc.decls)


def test_inout_is_a_name_elsewhere():
    doc = parse_source(b"warrant p\n  model m(inout: Nat) -> Nat = inout\n")
    assert doc.decls[0].params[0].name == "inout"


# -- malformed and hostile source -------------------------------------------------


@pytest.mark.parametrize(
    "text",
    [
        "",  # nothing
        "warrant\n",  # no name
        "model m\n",  # not a warrant file
        "warrant p v2\n  error E = A\n",  # only v1 is a version marker
        "warrant p\n",  # no declarations
        "warrant p\n  record R(a: Nat)\n",  # v1 construct in a v0 document
        "warrant p\n  variant V = A | B\n",
        "warrant p\n  ensures f:\n    true\n",
        "warrant p\n  frobnicate x\n",  # unknown declaration
        "warrant p\n  error E =\n",
        "warrant p\n  model m() -> Bool\n",  # no body
        "warrant p\n  model m(a) -> Bool = true\n",  # parameter without a type
        "warrant p\n  model m(a: Nat) -> Bool = a +\n",
        "warrant p\n  model m(a: Nat) -> Bool = (a\n",
        "warrant p\n  model m(a: Nat) -> Bool = if a then 1\n",
        "warrant p\n  model m(a: Nat) -> Bool = match a with\n",
        "warrant p\n  op f(a: Nat) -> Bool fresh fresh\n",
        "warrant p\n  op f<N: Int>(a: Nat) -> Bool\n",
        "warrant p\n  claim c:\n    forall a: Nat a == a\n",  # missing separator dot
        "warrant p\n  assume platform:\n",  # empty block
        "warrant p\n  assume sandbox:\n    none\n",
        "warrant p\n  labels f:\n    a open\n",
        "warrant p\n  input f.a:\n    ensures x\n",  # missing hostile
        "warrant p\n  input f:\n    hostile\n    ensures x\n",  # not a qualified name
        "warrant p\n  bound f:\n    steps < 3\n",
        "warrant p\n  model m(a: (Nat)) -> Bool = true\n",  # 1-tuple type
        "warrant p\n  model m(a: Bytes<1,2>) -> Bool = true\n",
        "warrant p\n  model m(a: Bytes<8u8>) -> Bool = true\n",
        "warrant p\n  model m(a: Nat) -> Bool = then\n",  # reserved word as expression
        "warrant p\n  model m(a: Nat) -> Bool = a.\n",
        "warrant p\n  model m(a: Nat) -> Bool = f<1u8>(a)\n",
    ],
)
def test_malformed_documents_are_parse_errors(text):
    with pytest.raises(OracleError) as err:
        parse_source(text)
    assert err.value.code in ("parse", "lex")


def test_malformed_v1_forms():
    for text in [
        "warrant p v1\n  record R()\n",
        "warrant p v1\n  variant V = A\n",  # one constructor is not a sum... parse accepts; checker rules
    ]:
        try:
            parse_source(text)
        except OracleError as err:
            assert err.code == "parse"
    with pytest.raises(OracleError):
        parse_source("warrant p v1\n  model m(a: Nat) -> Nat = match a with (x) -> x\n")
    with pytest.raises(OracleError):
        parse_source("warrant p v1\n  model m(t: (Nat, Nat)) -> Nat = match t with if -> 1\n")
    with pytest.raises(OracleError):
        parse_source("warrant p v1\n  model m(a: Nat) -> Nat = match a with x\n")


def test_deep_parenthesis_nesting_is_a_limit_error():
    deep = "(" * (MAX_NESTING + 8) + "1" + ")" * (MAX_NESTING + 8)
    with pytest.raises(OracleError) as err:
        parse_source(f"warrant p\n  model m() -> Nat = {deep}\n")
    assert err.value.code == "limit"


def test_deep_unary_chain_is_a_limit_error():
    with pytest.raises(OracleError) as err:
        parse_source("warrant p\n  model m(a: Bool) -> Bool = " + "not " * 200 + "a\n")
    assert err.value.code == "limit"
    with pytest.raises(OracleError) as err:
        parse_source("warrant p\n  model m(a: Int) -> Int = " + "- " * 200 + "a\n")
    assert err.value.code == "limit"


def test_deep_implication_and_type_nesting_are_limit_errors():
    with pytest.raises(OracleError) as err:
        parse_source("warrant p\n  model m(a: Bool) -> Bool = " + "a => " * 300 + "a\n")
    assert err.value.code == "limit"
    nested = "List<" * 70 + "Nat" + ">" * 70
    with pytest.raises(OracleError) as err:
        parse_source(f"warrant p\n  model m(a: {nested}) -> Bool = true\n")
    assert err.value.code == "limit"


def test_nesting_just_under_the_limit_parses():
    deep = "(" * 30 + "1" + ")" * 30
    parse_source(f"warrant p\n  model m() -> Nat = {deep}\n")


def test_too_many_declarations():
    body = "".join(f"  error E{i} = A\n" for i in range(4097))
    with pytest.raises(OracleError) as err:
        parse_source("warrant p\n" + body)
    assert err.value.code == "limit"


def test_oversized_source_is_refused_before_lexing():
    with pytest.raises(OracleError) as err:
        parse_source("warrant p\n" + "-- " + "x" * (1 << 20) + "\n")
    assert err.value.code == "source"
    with pytest.raises(OracleError):
        parse_source(b"\xff")


def test_tuple_arity_and_match_arm_limits():
    with pytest.raises(OracleError):
        parse_source("warrant p v1\n  model m(a: Nat) -> Nat = (" + ",".join(["a"] * 9) + ")\n")
    with pytest.raises(OracleError):
        parse_source("warrant p v1\n  model m(a: (" + ",".join(["Nat"] * 9) + ")) -> Nat = 1\n")
    arms = " | ".join(f"x{i} -> {i}" for i in range(257))
    with pytest.raises(OracleError) as err:
        parse_source(f"warrant p v1\n  model m(a: Nat) -> Nat = match a with {arms}\n")
    assert err.value.code == "limit"
    deep_pat = "(" * 10 + "x, y" + ", y)" * 10
    with pytest.raises(OracleError) as err:
        parse_source(f"warrant p v1\n  model m(a: Nat) -> Nat = match a with {deep_pat} -> 1\n")
    assert err.value.code == "limit"


def test_every_failure_message_omits_source_text():
    secret = "SUPERSECRETTOKEN"
    with pytest.raises(OracleError) as err:
        parse_source(f"warrant p\n  frobnicate {secret}\n")
    assert secret not in str(err.value)
