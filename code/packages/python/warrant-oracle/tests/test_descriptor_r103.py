"""CV2-F17 / VER04r section 16.20 R103: the consumer-side checks of a ``tuple`` and a ``variant`` descriptor.

``descriptor_size`` refuses, as ``descriptor`` (the oracle's ``generation``, as ``errs`` of R72 does):

* a ``variant`` with fewer than 2 or more than 64 constructors, a constructor name that is not a string or is
  outside the name grammar of section 2.2 rule 4, a repeated name, a constructor with more than 8 fields;
* a ``tuple`` with no component or with more than 32.

Where a ``types`` table is present (:func:`.conformance._check_errs`, the corpus reader), the constructor list must
equal the declared one (names, order, field counts) and a record's ``tuple`` must have exactly its field count.
"""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle import domains as D
from coding_adventures_warrant_oracle.errors import OracleError

BOOL = {"d": "bool", "size": 2}


def tuple_of(n: int) -> dict:
    return {"d": "tuple", "of": [BOOL] * n, "size": 2**n}


def ctor(name, fields: int = 0) -> list:
    return [name, [BOOL] * fields]


def variant(ctors: list) -> dict:
    size = sum(2 ** len(fields) for _, fields in ctors if isinstance(fields, list))
    return {"d": "variant", "ctors": ctors, "size": size}


def refused(d: dict) -> OracleError:
    with pytest.raises(OracleError) as err:
        D.descriptor_size(d)
    assert err.value.code == "generation"
    return err.value


# ---------------------------------------------------------------------------------------------------
# tuple: 1 to 32 entries
# ---------------------------------------------------------------------------------------------------


def test_a_tuple_descriptor_of_one_entry_is_accepted_and_none_is_refused():
    assert D.descriptor_size({"d": "tuple", "of": [BOOL], "size": 2}) == 2
    refused({"d": "tuple", "of": [], "size": 1})


def test_a_tuple_descriptor_of_32_entries_is_accepted_and_33_is_refused():
    assert D.descriptor_size(tuple_of(32)) == 2**32
    assert "at most 32" in refused(tuple_of(33)).message


def test_a_tuple_of_entries_that_is_not_a_list_is_refused():
    refused({"d": "tuple", "of": {"d": "bool", "size": 2}, "size": 2})


# ---------------------------------------------------------------------------------------------------
# variant: 2 to 64 constructors, names, duplicates, 0 to 8 fields
# ---------------------------------------------------------------------------------------------------


def test_a_variant_of_two_constructors_is_accepted_and_one_or_none_is_refused():
    assert D.descriptor_size(variant([ctor("A"), ctor("B")])) == 2
    assert "2 to 64" in refused(variant([ctor("A")])).message
    refused({"d": "variant", "ctors": [], "size": 0})


def test_a_variant_of_64_constructors_is_accepted_and_65_is_refused():
    assert D.descriptor_size(variant([ctor(f"C{i}") for i in range(64)])) == 64
    assert "2 to 64" in refused(variant([ctor(f"C{i}") for i in range(65)])).message


def test_a_constructor_of_8_fields_is_accepted_and_9_is_refused():
    assert D.descriptor_size(variant([ctor("A", 8), ctor("B")])) == 257
    assert "0 to 8" in refused(variant([ctor("A", 9), ctor("B")])).message


@pytest.mark.parametrize("name", [1, None, ["A"], {"n": "A"}, b"A"])
def test_a_constructor_name_that_is_not_a_string_is_refused(name):
    assert "name grammar" in refused(variant([[name, []], ctor("B")])).message


@pytest.mark.parametrize("name", ["", "9A", "A-B", "A B", "A.B", "é", "A\n"])
def test_a_constructor_name_outside_the_grammar_is_refused(name):
    assert "name grammar" in refused(variant([[name, []], ctor("B")])).message


@pytest.mark.parametrize("name", ["A", "_", "_a9", "Z" * 200])
def test_a_constructor_name_of_the_grammar_is_accepted(name):
    assert D.descriptor_size(variant([[name, []], ctor("B")])) == 2


def test_a_repeated_constructor_name_is_refused_wherever_it_repeats():
    assert "repeats" in refused(variant([ctor("A"), ctor("A")])).message
    assert "repeats" in refused(variant([ctor("A"), ctor("B"), ctor("A", 1)])).message


def test_a_constructor_entry_that_is_not_a_name_and_a_field_list_is_refused():
    for bad in (["A"], ["A", [], []], "AB", ["A", "x"]):
        refused({"d": "variant", "ctors": [bad, ctor("B")], "size": 2})


def test_the_field_descriptors_of_a_constructor_are_still_checked_recursively():
    wrong = {"d": "bool", "size": 3}
    refused(variant([["A", [wrong]], ctor("B")]))


def test_the_kit_reader_reports_each_refusal_as_a_descriptor_error():
    """``desc_from_json`` maps the oracle's ``generation`` to the kit code ``descriptor`` (section 11).

    The type is a *named* one, which ``_fits`` accepts for any variant descriptor, so only the new checks of
    ``descriptor_size`` can refuse: on the base source each of these built a descriptor.
    """
    named = A.Ty("V")
    for bad in (variant([ctor("A")]), variant([ctor("A"), ctor("A")]), variant([ctor(f"C{i}") for i in range(65)])):
        with pytest.raises(C.Refused) as err:
            C.desc_from_json(bad, named)
        assert err.value.code == "descriptor"
    assert C.desc_from_json(variant([ctor("A"), ctor("B")]), named).size() == 2


# ---------------------------------------------------------------------------------------------------
# with a ``types`` table: the declared constructors and the arity of a record
# ---------------------------------------------------------------------------------------------------

TYPES = {
    "P": {"kind": "record", "fields": [["x", "Bool"], ["y", "Bool"]]},
    "V": {"kind": "variant", "ctors": [["Dot", []], ["Line", ["Bool"]], ["Box", ["Bool", "Bool"]]]},
    "W": {"kind": "variant", "ctors": [["Hold", ["P"]], ["Free", []]]},
}


def declared_variant() -> dict:
    return variant([ctor("Dot"), ctor("Line", 1), ctor("Box", 2)])


def check(d: dict, text: str) -> None:
    D.descriptor_size(d)
    C._check_errs(d, C.parse_type(text, TYPES), TYPES)


def test_a_variant_descriptor_equal_to_the_declared_constructors_is_accepted():
    check(declared_variant(), "V")


@pytest.mark.parametrize(
    "ctors",
    [
        [ctor("Dot"), ctor("Line", 1), ctor("Box", 1)],  # a field count differs
        [ctor("Dot"), ctor("Line", 1), ctor("Boxx", 2)],  # a name differs
        [ctor("Line", 1), ctor("Dot"), ctor("Box", 2)],  # the order differs
        [ctor("Dot"), ctor("Line", 1)],  # a constructor is missing
        [ctor("Dot"), ctor("Line", 1), ctor("Box", 2), ctor("Extra")],  # a constructor is added
        [ctor("Dot", 1), ctor("Line", 1), ctor("Box", 2)],  # a field is added
    ],
)
def test_a_variant_descriptor_that_differs_from_the_declared_constructors_is_refused(ctors):
    with pytest.raises(C.Refused) as err:
        check(variant(ctors), "V")
    assert err.value.code == "corpus" and "R103" in err.value.why


def test_a_record_tuple_must_have_exactly_the_field_count():
    check({"d": "tuple", "of": [BOOL, BOOL], "size": 4}, "P")
    for n in (1, 3):
        with pytest.raises(C.Refused) as err:
            check(tuple_of(n), "P")
        assert "R103" in err.value.why


def test_the_walk_reaches_a_variant_below_a_record_field_and_an_option():
    inner = {"d": "tuple", "of": [declared_variant(), BOOL], "size": 8 * 2}
    # not a W: a record of (V, Bool) written as a type of the table
    types = {**TYPES, "Q": {"kind": "record", "fields": [["v", "V"], ["b", "Bool"]]}}
    C._check_errs(inner, C.parse_type("Q", types), types)
    bad = {"d": "tuple", "of": [variant([ctor("Dot"), ctor("Box", 2)]), BOOL], "size": 5 * 2}
    with pytest.raises(C.Refused):
        C._check_errs(bad, C.parse_type("Q", types), types)
    wrapped = {"d": "option", "of": declared_variant(), "size": 1 + declared_variant()["size"]}
    C._check_errs(wrapped, A.Ty("Option", (A.Ty("V"),)), TYPES)
    with pytest.raises(C.Refused):
        C._check_errs(
            {"d": "option", "of": variant([ctor("Dot"), ctor("Box", 2)]), "size": 6},
            A.Ty("Option", (A.Ty("V"),)),
            TYPES,
        )


def corpus_refused(d: dict, text: str, types: dict = TYPES) -> None:
    with pytest.raises(C.Refused) as err:
        C._check_errs(d, C.parse_type(text, types), types)
    assert err.value.code == "corpus"


def test_a_tuple_arity_that_differs_from_a_tuple_type_is_refused_also_under_an_option():
    ok = {"d": "tuple", "of": [BOOL, BOOL], "size": 4}
    check(ok, "(Bool,Bool)")
    check({"d": "option", "of": ok, "size": 5}, "Option<(Bool,Bool)>")
    for n in (1, 3):
        corpus_refused(tuple_of(n), "(Bool,Bool)")
        corpus_refused({"d": "option", "of": tuple_of(n), "size": 2**n + 1}, "Option<(Bool,Bool)>")


@pytest.mark.parametrize(
    ("descriptor", "text"),
    [
        (variant([ctor("A"), ctor("B")]), "P"),  # a variant descriptor for a record
        (tuple_of(3), "V"),  # a tuple descriptor for a variant
        (BOOL, "P"),  # a leaf for a record
        (BOOL, "V"),  # a leaf for a variant
        (BOOL, "(Bool,Bool)"),  # a leaf for a tuple
        ({"d": "option", "of": BOOL, "size": 3}, "Bool"),
        ({"d": "list", "of": BOOL, "size": 15}, "Option<Bool>"),
        ({"d": "result", "ok": BOOL, "errs": ["A"], "size": 3}, "Bool"),
        ({"d": "u8_all", "size": 256}, "Bool"),
    ],
)
def test_a_descriptor_that_is_not_one_for_the_binder_type_is_refused(descriptor, text):
    corpus_refused(descriptor, text)


@pytest.mark.parametrize(
    "types",
    [
        {"P": {"kind": "record", "fields": [["x"]]}},
        {"P": {"kind": "record", "fields": [["x", "Bool", "extra"]]}},
        {"P": {"kind": "record", "fields": "xy"}},
        {"P": {"kind": "record"}},
        {"V": {"kind": "variant", "ctors": [["A"], ["B"]]}},
        {"V": {"kind": "variant", "ctors": [["A", 7], ["B", []]]}},
        {"V": {"kind": "variant", "ctors": 3}},
    ],
)
def test_a_malformed_types_row_is_a_corpus_refusal_not_a_traceback(types):
    name = next(iter(types))
    d = {"d": "tuple", "of": [BOOL], "size": 2} if name == "P" else variant([ctor("A"), ctor("B")])
    with pytest.raises(C.Refused) as err:
        C._check_errs(d, A.Ty(name), types)
    assert err.value.code == "corpus"
