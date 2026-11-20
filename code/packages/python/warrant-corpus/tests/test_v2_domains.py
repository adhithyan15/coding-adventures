"""Descriptors of VER04r section 5.1: ``size()`` and ``nth()`` for every node kind.

The kit pins the ``ir.core`` leaves and the ``result`` composite.  Tuples,
variants, options and lists belong to later levels, but the descriptor reader
implements every node of the section 5.1 table, so each is tested here against
the spec's own words.  The byte domains are also cross-checked against the v1
lane's independent enumeration (``coding_adventures_warrant_corpus.domains``).
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus import domains as v1
from coding_adventures_warrant_corpus.v2.domains import (
    MAX_DESCRIPTOR_DEPTH,
    build,
    descriptor_for,
    is_descriptor_for,
)
from coding_adventures_warrant_corpus.v2.errors import Refusal
from coding_adventures_warrant_corpus.v2.semantics import STANDARD
from coding_adventures_warrant_corpus.v2.wtypes import Ty, parse_type


def leaf(kind, size, **extra):
    return {"d": kind, "size": size, **extra}


BOOL = leaf("bool", 2)
U8 = leaf("u8_all", 256)
NAT3 = leaf("nat_count", 4, k=3)


def values(node):
    desc = build(node)
    return [desc.nth(i) for i in range(desc.size)]


def code_of(call, *args):
    with pytest.raises(Refusal) as caught:
        call(*args)
    return caught.value.code


# -- the leaves ---------------------------------------------------------------------
def test_the_simple_leaves():
    assert values(BOOL) == [False, True]
    assert values(U8) == list(range(256))
    assert values(leaf("nat_0_64", 65)) == list(range(65))
    assert values(NAT3) == [0, 1, 2, 3]
    assert values(leaf("nat_count", 1, k=0)) == [0]


def test_nat_shift_is_73_values_with_the_boundary_set():
    shift = values(leaf("nat_shift", 73))
    assert shift[:65] == list(range(65))
    assert shift[65:] == [65, 127, 128, 129, 255, 256, 65535, 65536]


def test_u64_edge_matches_the_v1_lanes_independent_enumeration():
    assert values(leaf("u64_edge", 377)) == v1._u64_edge()


def test_bytes_d1_matches_the_v1_lanes_independent_enumeration():
    assert values(leaf("bytes_d1", 347)) == v1._bytes_d1()


@pytest.mark.parametrize("width", [0, 1, 2, 3, 4, 5, 16, 33])
def test_bytes_n_matches_the_v1_lanes_independent_enumeration(width):
    expected = v1._bytes_n(width)
    assert values(leaf("bytes_n", len(expected), n=width)) == expected


def test_fx_full_is_every_value_ascending():
    assert values(leaf("fx_full", 256, t="I8")) == list(range(-128, 128))
    assert values(leaf("fx_full", 65536, t="U16"))[:3] == [0, 1, 2]
    assert values(leaf("fx_full", 65536, t="I16"))[-2:] == [32766, 32767]


@pytest.mark.parametrize(
    ("name", "size"),
    [("U16", 281), ("U32", 313), ("I16", 527), ("I32", 559), ("I64", 623)],
)
def test_fx_edge_sizes_are_the_specs(name, size):
    edge = values(leaf("fx_edge", size, t=name))
    assert len(edge) == size == len(set(edge))
    bits = int(name[1:])
    if name.startswith("U"):
        assert edge[:256] == list(range(256)) and edge[256] == 1 << 8
        assert edge[-1] == (1 << (bits - 1)) - 1  # MAX ^ (1 << (w-1))
        assert (1 << bits) - 1 in edge and 1 << (bits - 1) in edge
    else:
        assert edge == sorted(edge) and edge[0] == -(1 << (bits - 1))
        assert set(range(-255, 0)) <= set(edge)


# -- the composites ----------------------------------------------------------------------
def test_a_tuple_is_the_product_with_the_last_component_fastest():
    node = {"d": "tuple", "of": [BOOL, NAT3], "size": 8}
    assert values(node) == [(a, b) for a in (False, True) for b in range(4)]


def test_a_nested_tuple_unranks_by_mixed_radix_division():
    node = {
        "d": "tuple",
        "of": [BOOL, {"d": "tuple", "of": [NAT3, BOOL], "size": 8}, BOOL],
        "size": 32,
    }
    flat = values(node)
    assert flat[0] == (False, (0, False), False) and flat[1] == (
        False,
        (0, False),
        True,
    )
    assert flat[2] == (False, (0, True), False) and flat[-1] == (True, (3, True), True)
    assert len(flat) == len(set(flat)) == 32


def test_an_option_is_none_then_some_of_each_value():
    node = {"d": "option", "of": NAT3, "size": 5}
    assert values(node) == [
        ("none",),
        ("some", 0),
        ("some", 1),
        ("some", 2),
        ("some", 3),
    ]


def test_a_result_is_ok_of_each_value_then_each_error_case():
    node = {"d": "result", "ok": BOOL, "errs": ["BAD", "WORSE"], "size": 4}
    assert values(node) == [
        ("ok", False),
        ("ok", True),
        ("err", "BAD"),
        ("err", "WORSE"),
    ]


def test_a_variant_is_its_constructors_in_order_each_a_field_product():
    node = {
        "d": "variant",
        "ctors": [["Dot", []], ["One", [BOOL]], ["Two", [BOOL, NAT3]]],
        "size": 1 + 2 + 8,
    }
    got = values(node)
    assert got[0] == ("Dot", ()) and got[1:3] == [("One", (False,)), ("One", (True,))]
    assert (
        got[3] == ("Two", (False, 0))
        and got[-1] == ("Two", (True, 3))
        and len(got) == 11
    )


def test_a_list_descriptor_is_the_level_ir_list_whatever_its_size():
    """[D63]: the formula of section 5.1 is not implemented; the level is refused."""
    for node in (
        {"d": "list", "of": BOOL, "size": 15},
        {"d": "list", "of": BOOL, "size": 1},  # the size is never even compared
    ):
        assert code_of(build, node) == "level-missing"
    assert (
        code_of(
            descriptor_for, {"d": "list", "of": BOOL, "size": 15}, "List<Bool>", None
        )
        == "level-missing"
    )


# -- validation -----------------------------------------------------------------------------------
BAD = [
    ("an unknown kind", {"d": "frobnicate", "size": 2}),
    ("a missing size", {"d": "bool"}),
    ("a size that is text", leaf("bool", "2")),
    ("a size that is a bool", leaf("bool", True)),
    ("the wrong size", leaf("bool", 3)),
    ("an extra key", leaf("bool", 2, k=1)),
    ("a missing key", {"d": "bytes_n", "size": 16}),
    ("not an object", ["bool", 2]),
    ("d is not text", {"d": 3, "size": 2}),
    ("fx_edge of U8", leaf("fx_edge", 256, t="U8")),
    ("fx_edge of U64", leaf("fx_edge", 377, t="U64")),
    ("fx_edge of 8 bits", leaf("fx_edge", 256, t="I8")),
    ("fx_full of U8", leaf("fx_full", 256, t="U8")),
    ("fx_full of 32 bits", leaf("fx_full", 1 << 32, t="U32")),
    ("fx_full of Nat", leaf("fx_full", 256, t="Nat")),
    ("fx_full with t not text", leaf("fx_full", 256, t=8)),
    ("bytes_n past 4096", leaf("bytes_n", 12295, n=4097)),
    ("bytes_n negative", leaf("bytes_n", 1, n=-1)),
    ("bytes_n not an int", leaf("bytes_n", 1, n=True)),
    ("nat_count past 4096", leaf("nat_count", 4098, k=4097)),
    ("nat_count negative", leaf("nat_count", 0, k=-1)),
    ("result errs not names", {"d": "result", "ok": BOOL, "errs": [1], "size": 3}),
    (
        "result with 65 errs",
        {"d": "result", "ok": BOOL, "errs": [f"E{i}" for i in range(65)], "size": 67},
    ),
    (
        "result with a duplicate err",
        {"d": "result", "ok": BOOL, "errs": ["A", "A"], "size": 4},
    ),
    (
        "result with a bad err name",
        {"d": "result", "ok": BOOL, "errs": ["not a name"], "size": 3},
    ),
    ("result with no errs", {"d": "result", "ok": BOOL, "errs": [], "size": 2}),
    ("result errs not a list", {"d": "result", "ok": BOOL, "errs": "BAD", "size": 3}),
    (
        "result with a wrong inner size",
        {"d": "result", "ok": leaf("bool", 3), "errs": ["B"], "size": 4},
    ),
    ("a one-component tuple", {"d": "tuple", "of": [BOOL], "size": 2}),
    ("a nine-component tuple", {"d": "tuple", "of": [BOOL] * 9, "size": 512}),
    ("a tuple whose of is not a list", {"d": "tuple", "of": "x", "size": 2}),
    ("a one-constructor variant", {"d": "variant", "ctors": [["A", []]], "size": 1}),
    (
        "a malformed constructor",
        {"d": "variant", "ctors": [["A"], ["B", []]], "size": 2},
    ),
    (
        "a constructor name that is not text",
        {"d": "variant", "ctors": [[1, []], ["B", []]], "size": 2},
    ),
    (
        "a constructor's fields not a list",
        {"d": "variant", "ctors": [["A", 1], ["B", []]], "size": 2},
    ),
]


@pytest.mark.parametrize(("why", "node"), BAD, ids=[w for w, _n in BAD])
def test_a_malformed_descriptor_is_refused_as_descriptor(why, node):
    assert code_of(build, node) == "descriptor", why


def test_a_tree_deeper_than_16_levels_is_refused():
    def nest(levels):
        node, size = BOOL, 2
        for _ in range(levels - 1):
            size += 1
            node = {"d": "option", "of": node, "size": size}
        return node

    assert build(nest(MAX_DESCRIPTOR_DEPTH)).size == 17
    assert code_of(build, nest(MAX_DESCRIPTOR_DEPTH + 1)) == "descriptor"


# -- is the tree one FOR the type? -------------------------------------------------------------------
TYPES = {
    "E": {"kind": "error", "cases": ["BAD", "WORSE"]},
    "Point": {"kind": "record", "fields": [["x", "Bool"], ["y", "U8"]]},
    "Shape": {"kind": "variant", "ctors": [["Dot", []], ["One", ["Bool"]]]},
}


def desc_for(node, text, types=TYPES):
    return descriptor_for(node, text, types)[0]


def test_a_tuple_descriptor_fits_a_tuple_or_a_record_type():
    node = {"d": "tuple", "of": [BOOL, U8], "size": 512}
    assert desc_for(node, "(Bool,U8)").size == 512
    assert desc_for(node, "Point").size == 512
    assert desc_for(node, "(Bool,U8)", None).size == 512
    assert code_of(descriptor_for, node, "(U8,Bool)", TYPES) == "descriptor"
    assert code_of(descriptor_for, node, "(Bool,U8,Bool)", TYPES) == "descriptor"
    assert code_of(descriptor_for, node, "Bool", TYPES) == "descriptor"


def test_option_list_and_result_descriptors_fit_their_types():
    assert desc_for({"d": "option", "of": BOOL, "size": 3}, "Option<Bool>").size == 3
    result = {"d": "result", "ok": BOOL, "errs": ["BAD", "WORSE"], "size": 4}
    assert desc_for(result, "Result<Bool,E>").size == 4
    assert descriptor_for(result, "Result<Bool,Free>", None)[1] == parse_type(
        "Result<Bool,Free>", None
    )
    assert code_of(descriptor_for, result, "Result<U8,E>", TYPES) == "descriptor"
    assert code_of(descriptor_for, result, "Option<Bool>", TYPES) == "descriptor"


def test_a_variant_descriptor_fits_the_declared_variant():
    node = {"d": "variant", "ctors": [["Dot", []], ["One", [BOOL]]], "size": 3}
    assert desc_for(node, "Shape").size == 3
    renamed = {"d": "variant", "ctors": [["Dot", []], ["Two", [BOOL]]], "size": 3}
    assert code_of(descriptor_for, renamed, "Shape", TYPES) == "descriptor"
    wrong_arity = {
        "d": "variant",
        "ctors": [["Dot", []], ["One", [BOOL, BOOL]]],
        "size": 5,
    }
    assert code_of(descriptor_for, wrong_arity, "Shape", TYPES) == "descriptor"
    wrong_field = {"d": "variant", "ctors": [["Dot", []], ["One", [U8]]], "size": 257}
    assert code_of(descriptor_for, wrong_field, "Shape", TYPES) == "descriptor"
    three = {
        "d": "variant",
        "ctors": [["Dot", []], ["One", [BOOL]], ["X", []]],
        "size": 4,
    }
    assert code_of(descriptor_for, three, "Shape", TYPES) == "descriptor"
    assert code_of(descriptor_for, node, "Bool", TYPES) == "descriptor"


def test_a_bad_type_text_or_an_unknown_named_type_is_a_descriptor_refusal():
    assert code_of(descriptor_for, BOOL, "Nope", TYPES) == "descriptor"
    assert code_of(descriptor_for, BOOL, 7, TYPES) == "descriptor"


@pytest.mark.parametrize(
    ("node", "text"),
    [
        (BOOL, "Bool"), (U8, "U8"), (leaf("u64_edge", 377), "U64"), (leaf("nat_0_64", 65), "Nat"),
        (NAT3, "Nat"), (leaf("nat_shift", 73), "Nat"), (leaf("bytes_d1", 347), "Bytes"),
        (leaf("bytes_n", 16, n=2), "Bytes<2>"), (leaf("fx_full", 256, t="I8"), "I8"),
        (leaf("fx_edge", 281, t="U16"), "U16"),
    ],
)  # fmt: skip
def test_every_leaf_fits_exactly_its_own_type(node, text):
    assert is_descriptor_for(node, parse_type(text, {}), {})
    assert not is_descriptor_for(node, Ty("Result", (Ty("Bool"), Ty("E"))), {})


def test_the_semantics_supplies_the_fx_edge_values():
    class Narrow(type(STANDARD)):
        def fx_edge_values(self, ty):
            return [1, 2, 3]

    assert build(leaf("fx_edge", 3, t="U16"), Narrow()).size == 3
