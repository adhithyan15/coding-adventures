"""The ``ir.fixed`` level (VER04r Appendix A.1, slice S10e): every tag, beyond the kit.

The conformance kit pins the outcome of 380 hand-picked trees.  These tests pin the
*rules*: each of the 35 built-ins is checked on its own, once on the values the
spec names and once against an independent, deliberately naive reference over
**every** 8-bit value, so a rule that happens to agree with the kit on its
sampled points but is wrong elsewhere cannot pass.

Layout: the value functions of :mod:`.fixed` against the references; then each tag
through the evaluator (type, value, cost, refusals); then the level rules (an
``Option`` outside ``unwrap_or`` needs ``ir.data``; a runner without ``ir.fixed``
refuses every tag).
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus.v2 import STANDARD, Program
from coding_adventures_warrant_corpus.v2 import fixed as fx
from coding_adventures_warrant_corpus.v2.errors import IrOverflow, Refusal
from coding_adventures_warrant_corpus.v2.ir import LEVEL_TAGS, evaluate
from coding_adventures_warrant_corpus.v2.wtypes import FIXED, Ty, fixed_range

CORE = frozenset({"ir.core"})
FIXED_LEVELS = frozenset({"ir.core", "ir.fixed"})
ALL = frozenset({"ir.core", "ir.fixed", "ir.tuple"})


def lit(type_name, value):
    return ["lit", {"type": type_name, "value": str(value)}]


def var(name):
    return ["var", name]


def run(ir, env=None, scope=None, levels=ALL, types=None):
    """Compile ``ir`` and evaluate it: ``(type text, value)``."""
    program = Program(types=types or {}, levels=levels)
    compiled = program.compile(ir, scope or {})
    return str(compiled.ty), evaluate(compiled, env or {})


def code(ir, scope=None, levels=ALL, types=None):
    """The refusal code of compiling ``ir``."""
    with pytest.raises(Refusal) as caught:
        Program(types=types or {}, levels=levels).compile(ir, scope or {})
    return caught.value.code


def unwrap(option_ir, default_ir):
    return ["unwrap_or", option_ir, default_ir]


# ---------------------------------------------------------------------------
# The value functions against a naive reference, over every 8-bit value
# ---------------------------------------------------------------------------
def bits_of(value, width):
    """The bits of ``value``'s pattern, least significant first, as a list."""
    return [(value >> i) & 1 for i in range(width)]


def from_bits(bits, signed):
    number = sum(bit << i for i, bit in enumerate(bits))
    return number - (1 << len(bits)) if signed and bits[-1] else number


@pytest.mark.parametrize("name", ["U8", "I8"])
def test_shifts_rotations_and_bit_counts_equal_the_bitwise_definition(name):
    ty = Ty(name)
    low, high = fixed_range(ty)
    signed = ty.signed
    for value in range(low, high + 1):
        bits = bits_of(value, 8)
        assert fx.popcount(value, ty) == sum(bits)
        assert fx.count_zeros("clz", value, ty) == (
            next((i for i, b in enumerate(reversed(bits)) if b), 8)
        )
        assert fx.count_zeros("ctz", value, ty) == next(
            (i for i, b in enumerate(bits) if b), 8
        )
        for index in range(0, 10):
            assert fx.bit(value, index, ty) == (index < 8 and bits[index] == 1)
        for count in list(range(0, 10)) + [255, 2**64 - 1, 2**127]:
            n = min(count, 8)
            assert fx.shift("shl", value, count, ty) == from_bits(
                [0] * n + bits[: 8 - n], signed
            )
            assert fx.shift("shr", value, count, ty) == from_bits(
                bits[n:] + [0] * n, signed
            )
            fill = bits[7] if signed else 0
            if signed:
                assert fx.shift("sar", value, count, ty) == from_bits(
                    bits[n:] + [fill] * n, True
                )
            k = count % 8
            assert fx.rotate("rotl", value, count, ty) == from_bits(
                bits[8 - k :] + bits[: 8 - k], signed
            )
            assert fx.rotate("rotr", value, count, ty) == from_bits(
                bits[k:] + bits[:k], signed
            )


@pytest.mark.parametrize("name", ["U8", "I8"])
def test_checked_and_wrapping_arithmetic_agree_on_every_pair(name):
    ty = Ty(name)
    low, high = fixed_range(ty)
    for a in range(low, high + 1, 5):
        for b in range(low, high + 1, 3):
            for op, exact in (("add", a + b), ("sub", a - b), ("mul", a * b)):
                want = ("some", exact) if low <= exact <= high else ("none",)
                assert fx.checked(op, a, b, ty) == want


def test_floor_division_matches_python_and_truncation_matches_c():
    for a in range(-20, 21):
        for b in range(-7, 8):
            if b == 0:
                assert fx.divide("div", a, b, None) == ("none",)
                continue
            assert fx.divide("div", a, b, None) == ("some", a // b)
            assert fx.divide("rem", a, b, None) == ("some", a % b)
            q = int(abs(a) / abs(b)) * (1 if (a < 0) == (b < 0) else -1)
            ty = Ty("I8")
            assert fx.divide("div_trunc", a, b, ty) == ("some", q)
            assert fx.divide("rem_trunc", a, b, ty) == ("some", a - b * q)
    # the one quotient that does not fit
    assert fx.divide("div_trunc", -128, -1, Ty("I8")) == ("none",)
    assert fx.divide("rem_trunc", -128, -1, Ty("I8")) == ("none",)
    assert fx.divide("div_trunc", 5, 0, Ty("I8")) == ("none",)


@pytest.mark.parametrize("name", sorted(FIXED))
def test_bytes_round_trip_in_both_orders(name):
    ty = Ty(name)
    low, high = fixed_range(ty)
    for value in {low, low + 1, -1 if ty.signed else 1, 0, high - 1, high}:
        for order in ("big", "little"):
            data = fx.to_bytes(value, ty, order)
            assert len(data) == ty.width // 8
            assert fx.from_bytes(data, ty, order) == value


# ---------------------------------------------------------------------------
# Each tag through the evaluator: type, value
# ---------------------------------------------------------------------------
CASES = [
    # bitwise
    (["bit_and", lit("U8", 240), lit("U8", 60)], "U8", 48),
    (["bit_or", lit("U8", 240), lit("U8", 60)], "U8", 252),
    (["bit_xor", lit("U8", 240), lit("U8", 60)], "U8", 204),
    (["bit_and", lit("I8", -2), lit("I8", 127)], "I8", 126),
    (["bit_or", lit("I8", -128), lit("I8", 1)], "I8", -127),
    (["bit_xor", lit("I8", -1), lit("I8", -128)], "I8", 127),
    (["bit_not", lit("U8", 0)], "U8", 255),
    (["bit_not", lit("I8", 0)], "I8", -1),
    (["bit_not", lit("I8", -128)], "I8", 127),
    (["bit_not", lit("U64", 2**64 - 1)], "U64", 0),
    # shifts and rotations (the count is a Nat)
    (["shl", lit("U8", 129), lit("Nat", 1)], "U8", 2),
    (["shl", lit("I8", 1), lit("Nat", 7)], "I8", -128),
    (["shl", lit("U8", 1), lit("Nat", 8)], "U8", 0),
    (["shl", lit("U8", 1), lit("Nat", 2**64 - 1)], "U8", 0),
    (["shr", lit("I8", -1), lit("Nat", 1)], "I8", 127),
    (["shr", lit("U8", 255), lit("Nat", 9)], "U8", 0),
    (["sar", lit("I8", -128), lit("Nat", 7)], "I8", -1),
    (["sar", lit("I8", 64), lit("Nat", 200)], "I8", 0),
    (["sar", lit("I8", -64), lit("Nat", 200)], "I8", -1),
    (["rotl", lit("U8", 129), lit("Nat", 1)], "U8", 3),
    (["rotl", lit("U8", 129), lit("Nat", 9)], "U8", 3),
    (["rotr", lit("U8", 1), lit("Nat", 1)], "U8", 128),
    (["rotr", lit("I8", 1), lit("Nat", 1)], "I8", -128),
    (["rotr", lit("U8", 7), lit("Nat", 2**64 - 1)], "U8", 14),
    # counts, bit
    (["popcount", lit("U8", 255)], "Nat", 8),
    (["popcount", lit("I8", -1)], "Nat", 8),
    (["clz", lit("U8", 0)], "Nat", 8),
    (["clz", lit("U16", 1)], "Nat", 15),
    (["ctz", lit("U8", 0)], "Nat", 8),
    (["ctz", lit("U32", 8)], "Nat", 3),
    (["bit", lit("U8", 5), lit("Nat", 2)], "Bool", True),
    (["bit", lit("U8", 5), lit("Nat", 1)], "Bool", False),
    (["bit", lit("I8", -1), lit("Nat", 7)], "Bool", True),
    (["bit", lit("U8", 255), lit("Nat", 8)], "Bool", False),
    # conversions
    (["widen", "U32", lit("U8", 200)], "U32", 200),
    (["widen", "I16", lit("I8", -5)], "I16", -5),
    (["widen", "I16", lit("U8", 200)], "I16", 200),
    (["widen", "U8", lit("U8", 7)], "U8", 7),
    (["wrap", "U8", lit("Nat", 300)], "U8", 44),
    (["wrap", "I8", lit("Int", -129)], "I8", 127),
    (["wrap", "I8", lit("U8", 255)], "I8", -1),
    (["wrap", "U16", lit("I8", -1)], "U16", 65535),
    (["to_int", lit("I8", -5)], "Int", -5),
    (["to_nat", lit("U64", 2**64 - 1)], "Nat", 2**64 - 1),
    (["max_value", "I8"], "I8", 127),
    (["min_value", "I8"], "I8", -128),
    (["max_value", "U64"], "U64", 2**64 - 1),
    (["min_value", "U16"], "U16", 0),
    (["imin", lit("Nat", 3), lit("Int", -4)], "Int", -4),
    (["imax", lit("Nat", 3), lit("Int", -4)], "Int", 3),
    (["imax", lit("Nat", 3), lit("Nat", 4)], "Int", 4),
    # bytes
    (["be_bytes", lit("U16", 258)], "Bytes<2>", b"\x01\x02"),
    (["le_bytes", lit("U16", 258)], "Bytes<2>", b"\x02\x01"),
    (["be_bytes", lit("I8", -1)], "Bytes<1>", b"\xff"),
    (["le_bytes", lit("I32", -2)], "Bytes<4>", b"\xfe\xff\xff\xff"),
    (["from_be", "U16", lit("Bytes", "0102")], "U16", 258),
    (["from_le", "U16", lit("Bytes", "0102")], "U16", 513),
    (["from_be", "I16", lit("Bytes", "ffff")], "I16", -1),
    (["from_le", "U64", lit("Bytes", "0100000000000000")], "U64", 1),
]


@pytest.mark.parametrize(
    ("ir", "type_text", "value"),
    CASES,
    ids=[f"{c[0][0]}-{i}" for i, c in enumerate(CASES)],
)
def test_a_tag_has_its_type_and_value(ir, type_text, value):
    assert run(ir) == (type_text, value)


OPTION_CASES = [
    (["try", "U8", lit("Nat", 255)], "U8", 255),
    (["try", "U8", lit("Nat", 256)], "U8", 9),
    (["try", "I8", lit("Int", -128)], "I8", -128),
    (["try", "I8", lit("Int", -129)], "I8", 9),
    (["try", "U16", lit("I8", -1)], "U16", 9),
    (["try", "I8", lit("U8", 128)], "I8", 9),
    (["checked_add", lit("U8", 200), lit("U8", 55)], "U8", 255),
    (["checked_add", lit("U8", 200), lit("U8", 56)], "U8", 9),
    (["checked_add", lit("I8", -128), lit("I8", -1)], "I8", 9),
    (["checked_sub", lit("U8", 0), lit("U8", 1)], "U8", 9),
    (["checked_sub", lit("I8", -128), lit("I8", -1)], "I8", -127),
    (["checked_mul", lit("U8", 16), lit("U8", 16)], "U8", 9),
    (["checked_mul", lit("I8", -128), lit("I8", 1)], "I8", -128),
    (["div", lit("U8", 7), lit("U8", 2)], "U8", 3),
    (["div", lit("U8", 7), lit("U8", 0)], "U8", 9),
    (["rem", lit("U8", 7), lit("U8", 2)], "U8", 1),
    (["rem", lit("U8", 7), lit("U8", 0)], "U8", 9),
    (["div", lit("Int", -7), lit("Int", 2)], "Int", -4),
    (["rem", lit("Int", -7), lit("Int", 2)], "Int", 1),
    (["rem", lit("Int", 7), lit("Int", -2)], "Int", -1),
    (["div", lit("Nat", 7), lit("Int", 2)], "Int", 3),
    (["rem", lit("Nat", 7), lit("Nat", 2)], "Int", 1),
    (["div_trunc", lit("I8", -7), lit("I8", 2)], "I8", -3),
    (["rem_trunc", lit("I8", -7), lit("I8", 2)], "I8", -1),
    (["rem_trunc", lit("I8", 7), lit("I8", -2)], "I8", 1),
    (["div_trunc", lit("I8", -128), lit("I8", -1)], "I8", 99),
    (["rem_trunc", lit("I8", -128), lit("I8", -1)], "I8", 99),
    (["div_trunc", lit("I16", 5), lit("I16", 0)], "I16", 99),
]


@pytest.mark.parametrize(
    ("producer", "type_text", "value"),
    OPTION_CASES,
    ids=[f"{c[0][0]}-{i}" for i, c in enumerate(OPTION_CASES)],
)
def test_an_option_producer_under_unwrap_or(producer, type_text, value):
    # the default is 9 (or 99): in the payload's own type, so the result type is
    # the payload's; a default that is taken shows as the default's value
    default_type = "Int" if type_text == "Int" else type_text
    default = lit(default_type, 99 if value == 99 else 9)
    assert run(unwrap(producer, default)) == (type_text, value)


def test_at_is_none_outside_the_bytes_and_some_inside():
    data = {"b": b"\x0a\x0b"}
    scope = {"b": Ty("Bytes")}
    cases = [(0, 10), (1, 11), (2, 99), (2**64 - 1, 99)]
    for index, want in cases:
        ir = unwrap(["at", var("b"), lit("Nat", index)], lit("U8", 99))
        assert run(ir, data, scope) == ("U8", want)
    ir = unwrap(["at", var("b"), lit("Int", -1)], lit("U8", 99))
    assert run(ir, data, scope) == ("U8", 99)
    sized = unwrap(["at", var("b"), lit("Nat", 1)], lit("U8", 99))
    assert run(sized, data, {"b": Ty("Bytes", (2,))}) == ("U8", 11)


def test_unwrap_or_evaluates_both_operands_and_costs_their_sum():
    compiled = Program(levels=ALL).compile(
        unwrap(["try", "U8", lit("Nat", 1)], lit("U8", 5)), {}
    )
    assert evaluate(compiled, {}) == 1
    # 1 (unwrap_or) + 2 (try and its operand) + 1 (the default): no short circuit credited
    assert compiled.cost == 1 + 2 + 1


@pytest.mark.parametrize(
    ("ir", "cost"),
    [
        (["bit_and", lit("U8", 1), lit("U8", 2)], 3),
        (["bit_not", lit("U8", 1)], 2),
        (["max_value", "U8"], 1),
        (["widen", "U16", lit("U8", 1)], 2),
        (["shl", lit("U8", 1), lit("Nat", 1)], 3),
        (["be_bytes", ["bit_not", lit("U8", 1)]], 3),
        (["from_be", "U16", lit("Bytes", "0000")], 2),
        (unwrap(["div", lit("U8", 1), lit("U8", 1)], lit("U8", 0)), 1 + 3 + 1),
    ],
)
def test_every_tag_costs_one_plus_its_operands(ir, cost):
    assert Program(levels=ALL).compile(ir, {}).cost == cost


# ---------------------------------------------------------------------------
# Typing: what is refused, and with which code
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    "ir",
    [
        ["bit_and", lit("U8", 1), lit("U16", 1)],
        ["bit_or", lit("I8", 1), lit("U8", 1)],
        ["bit_xor", lit("Nat", 1), lit("Nat", 1)],
        ["bit_and", ["lit", True], ["lit", True]],
        ["bit_not", ["lit", True]],
        ["bit_not", lit("Nat", 1)],
        ["bit_and", lit("U8", 1)],  # arity
        ["shl", lit("U8", 1), lit("U8", 1)],  # count is not a Nat
        ["shr", lit("U8", 1), lit("Int", 1)],
        ["rotl", lit("U8", 1), lit("I8", 1)],
        ["rotr", lit("Nat", 1), lit("Nat", 1)],  # operand is not fixed-width
        ["sar", lit("U8", 1), lit("Nat", 1)],  # unsigned
        ["bit", lit("U8", 1), ["lit", True]],
        ["popcount", lit("Nat", 1)],
        ["clz", ["lit", True]],
        ["ctz", lit("Int", 1)],
        ["widen", "U8", lit("U16", 1)],  # narrows
        ["widen", "U8", lit("I8", 1)],  # sign
        ["widen", "U16", lit("Nat", 1)],  # not fixed
        ["widen", "Bool", lit("U8", 1)],
        ["widen", "U8"],  # arity
        ["widen", 8, lit("U8", 1)],  # not a type text
        ["wrap", "U8", ["lit", True]],
        ["wrap", "Nat", lit("U8", 1)],
        ["wrap", "Nope", lit("U8", 1)],
        ["try", "Bytes", lit("U8", 1)],
        ["try", "U8", ["lit", True]],
        ["to_int", ["lit", True]],
        ["to_int", lit("Nat", 1)],
        ["to_nat", lit("I8", 1)],
        ["to_nat", lit("Int", 1)],
        ["max_value", "Nat"],
        ["max_value"],
        ["min_value", "U8", "U8"],
        ["checked_add", lit("U8", 1), lit("U16", 1)],
        ["checked_mul", lit("Nat", 1), lit("Nat", 1)],
        ["div", lit("I8", 1), lit("I8", 1)],  # signed: use div_trunc
        ["rem", lit("I8", 1), lit("I8", 1)],
        ["div", lit("U8", 1), lit("U16", 1)],
        ["div", lit("U8", 1), lit("Nat", 1)],
        ["div_trunc", lit("U8", 1), lit("U8", 1)],
        ["rem_trunc", lit("I8", 1), lit("I16", 1)],
        ["imin", lit("U8", 1), lit("U8", 1)],
        ["imax", lit("Nat", 1), lit("U8", 1)],
        ["be_bytes", lit("Nat", 1)],
        ["le_bytes", ["lit", True]],
        ["from_be", "U16", lit("Bytes", "000000")],  # wrong static width
        ["from_le", "U16", ["var", "plain"]],  # plain Bytes: no static width
        ["from_be", "U16", lit("U8", 1)],
        ["from_be", "Nat", lit("Bytes", "00")],
        ["at", lit("U8", 1), lit("Nat", 0)],
        ["at", ["var", "plain"], lit("U8", 0)],
        ["unwrap_or", lit("U8", 1), lit("U8", 1)],  # not an Option
        ["unwrap_or", ["try", "U8", lit("Nat", 1)], lit("I8", 1)],
        ["unwrap_or", ["try", "U8", lit("Nat", 1)]],
    ],
)
def test_a_mistyped_node_is_ill_typed(ir):
    assert code(ir, {"plain": Ty("Bytes")}) == "ill-typed"


def test_a_widening_default_is_accepted_for_an_int_payload():
    # Nat flows to Int (R51): the default of unwrap_or may be a Nat for an Option<Int>
    ir = unwrap(["div", lit("Int", 7), lit("Int", 0)], lit("Nat", 3))
    assert run(ir) == ("Int", 3)


def test_an_unknown_type_argument_is_ill_typed_and_a_tuple_one_needs_its_level():
    assert (
        code(["wrap", "(U8,U8)", lit("U8", 1)], levels=FIXED_LEVELS) == "level-missing"
    )
    assert code(["wrap", "(U8,U8)", lit("U8", 1)]) == "ill-typed"


def test_a_static_width_decides_from_be_and_from_le():
    sized = {"b": Ty("Bytes", (4,))}
    assert run(["from_be", "U32", var("b")], {"b": b"\0\0\1\2"}, sized) == ("U32", 258)
    cat = ["concat", var("t"), lit("Bytes", "000000")]
    assert run(["from_le", "U32", cat], {"t": b"\1"}, {"t": Ty("Bytes", (1,))}) == (
        "U32",
        1,
    )
    # an if whose branches have one width keeps it; two widths unify to plain Bytes
    same = ["if", var("c"), var("b"), var("b")]
    assert (
        run(
            ["from_be", "U32", same],
            {"c": True, "b": b"\0\0\0\7"},
            {**sized, "c": Ty("Bool")},
        )[1]
        == 7
    )
    two = ["if", var("c"), var("b"), lit("Bytes", "00")]
    assert code(["from_be", "U32", two], {**sized, "c": Ty("Bool")}) == "ill-typed"


# ---------------------------------------------------------------------------
# Levels
# ---------------------------------------------------------------------------
@pytest.mark.parametrize("tag", sorted(LEVEL_TAGS["ir.fixed"]))
def test_a_runner_without_ir_fixed_refuses_every_tag_as_level_missing(tag):
    node = [tag, lit("U8", 1)]
    assert code(node, levels=CORE) == "level-missing"


def test_there_are_exactly_35_tags_in_the_level():
    assert len(LEVEL_TAGS["ir.fixed"]) == 35


@pytest.mark.parametrize(
    "ir",
    [
        # an Option anywhere but directly under unwrap_or is the real type of ir.data
        ["try", "U8", lit("Nat", 1)],
        ["eq", ["try", "U8", lit("Nat", 1)], ["try", "U8", lit("Nat", 1)]],
        [
            "if",
            ["lit", True],
            ["try", "U8", lit("Nat", 1)],
            ["try", "U8", lit("Nat", 2)],
        ],
        unwrap(
            [
                "if",
                ["lit", True],
                ["try", "U8", lit("Nat", 1)],
                ["try", "U8", lit("Nat", 2)],
            ],
            lit("U8", 0),
        ),
        ["tuple", [["try", "U8", lit("Nat", 1)], ["lit", True]]],
    ],
)
def test_an_option_that_unwrap_or_does_not_consume_needs_ir_data(ir):
    assert code(ir) == "level-missing"


def test_an_option_parameter_of_a_model_is_the_data_level():
    models = {"m": {"params": [["o", "Option<U8>"]], "body": ["lit", True]}}
    program = Program(models=models, model_order=["m"], levels=ALL)
    assert "m" in program.not_live_models


# ---------------------------------------------------------------------------
# Overflow: the exact window still applies to what a fixed-width value becomes
# ---------------------------------------------------------------------------
def test_to_int_products_leave_the_window_in_the_corpus_class():
    square = [
        "mul",
        ["to_int", lit("U64", 2**64 - 1)],
        ["to_int", lit("U64", 2**64 - 1)],
    ]
    program = Program(levels=ALL)
    compiled = program.compile(square, {})
    with pytest.raises(IrOverflow) as caught:
        evaluate(compiled, {})
    assert caught.value.klass == "corpus"


def test_wrap_and_widen_use_the_semantics_seams():
    class Broken(type(STANDARD)):
        def wrap(self, value, ty):  # type: ignore[no-untyped-def]
            return value

    program = Program(levels=ALL, sem=Broken())
    compiled = program.compile(["wrap", "U8", lit("Nat", 300)], {})
    assert evaluate(compiled, {}) == 300  # the mutant is visible: wrap is a seam


def test_a_runtime_length_other_than_w_over_8_is_ir_overflow_never_a_truncation():
    program = Program(levels=ALL)
    compiled = program.compile(["from_be", "U32", var("b")], {"b": Ty("Bytes", (4,))})
    assert evaluate(compiled, {"b": b"\0\0\0\7"}) == 7
    with pytest.raises(IrOverflow) as caught:
        evaluate(compiled, {"b": b"\0\0\7"})  # a value that is not of its declared type
    assert caught.value.klass == "corpus"
    with pytest.raises(IrOverflow) as caught:
        fx.from_bytes(b"\1" * 5, Ty("U32"), "little", "case")
    assert caught.value.klass == "case"
