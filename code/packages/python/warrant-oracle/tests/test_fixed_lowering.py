"""Slice S10a: lowering, typing and refusals of the 35 ``ir.fixed`` built-ins (VER04r A.1).

Every built-in is lowered to the exact JSON shape of Appendix A.1, typed from its operands, and
the unit that uses it is evaluable since slice S10c-1 (it was marked ``unsupported`` at the level
``ir.fixed`` in S10a, when evaluation still refused).  ``Option<T>`` was an IR-only intermediate whose only
consumer is ``unwrap_or`` (section 16.10 D23, R8); since slice S11a it is a real type of ``ir.data`` and what
this file pins is the **intermediate use** that stays ``ir.fixed`` (R29).  ``div``/``rem`` on two ``Nat``
operands give ``Option<Int>`` (R7).
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle import levels as L
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import FIXED, lower_document
from coding_adventures_warrant_oracle.parser import parse_source

TYPES = list(FIXED)  # U8 U16 U32 U64 I8 I16 I32 I64
UNSIGNED = [t for t in TYPES if t.startswith("U")]
SIGNED = [t for t in TYPES if t.startswith("I")]


def lit(ty: str, value: int) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


def lower(text: str):
    return lower_document(parse_source(text))


def model(body: str, params: str, ret: str, prelude: str = ""):
    """The lowered model ``m`` of a one-model v1 document."""
    return lower(f"warrant p v1\n{prelude}  model m({params}) -> {ret} = {body}\n").models["m"]


def lowered_ir(body: str, params: str, ret: str):
    m = model(body, params, ret)
    # ir.fixed is evaluated since S10c-1: the unit is not refused, and its IR is there to be dumped.
    assert m.unsupported is None, m.unsupported
    return m.ir


def refused_type(body: str, params: str, ret: str) -> OracleError:
    with pytest.raises(OracleError) as err:
        model(body, params, ret)
    assert err.value.code == "type", err.value
    return err.value


def refused_level(body: str, params: str, ret: str, level: str) -> OracleError:
    m = model(body, params, ret)
    assert m.unsupported is not None and m.unsupported.level == level, m.unsupported
    assert m.ir is None  # not lowerable: there is no IR to dump
    return m.unsupported


# -- the 35 names ---------------------------------------------------------------------


def test_the_level_table_names_exactly_the_35_built_ins_of_appendix_a1():
    assert len(L.FIXED_TAGS) == 35
    assert "try" in L.FIXED_TAGS and "try_nat" not in L.FIXED_TAGS
    assert L.OPTION_PRODUCERS <= L.FIXED_TAGS and len(L.OPTION_PRODUCERS) == 9
    assert set(L.FIXED_ARITY) == L.FIXED_TAGS
    assert L.STATIC_TYPE_TAGS <= L.FIXED_TAGS
    assert not (L.FIXED_TAGS & L.CORE_TAGS)  # core and fixed never overlap


@pytest.mark.parametrize("t", TYPES)
def test_binary_bit_operations_keep_the_type(t):
    for tag in ("bit_and", "bit_or", "bit_xor"):
        assert lowered_ir(f"{tag}(a, b)", f"a: {t}, b: {t}", t) == [tag, ["var", "a"], ["var", "b"]]


@pytest.mark.parametrize("t", TYPES)
def test_shifts_rotates_and_bit_take_a_nat_count(t):
    for tag in ("shl", "shr", "rotl", "rotr"):
        assert lowered_ir(f"{tag}(a, n)", f"a: {t}, n: Nat", t) == [tag, ["var", "a"], ["var", "n"]]
    assert lowered_ir("bit(a, n)", f"a: {t}, n: Nat", "Bool") == ["bit", ["var", "a"], ["var", "n"]]
    # a bare decimal count is a Nat in the count position
    assert lowered_ir("shl(a, 3)", f"a: {t}", t) == ["shl", ["var", "a"], lit("Nat", 3)]
    assert lowered_ir("bit(a, 0)", f"a: {t}", "Bool") == ["bit", ["var", "a"], lit("Nat", 0)]


@pytest.mark.parametrize("t", SIGNED)
def test_sar_is_for_signed_types(t):
    assert lowered_ir("sar(a, n)", f"a: {t}, n: Nat", t) == ["sar", ["var", "a"], ["var", "n"]]


@pytest.mark.parametrize("t", TYPES)
def test_bit_not_and_the_counts(t):
    assert lowered_ir("bit_not(a)", f"a: {t}", t) == ["bit_not", ["var", "a"]]
    for tag in ("popcount", "clz", "ctz"):
        assert lowered_ir(f"{tag}(a)", f"a: {t}", "Nat") == [tag, ["var", "a"]]


@pytest.mark.parametrize("t", TYPES)
def test_bounds_are_not_folded_to_a_literal(t):
    # unoptimised lowering (section 10.3): the static type is the canonical name, position 1.
    assert lowered_ir(f"max_value<{t}>()", "a: Bool", t) == ["max_value", t]
    assert lowered_ir(f"min_value<{t}>()", "a: Bool", t) == ["min_value", t]


@pytest.mark.parametrize("src", TYPES)
@pytest.mark.parametrize("dst", TYPES)
def test_conversions_for_every_source_and_target(src, dst):
    bits = lambda t: int(t[1:])  # noqa: E731
    lo = lambda t: -(1 << (bits(t) - 1)) if t[0] == "I" else 0  # noqa: E731
    hi = lambda t: (1 << (bits(t) - 1)) - 1 if t[0] == "I" else (1 << bits(t)) - 1  # noqa: E731
    wraps = ["wrap", dst, ["var", "a"]]
    assert lowered_ir(f"wrap<{dst}>(a)", f"a: {src}", dst) == wraps
    expr = f"unwrap_or(try<{dst}>(a), {'0' + dst.lower()})"
    assert lowered_ir(expr, f"a: {src}", dst)[:2] == ["unwrap_or", ["try", dst, ["var", "a"]]]
    if lo(dst) <= lo(src) and hi(src) <= hi(dst):
        assert lowered_ir(f"widen<{dst}>(a)", f"a: {src}", dst) == ["widen", dst, ["var", "a"]]
    else:
        err = refused_type(f"widen<{dst}>(a)", f"a: {src}", dst)
        assert "widen" in err.message


@pytest.mark.parametrize("src", ["Int", "Nat"])
@pytest.mark.parametrize("dst", TYPES)
def test_wrap_and_try_accept_int_and_nat(src, dst):
    assert lowered_ir(f"wrap<{dst}>(a)", f"a: {src}", dst) == ["wrap", dst, ["var", "a"]]
    assert lowered_ir(f"unwrap_or(try<{dst}>(a), 0{dst.lower()})", f"a: {src}", dst)[1] == [
        "try",
        dst,
        ["var", "a"],
    ]
    # ``wrap<U8>(-1)``: the operand is Int
    assert lowered_ir(f"wrap<{dst}>(-1)", "a: Bool", dst)[2][0] == "neg"


@pytest.mark.parametrize("t", TYPES)
def test_to_int_for_every_fixed_type_and_to_nat_for_unsigned_ones(t):
    assert lowered_ir("to_int(a)", f"a: {t}", "Int") == ["to_int", ["var", "a"]]
    if t in UNSIGNED:
        assert lowered_ir("to_nat(a)", f"a: {t}", "Nat") == ["to_nat", ["var", "a"]]
    else:
        refused_type("to_nat(a)", f"a: {t}", "Nat")


@pytest.mark.parametrize("t", TYPES)
def test_checked_arithmetic_gives_an_option_consumed_by_unwrap_or(t):
    for tag in ("checked_add", "checked_sub", "checked_mul"):
        ir = lowered_ir(f"unwrap_or({tag}(a, b), a)", f"a: {t}, b: {t}", t)
        assert ir == ["unwrap_or", [tag, ["var", "a"], ["var", "b"]], ["var", "a"]]


@pytest.mark.parametrize("t", UNSIGNED)
def test_div_and_rem_on_unsigned_fixed_widths(t):
    for tag in ("div", "rem"):
        ir = lowered_ir(f"unwrap_or({tag}(a, b), a)", f"a: {t}, b: {t}", t)
        assert ir == ["unwrap_or", [tag, ["var", "a"], ["var", "b"]], ["var", "a"]]
    for tag in ("div_trunc", "rem_trunc"):
        refused_type(f"unwrap_or({tag}(a, b), a)", f"a: {t}, b: {t}", t)


@pytest.mark.parametrize("t", SIGNED)
def test_signed_div_and_rem_must_say_trunc(t):
    for tag in ("div_trunc", "rem_trunc"):
        ir = lowered_ir(f"unwrap_or({tag}(a, b), a)", f"a: {t}, b: {t}", t)
        assert ir == ["unwrap_or", [tag, ["var", "a"], ["var", "b"]], ["var", "a"]]
    for tag in ("div", "rem"):
        err = refused_type(f"unwrap_or({tag}(a, b), a)", f"a: {t}, b: {t}", t)
        assert "unsigned" in err.message  # floor versus truncation is a portability trap


@pytest.mark.parametrize("a_ty", ["Nat", "Int"])
@pytest.mark.parametrize("b_ty", ["Nat", "Int"])
def test_int_and_nat_div_rem_are_option_int_r7(a_ty, b_ty):
    # R7: two Nat operands give Option<Int> (the grammar checker's typing), so the result of
    # ``unwrap_or`` is an Int and cannot be returned as a Nat.
    for tag in ("div", "rem"):
        ir = lowered_ir(f"unwrap_or({tag}(a, b), 0)", f"a: {a_ty}, b: {b_ty}", "Int")
        assert ir == ["unwrap_or", [tag, ["var", "a"], ["var", "b"]], lit("Int", 0)]
        err = refused_type(f"unwrap_or({tag}(a, b), 0)", f"a: {a_ty}, b: {b_ty}", "Nat")
        assert "declared type" in err.message


def test_div_of_two_bare_literals_is_int():
    ir = lowered_ir("unwrap_or(div(7, 2), 0)", "a: Bool", "Int")
    assert ir[1] == ["div", lit("Int", 7), lit("Int", 2)]


@pytest.mark.parametrize("a_ty", ["Nat", "Int"])
@pytest.mark.parametrize("b_ty", ["Nat", "Int"])
def test_imin_and_imax_are_int(a_ty, b_ty):
    for tag in ("imin", "imax"):
        assert lowered_ir(f"{tag}(a, b)", f"a: {a_ty}, b: {b_ty}", "Int") == [
            tag,
            ["var", "a"],
            ["var", "b"],
        ]
    refused_type("imin(a, b)", "a: U8, b: U8", "Int")  # a fixed width never mixes with Int


@pytest.mark.parametrize("t", TYPES)
def test_endian_conversions_know_the_width(t):
    n = int(t[1:]) // 8
    for tag in ("be_bytes", "le_bytes"):
        assert lowered_ir(f"{tag}(a)", f"a: {t}", f"Bytes<{n}>") == [tag, ["var", "a"]]
    for tag in ("from_be", "from_le"):
        ir = lowered_ir(f"{tag}<{t}>(b)", f"b: Bytes<{n}>", t)
        assert ir == [tag, t, ["var", "b"]]
        # a runtime length other than w/8 is not accepted: no implicit truncation.
        refused_type(f"{tag}<{t}>(b)", f"b: Bytes<{n + 1}>", t)
        refused_type(f"{tag}<{t}>(b)", "b: Bytes", t)


def test_from_be_accepts_a_literal_and_a_concat_of_known_widths():
    ir = lowered_ir('from_be<U16>(b"0102")', "a: Bool", "U16")
    assert ir == ["from_be", "U16", ["lit", {"type": "Bytes", "value": "0102"}]]
    ir = lowered_ir('from_le<U32>(concat(t, b"000000"))', "t: Bytes<1>", "U32")
    assert ir[0] == "from_le" and ir[2][0] == "concat"
    refused_type('from_le<U32>(concat(t, b"0000"))', "t: Bytes<1>", "U32")
    refused_type("from_le<U32>(take(t, 4))", "t: Bytes<8>", "U32")


def test_at_on_bytes_with_a_nat_or_an_int_index():
    for params in ("b: Bytes, i: Nat", "b: Bytes<4>, i: Int", "b: Bytes, i: Int"):
        ir = lowered_ir("unwrap_or(at(b, i), 0u8)", params, "U8")
        assert ir == ["unwrap_or", ["at", ["var", "b"], ["var", "i"]], lit("U8", 0)]
    assert lowered_ir("unwrap_or(at(b, 0), 0u8)", "b: Bytes", "U8")[1][2] == lit("Nat", 0)


def test_at_on_a_list_is_the_reserved_ir_list():
    refused_level("unwrap_or(at(b, 0), 0u8)", "b: List<U8>", "U8", "ir.list")
    refused_type("unwrap_or(at(b, 0), 0u8)", "b: Nat", "U8")
    refused_type("unwrap_or(at(b, true), 0u8)", "b: Bytes", "U8")


def test_unwrap_or_is_typed_by_the_option_and_its_default():
    assert lowered_ir("unwrap_or(try<U8>(a), 7u8)", "a: Int", "U8")[2] == lit("U8", 7)
    refused_type("unwrap_or(try<U8>(a), 7)", "a: Int", "U8")  # an Int default for a U8 option
    refused_type("unwrap_or(try<U8>(a), 7u16)", "a: Int", "U8")
    refused_type("unwrap_or(a, 7u8)", "a: U8", "U8")  # not an Option
    # both operands are evaluated (cost is the sum), so a default may be any pure expression
    assert lowered_ir("unwrap_or(try<U8>(a), a + 1u8 + 0u8)", "a: U8", "U8")[2][0] == "add"


def test_every_option_producer_feeds_unwrap_or():
    cases = {
        "try": ("unwrap_or(try<U8>(a), 0u8)", "a: Int"),
        "checked_add": ("unwrap_or(checked_add(a, a), 0u8)", "a: U8"),
        "checked_sub": ("unwrap_or(checked_sub(a, a), 0u8)", "a: U8"),
        "checked_mul": ("unwrap_or(checked_mul(a, a), 0u8)", "a: U8"),
        "div": ("unwrap_or(div(a, a), 0u8)", "a: U8"),
        "rem": ("unwrap_or(rem(a, a), 0u8)", "a: U8"),
        "div_trunc": ("unwrap_or(div_trunc(a, a), 0i8)", "a: I8"),
        "rem_trunc": ("unwrap_or(rem_trunc(a, a), 0i8)", "a: I8"),
        "at": ("unwrap_or(at(b, 0), 0u8)", "b: Bytes"),
    }
    assert set(cases) == L.OPTION_PRODUCERS
    for tag, (body, params) in cases.items():
        ret = "I8" if tag.endswith("_trunc") else "U8"
        assert lowered_ir(body, params, ret)[1][0] == tag


# -- shapes outside Appendix A.1 are refused ----------------------------------------------


@pytest.mark.parametrize(
    "body,params,ret",
    [
        ("bit_xor(a, b)", "a: U8, b: U16", "U8"),  # mixed widths
        ("bit_xor(a, b)", "a: U8, b: I8", "U8"),  # mixed signedness
        ("bit_and(a, 1)", "a: U8", "U8"),  # a bare literal never becomes fixed
        ("bit_and(a, b)", "a: Nat, b: Nat", "Nat"),  # Nat is not fixed width
        ("bit_not(a)", "a: Nat", "Nat"),
        ("sar(a, 1)", "a: U8", "U8"),  # sar on an unsigned type
        ("shl(a, 1u8)", "a: U8", "U8"),  # the count is a Nat, not a fixed width
        ("shl(a, n)", "a: U8, n: Int", "U8"),  # an Int count is not a Nat
        ("rotl(a, n)", "a: Nat, n: Nat", "Nat"),
        ("bit(a, n)", "a: Int, n: Nat", "Bool"),
        ("popcount(a)", "a: Nat", "Nat"),
        ("clz(a)", "a: Bool", "Nat"),
        ("widen<U8>(a)", "a: U16", "U8"),  # range not inside the target
        ("widen<U32>(a)", "a: I8", "U32"),
        ("widen<U32>(a)", "a: Int", "U32"),  # widen is for fixed widths only
        ("wrap<U8>(a)", "a: Bool", "U8"),
        ("wrap<U8>(a)", "a: Bytes", "U8"),
        ("to_int(a)", "a: Nat", "Int"),
        ("to_nat(a)", "a: Int", "Nat"),
        ("to_nat(a)", "a: I8", "Nat"),  # a signed value reaches Nat only through try/wrap
        ("unwrap_or(checked_add(a, b), 0)", "a: U8, b: U8", "U8"),
        ("unwrap_or(checked_add(a, b), 0u8)", "a: Nat, b: Nat", "U8"),
        ("unwrap_or(div(a, b), 0u8)", "a: U8, b: I8", "U8"),
        ("unwrap_or(div(a, b), 0u8)", "a: U8, b: Nat", "U8"),  # fixed never mixes with Nat
        ("unwrap_or(div(a, b), 0)", "a: Bool, b: Bool", "Int"),
        ("unwrap_or(div_trunc(a, b), 0i8)", "a: I8, b: I16", "I8"),
        ("imax(a, b)", "a: U8, b: U8", "Int"),
        ("be_bytes(a)", "a: Nat", "Bytes"),
        ("le_bytes(a)", "a: Bool", "Bytes"),
        ("from_be<U16>(b)", "b: Bytes<1>", "U16"),
        ("from_be<U8>(a)", "a: U8", "U8"),
        ("max_value<Nat>()", "a: Bool", "Nat"),  # the static type is a fixed width
        ("max_value<Frob>()", "a: Bool", "U8"),
        ("max_value<8>()", "a: Bool", "U8"),
        ("min_value()", "a: Bool", "U8"),
        ("widen(a)", "a: U8", "U16"),  # missing static type argument
        ("wrap<U8, U16>(a)", "a: Int", "U8"),  # too many
    ],
)
def test_shapes_outside_appendix_a1_are_type_errors(body, params, ret):
    refused_type(body, params, ret)


@pytest.mark.parametrize(
    "body,params",
    [
        ("bit_and(a)", "a: U8"),  # arity
        ("bit_and(a, a, a)", "a: U8"),
        ("popcount(a, a)", "a: U8"),
        ("max_value<U8>(a)", "a: U8"),
        ("unwrap_or(div(a, a))", "a: U8"),
        ("bit_not", "a: U8"),  # a built-in needs an argument list
    ],
)
def test_wrong_arity_is_a_type_error(body, params):
    refused_type(body, params, "U8")


def test_a_static_argument_on_a_built_in_that_takes_none_is_refused():
    # ``shl<U8>(a, 1)`` does not even parse as a static application (shl takes none).
    with pytest.raises(OracleError):
        model("shl<U8>(a, 1)", "a: U8", "U8")


def test_a_built_in_in_a_v0_document_keeps_its_old_refusal():
    # VER04j 1.2: the 35 names are version scoped.  A v0 document is out of scope for S10a and
    # is refused at the level ir.fixed without being lowered, exactly as before.
    spec = lower("warrant p\n  model m(a: U8) -> U8 = bit_not(a)\n")
    m = spec.models["m"]
    assert m.ir is None and m.unsupported.level == "ir.fixed"


def test_a_user_model_may_not_be_named_like_a_built_in_but_may_be_named_like_a_lowerer_only_name():
    # `rem` is a v1 built-in of the checker (W0201): a declaration of it is refused (R58); `imin` is not one
    with pytest.raises(OracleError) as err:
        lower("warrant p v1\n  model rem(a: Nat) -> Nat = a\n")
    assert err.value.code == "type"
    spec = lower("warrant p v1\n  model imin2(a: Nat) -> Nat = a\n  model m(a: Nat) -> Nat = imin2(a)\n")
    assert spec.models["m"].ir == ["model_call", "imin2", [["var", "a"]]]


# -- Option<T>: the intermediate use is ir.fixed, every other Option is ir.data (R29) -----------


@pytest.mark.parametrize(
    "body,params,ret",
    [
        ("div(a, b) == div(a, b)", "a: U8, b: U8", "Bool"),  # eq operand
        ("div(a, b) != div(a, b)", "a: U8, b: U8", "Bool"),  # ne operand
        ("unwrap_or(if c then div(a, b) else div(b, a), 0)", "c: Bool, a: Nat, b: Nat", "Int"),
        ("try<U8>(a) == try<U8>(a)", "a: Int", "Bool"),
        ("checked_add(a, a) == checked_add(a, a)", "a: U8", "Bool"),
        ("at(b, 0) == at(b, 1)", "b: Bytes", "Bool"),
        ("not (div(a, b) == div(a, b))", "a: U8, b: U8", "Bool"),
        ("div(a, b) == div(a, b) and true", "a: U8, b: U8", "Bool"),
    ],
)
def test_an_option_anywhere_but_the_intermediate_use_requires_ir_data(body, params, ret):
    """R29 (d): a producer that is not directly the first operand of ``unwrap_or`` brings ``ir.data``."""
    m = model(body, params, ret)
    assert m.lowered and m.ir is not None, body
    assert "ir.data" in m.requires and "ir.fixed" in m.requires, body
    # S11c-1: evaluated (S11a: the evaluation refusal ``level ir.data``, never a lowering refusal)
    assert m.unsupported is None, body


@pytest.mark.parametrize(
    "body,params,ret",
    [
        ("rem(a, b) == a", "a: Nat, b: Nat", "Bool"),  # Option<Int> against a Nat
        ("unwrap_or(div(div(a, b), b), 0)", "a: Nat, b: Nat", "Int"),  # div of an Option
        ("unwrap_or(rem(a, b), rem(a, b))", "a: Nat, b: Nat", "Int"),  # an Option as the default
        ("id(div(a, b)) == a", "a: Nat, b: Nat", "Bool"),  # a model argument of the wrong type
        ("div(a, b)", "a: Nat, b: Nat", "Bool"),  # an Option where a Bool is declared
        ("if c then div(a, b) else div(b, a) == a", "c: Bool, a: Nat, b: Nat", "Bool"),
    ],
)
def test_an_option_that_does_not_fit_where_it_is_used_is_a_type_error(body, params, ret):
    """The consumer rule is relaxed, not removed (R29 (e)): an Option still has to have the right type."""
    refused_type_with_prelude(body, params, ret)


def refused_type_with_prelude(body: str, params: str, ret: str) -> OracleError:
    with pytest.raises(OracleError) as err:
        model(body, params, ret, prelude="  model id(a: Int) -> Int = a\n")
    assert err.value.code == "type", err.value
    return err.value


def test_an_option_is_a_real_signature_binder_and_value_type():
    spec = lower(
        "warrant p v1\n  op f(a: U8) -> Option<U8>\n  op g(a: Option<U8>) -> U8\n"
        "  claim c:\n    forall a: U8. f(a) == f(a)\n"
        "  claim d:\n    forall a: Option<U8>. g(a) == g(a)\n"
    )
    assert spec.ops["f"].requires == spec.ops["g"].requires == ("ir.core", "ir.data")
    assert [s.requires for s in spec.suites] == [("ir.core", "ir.data")] * 2
    # lowered and, since S11c-1, evaluated (S11a: the only refusal was the evaluation one, ``level ir.data``)
    assert all(s.lowered and s.unsupported is None for s in spec.suites)
    refused_type("a", "a: U8", "Option<U8>")  # a U8 is not an Option<U8>: Some(a) is


def test_some_none_and_match_on_an_option_are_ir_data():
    for body in ("unwrap_or(Some(a), 0u8) == a", "match try<U8>(a) with Some(x) -> true | None -> false"):
        m = model(body, "a: U8", "Bool") if "Some(a)" in body else model(body, "a: Int", "Bool")
        assert m.lowered and "ir.data" in m.requires and m.unsupported is None, body


def test_an_option_from_a_subject_op_may_be_unwrapped_and_requires_both_levels():
    # ``unwrap_or`` takes any Option (R29 (e)); only a producer node in the direct position is the
    # intermediate use, so this one requires ir.data as well as ir.fixed.
    spec = lower(
        "warrant p v1\n  op f(a: U8) -> Option<U8>\n  claim c:\n    forall a: U8. unwrap_or(f(a), 0u8) == a\n"
    )
    assert spec.suites[0].requires == ("ir.core", "ir.data", "ir.fixed")
    assert spec.suites[0].lowered


def test_a_producer_in_a_claim_is_lowered_and_requires_ir_fixed():
    spec = lower(
        "warrant p v1\n  op f(a: U8, b: U8) -> U8\n  claim c:\n"
        "    forall a: U8, b: U8. f(a, b) == unwrap_or(checked_add(a, b), 255u8)\n"
    )
    s = spec.suites[0]
    assert s.unsupported is None and s.kind == "reference" and s.ops == ["f"]
    assert s.expected[0] == "unwrap_or" and s.requires == ("ir.core", "ir.fixed")


# -- typing helpers on the type language ---------------------------------------------------


def test_type_levels_look_inside_type_arguments():
    from coding_adventures_warrant_oracle import ast as A

    tup = A.Ty("()", (A.Ty("U8"), A.Ty("Nat")))
    assert L.type_levels(A.Ty("U8")) == set()
    assert L.type_levels(tup) == {"ir.tuple"}
    assert L.type_levels(A.Ty("Result", (tup, A.Ty("E")))) == {"ir.tuple"}
    assert L.type_levels(A.Ty("P"), {"P"}) == {"ir.data"}  # a record or variant name
    assert L.type_levels(A.Ty("P")) == set()
    # slice S11a: ``Option`` is counted (R29 (d)), a list is not counted but refused (R27)
    assert L.type_levels(A.Ty("Option", (A.Ty("U8"),))) == {"ir.data"}
    assert L.type_levels(A.Ty("Option", (tup,))) == {"ir.data", "ir.tuple"}
    assert L.type_levels(A.Ty("List", (A.Ty("U8"),))) == set()
    assert L.type_refusal(tup) is None  # counted in requires, not refused
    assert L.type_refusal(A.Ty("Option", (A.Ty("U8"),))) is None
    assert L.type_refusal(A.Ty("List", (A.Ty("Option", (tup,)),))) == "ir.list"


def test_static_width_follows_the_checker_literals_concat_and_if():
    # warrant-grammar checker: a literal is Bytes<len>, a concat of two sized operands the sum,
    # an if with branches of the same static width keeps it (common_type), else plain Bytes.
    cases = [
        ('from_be<U16>(if c then b"0102" else b"0304")', "c: Bool"),
        ('from_be<U32>(if c then concat(t, b"000000") else b"01020304")', "c: Bool, t: Bytes<1>"),
        ('from_be<U16>(if c then t else b"0102")', "c: Bool, t: Bytes<2>"),
        ('from_le<U32>(concat(concat(t, b"00"), b"0000"))', "t: Bytes<1>"),
    ]
    for body, params in cases:
        ret = "U32" if "U32" in body else "U16"
        assert lowered_ir(body, params, ret)[0] in ("from_be", "from_le")
    refused_type('from_be<U16>(if c then b"01" else b"0102")', "c: Bool", "U16")  # widths differ
    refused_type('from_be<U16>(if c then t else b"0102")', "c: Bool, t: Bytes", "U16")
    refused_type('from_be<U8>(take(t, 1))', "t: Bytes<2>", "U8")


def test_deeply_nested_from_be_lowers_in_one_pass():
    import time

    expr = "a"
    for _ in range(21):
        expr = f'from_be<U8>(concat(be_bytes({expr}), b""))'
    start = time.perf_counter()
    lowered_ir(expr, "a: U8", "U8")
    assert time.perf_counter() - start < 10  # it was exponential (17 s at 21 nestings)


def test_requires_always_includes_ir_core_for_ops_and_units():
    spec = lower(
        "warrant r v1\n"
        "  op f(a: U8 | bit(max_value<U8>(), to_nat(max_value<U8>()))) -> U8\n"
        "  model m() -> U8 = max_value<U8>()\n"
        "  claim c:\n    forall a: U8. f(a) == a\n"
    )
    assert spec.ops["f"].requires == ("ir.core", "ir.fixed")
    assert spec.models["m"].requires == ("ir.core", "ir.fixed")


def test_record_variant_and_option_types_add_ir_data_and_a_list_is_refused():
    spec = lower(
        "warrant r v1\n  record P(x: Bool, y: Bool)\n  variant V = A | B\n"
        "  op h(p: P) -> Bool\n  op k(v: V) -> Bool\n  op o(a: U8) -> Option<U8>\n"
        "  op plain(a: U8) -> Bool\n"
        "  claim cr:\n    forall p: P. h(p) == true\n"
        "  claim cp:\n    forall a: U8. plain(a) == true\n"
    )
    assert {s.claim: s.requires for s in spec.suites} == {
        "cr": ("ir.core", "ir.data"),
        "cp": ("ir.core",),
    }
    assert spec.ops["h"].requires == ("ir.core", "ir.data")
    assert spec.ops["k"].requires == spec.ops["o"].requires == ("ir.core", "ir.data")
    assert spec.ops["plain"].requires == ("ir.core",)
    # a list type in a signature is the reserved ir.list, and refuses the document whole (R27)
    with pytest.raises(OracleError) as err:
        lower("warrant r v1\n  op g(t: List<Bool>) -> Bool\n")
    assert err.value.code == "level" and err.value.level == "ir.list"
    # in a claim binder or a model it is recorded on the unit, which did not lower
    spec = lower(
        "warrant r v1\n  op plain(a: U8) -> Bool\n"
        "  model m(t: List<Bool>) -> Bool = true\n"
        "  claim cl:\n    forall t: List<Bool>. plain(0u8) == true\n"
    )
    assert spec.models["m"].unsupported.level == "ir.list" and not spec.models["m"].lowered
    assert spec.suites[0].unsupported.level == "ir.list" and not spec.suites[0].lowered


def test_seal_reads_callee_requires_without_rewalking_the_closure():
    # a deep chain: each unit's closure is read from its callee's own requires
    lines = ["warrant r v1", "  model m0(a: U32) -> U32 = rotl(a, 1)"]
    for i in range(1, 40):
        lines.append(f"  model m{i}(a: U32) -> U32 = m{i - 1}(a)")
    spec = lower("\n".join(lines) + "\n")
    assert spec.models["m39"].requires == ("ir.core", "ir.fixed")
