"""Defects the independent TypeScript lane's review found in this lane (CV2-S6a fix).

Both lanes were run on the same crafted inputs; where they disagreed, VER04r decided
which was right.  One test group per disagreement, each built from the spec text:

* 2   the section 2.2 limits of the ``types`` table, in every context that reads it;
* 6   an unknown tag inside a ``match`` arm (section 4.2 rule 7, R69 (d));
* 9   an undefined level name in a ``requires`` (section 4.2 rule 7);
* 10  a claim name that is not a legal path component (section 5.5);
* 1   the static type of a bytes literal is ``Bytes<len>`` (Appendix A, R51, R73);
* 5   a repeated model parameter name;
* 16  a ``tuple`` descriptor against a scalar type is ``descriptor`` (section 11).
"""

from __future__ import annotations

import json

import pytest
from test_v2_chunks import OPS, bare
from test_v2_ir import bump, case, u8, var

from coding_adventures_warrant_corpus.v2 import STANDARD, Program
from coding_adventures_warrant_corpus.v2 import chunks as C
from coding_adventures_warrant_corpus.v2.errors import Refusal
from coding_adventures_warrant_corpus.v2.faults import compile_fault
from coding_adventures_warrant_corpus.v2.ir import DEFINED_LEVELS, check_requires
from coding_adventures_warrant_corpus.v2.kit import (
    outcome_domain,
    outcome_fault,
    outcome_ir,
    outcome_value,
)
from coding_adventures_warrant_corpus.v2.wtypes import (
    BUILTIN_TYPE_NAMES,
    check_types_table,
)

# -- disagreement 2: the types table ---------------------------------------------------------
ERR = {"kind": "error", "cases": ["A", "B"]}


def many(count, make):
    return [make(n) for n in range(count)]


BAD_TABLES = {
    "error with no case": {"E": {"kind": "error", "cases": []}},
    "error with 65 cases": {
        "E": {"kind": "error", "cases": many(65, lambda n: f"C{n}")}
    },
    "error with a duplicate case": {"E": {"kind": "error", "cases": ["A", "A"]}},
    "error with a case named 9": {"E": {"kind": "error", "cases": ["A", "9"]}},
    "error with a case that is not a name": {"E": {"kind": "error", "cases": ["a-b"]}},
    "error with a case that is empty text": {"E": {"kind": "error", "cases": [""]}},
    "error with a case that is not text": {"E": {"kind": "error", "cases": ["A", 7]}},
    "record with no field": {"P": {"kind": "record", "fields": []}},
    "record with 33 fields": {
        "P": {"kind": "record", "fields": many(33, lambda n: [f"f{n}", "U8"])}
    },
    "variant with 1 constructor": {"V": {"kind": "variant", "ctors": [["Only", []]]}},
    "variant with 65 constructors": {
        "V": {"kind": "variant", "ctors": many(65, lambda n: [f"K{n}", []])}
    },
    "constructor with 9 fields": {
        "V": {"kind": "variant", "ctors": [["A", ["U8"] * 9], ["B", []]]}
    },
    "bad field name": {"P": {"kind": "record", "fields": [["1x", "U8"]]}},
    "bad constructor name": {
        "V": {"kind": "variant", "ctors": [["a-b", []], ["B", []]]}
    },
    "constructor named like a built-in": {
        "V": {"kind": "variant", "ctors": [["Bytes", []], ["B", []]]}
    },
    "unknown kind": {"X": {"kind": "union"}},
    # R108: the three checks no consumer made.
    "record containing itself": {"R": {"kind": "record", "fields": [["me", "R"]]}},
    "record containing itself through Option": {
        "R": {"kind": "record", "fields": [["me", "Option<R>"]]}
    },
    "record containing itself through a tuple": {
        "R": {"kind": "record", "fields": [["me", "(U8,R)"]]}
    },
    "record containing itself through Result": {
        "E": ERR,
        "R": {"kind": "record", "fields": [["me", "Result<R,E>"]]},
    },
    "record containing itself through List": {
        "R": {"kind": "record", "fields": [["me", "List<R>"]]}
    },
    "mutually recursive records through Option": {
        "A": {"kind": "record", "fields": [["b", "Option<B>"]]},
        "B": {"kind": "record", "fields": [["a", "Option<A>"]]},
    },
    "a variant containing itself": {
        "V": {"kind": "variant", "ctors": [["Leaf", []], ["Node", ["V"]]]}
    },
    "a three-step chain that closes": {
        "A": {"kind": "record", "fields": [["x", "B"]]},
        "B": {"kind": "record", "fields": [["x", "C"]]},
        "C": {"kind": "variant", "ctors": [["K", ["Option<A>"]], ["L", []]]},
    },
    "record with a repeated field name": {
        "P": {"kind": "record", "fields": [["a", "U8"], ["a", "Bool"]]}
    },
    "variant with a repeated constructor name": {
        "V": {"kind": "variant", "ctors": [["A", []], ["A", ["U8"]]]}
    },
    "field of an undeclared type": {
        "P": {"kind": "record", "fields": [["a", "Ghost"]]}
    },
    "field of an undeclared type under Option": {
        "P": {"kind": "record", "fields": [["a", "Option<Ghost>"]]}
    },
    "constructor field of an undeclared type": {
        "V": {"kind": "variant", "ctors": [["A", ["Ghost"]], ["B", []]]}
    },
    "Result with an undeclared error": {
        "P": {"kind": "record", "fields": [["a", "Result<U8,Ghost>"]]}
    },
    "field with a named Bytes width": {
        "P": {"kind": "record", "fields": [["a", "Bytes<N>"]]}
    },
    "field type that is not a type text": {
        "P": {"kind": "record", "fields": [["a", "U8 U8"]]}
    },
    "row that is not an object": {"X": 7},
}
BAD_TABLES |= {f"type named {name}": {name: ERR} for name in sorted(BUILTIN_TYPE_NAMES)}

GOOD_TABLES = {
    "64 error cases": {"E": {"kind": "error", "cases": many(64, lambda n: f"C{n}")}},
    "32 fields": {
        "P": {"kind": "record", "fields": many(32, lambda n: [f"f{n}", "U8"])}
    },
    "2 constructors": {"V": {"kind": "variant", "ctors": [["A", []], ["B", ["U8"]]]}},
    "64 constructors of 8 fields": {
        "V": {"kind": "variant", "ctors": many(64, lambda n: [f"K{n}", ["U8"] * 8])}
    },
    "a List of a plain type": {"P": {"kind": "record", "fields": [["a", "List<U8>"]]}},
    "a Result with a declared error": {
        "E": ERR,
        "P": {"kind": "record", "fields": [["a", "Result<U8,E>"]]},
    },
    "records that share a field type without a cycle": {
        "Inner": {"kind": "record", "fields": [["x", "U8"]]},
        "A": {"kind": "record", "fields": [["i", "Inner"], ["j", "Option<Inner>"]]},
        "B": {"kind": "variant", "ctors": [["K", ["A", "Inner"]], ["L", []]]},
    },
    "a long chain of records": {
        **{
            f"T{n}": {"kind": "record", "fields": [["next", f"Option<T{n + 1}>"]]}
            for n in range(2000)
        },
        "T2000": {"kind": "record", "fields": [["x", "U8"]]},
    },
    "a name that merely starts like a built-in": {
        "Bytes2": ERR,
        "U8x": ERR,
        "Int_": ERR,
    },
}


@pytest.mark.parametrize("table", BAD_TABLES.values(), ids=list(BAD_TABLES))
def test_the_table_checker_refuses_with_the_code_of_the_context(table):
    for code in ("type", "corpus", "descriptor", "ill-typed"):
        with pytest.raises(Refusal) as caught:
            check_types_table(table, code)
        assert caught.value.code == code


@pytest.mark.parametrize("table", GOOD_TABLES.values(), ids=list(GOOD_TABLES))
def test_a_table_at_every_limit_is_accepted(table):
    check_types_table(table, "type")


def test_a_table_that_is_not_an_object_is_refused():
    with pytest.raises(Refusal):
        check_types_table([], "type")


def value_case(types, type_text="Bool"):
    return {"type": type_text, "types": types, "input_hex": b"true".hex()}


@pytest.mark.parametrize("table", BAD_TABLES.values(), ids=list(BAD_TABLES))
def test_v2_value_refuses_a_bad_table_as_type_even_when_the_type_is_unrelated(table):
    assert outcome_value(value_case(table), STANDARD) == {
        "outcome": "reject",
        "error": "type",
    }


def test_v2_value_reads_the_json_before_the_table():
    # code order (section 11): malformed-json (1) comes before type (2)
    bad = value_case(BAD_TABLES["error with no case"]) | {"input_hex": b"{".hex()}
    assert outcome_value(bad, STANDARD)["error"] == "malformed-json"


@pytest.mark.parametrize("table", GOOD_TABLES.values(), ids=list(GOOD_TABLES))
def test_v2_value_accepts_a_table_at_its_limits(table):
    assert outcome_value(value_case(table), STANDARD)["outcome"] == "accept"


@pytest.mark.parametrize("table", BAD_TABLES.values(), ids=list(BAD_TABLES))
def test_v2_ir_refuses_a_bad_table_even_when_nothing_reaches_it(table):
    got = outcome_ir(case(["lit", True], types=table, subjects={}), STANDARD)
    assert got == {"error": "ill-typed"}


@pytest.mark.parametrize("table", BAD_TABLES.values(), ids=list(BAD_TABLES))
def test_v2_domain_refuses_a_bad_table_as_descriptor(table):
    leaf = {"d": "u8_all", "size": 256}
    got = outcome_domain(
        {"descriptor": leaf, "type": "U8", "types": table, "index": 0}, STANDARD
    )
    assert got == {"error": "descriptor"}


@pytest.mark.parametrize("table", BAD_TABLES.values(), ids=list(BAD_TABLES))
def test_v2_chunk_refuses_a_bad_table_as_corpus(table):
    corpus, row = bare([("b", "U8", {"d": "u8_all", "size": 256})])
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus | {"types": table}, row)
    assert caught.value.code == "corpus"


def fault_row(**edits):
    return {
        "fault": "f",
        "op": "bump",
        "kind": "post",
        "expr": ["var", "result"],
        "requires": ["ir.core"],
    } | edits


def fault_case(**edits):
    op = {"params": [["a", "U8"]], "ret": "U8", "requires": ["ir.core"]}
    return {
        "fault": fault_row(),
        "op": op,
        "types": {},
        "levels": ["ir.core"],
        "args": ["1"],
        "result": "2",
    } | edits


@pytest.mark.parametrize("table", BAD_TABLES.values(), ids=list(BAD_TABLES))
def test_a_fault_over_a_bad_table_is_refused_as_corpus(table):
    assert outcome_fault(fault_case(types=table), STANDARD) == {"error": "corpus"}


def test_a_good_fault_still_runs():
    assert outcome_fault(fault_case(), STANDARD) == {"result": "2"}


# -- disagreement 6: tag names are checked before levels, and inside a match arm -------------
def match_on(*arm_bodies):
    return ["match", var("a"), [[["wild"], body] for body in arm_bodies]]


@pytest.mark.parametrize(
    "tree",
    [
        match_on(["frobnicate", var("a")]),
        match_on(u8(1), ["frobnicate"]),
        match_on(["if", ["lit", True], u8(1), ["frobnicate"]]),
        ["tuple", [u8(1), ["frobnicate"]]],
        ["ctor", "K", [["frobnicate"]]],
        ["match", ["frobnicate"], [[["wild"], u8(1)]]],
    ],
)
def test_an_unknown_tag_inside_a_construct_of_a_missing_level_is_unknown_tag(tree):
    # `match`, `tuple` and `ctor` are tags of levels this runner lacks.  The tag-name
    # check wins over level scoping (section 4.2 rule 7, R69 (d)).
    assert outcome_ir(case(tree), STANDARD) == {"error": "unknown-tag"}


def test_a_match_with_only_known_tags_is_still_level_missing():
    assert outcome_ir(case(match_on(u8(1))), STANDARD) == {"error": "level-missing"}
    # a pattern is not an expression: its tag (`ok`, `bind`, ...) is never a tag check
    pat = ["match", var("a"), [[["bind", "x"], u8(1)], [["zzz"], u8(2)]]]
    assert outcome_ir(case(pat), STANDARD) == {"error": "level-missing"}


def test_an_unknown_tag_in_a_match_arm_of_a_model_body_is_unknown_tag_too():
    models = {"m": {"params": [["a", "U8"]], "body": match_on(["frobnicate"])}}
    got = outcome_ir(
        case(["lit", True], models=models, model_order=["m"], subjects={}), STANDARD
    )
    assert got == {"error": "unknown-tag"}


def test_a_subject_call_in_a_match_arm_of_a_model_body_is_subject_forbidden():
    models = {"m": {"params": [["a", "U8"]], "body": match_on(bump())}}
    got = outcome_ir(
        case(["lit", True], models=models, model_order=["m"], subjects={}), STANDARD
    )
    assert got == {"error": "subject-forbidden"}


def test_a_match_arm_counts_toward_the_depth_limit():
    deep = u8(1)
    for _ in range(64):
        deep = ["not", deep]
    assert outcome_ir(case(match_on(deep)), STANDARD) == {"error": "depth"}


# -- disagreement 9: an undefined level name --------------------------------------------------
def test_the_defined_levels_are_the_seven_of_section_4_1():
    assert {
        "ir.core",
        "ir.fixed",
        "ir.tuple",
        "ir.data",
        "ir.list",
        "ir.seq",
        "ir.inout.records",
    } == DEFINED_LEVELS


@pytest.mark.parametrize(
    "bad", [["ir.bogus"], ["ir.core", "ir.Core"], "ir.core", [1], None]
)
def test_check_requires_refuses_what_is_not_a_list_of_defined_levels(bad):
    with pytest.raises(Refusal) as caught:
        check_requires(bad, "corpus", "x")
    assert caught.value.code == "corpus"


def test_check_requires_accepts_reserved_levels_too():
    assert check_requires(["ir.list", "ir.seq"], "corpus", "x") == ["ir.list", "ir.seq"]


@pytest.mark.parametrize(
    "levels", [["ir.bogus"], ["ir.core", "ir.bogus"], ["ir.fixed", "x"]]
)
def test_a_suite_requires_with_an_undefined_level_is_corpus_never_level_missing(levels):
    corpus, row = bare([("b", "U8", {"d": "u8_all", "size": 256})])
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus, row | {"requires": levels})
    assert caught.value.code == "corpus"


def test_a_suite_requires_with_a_defined_but_missing_level_is_unchanged():
    corpus, row = bare([("b", "U8", {"d": "u8_all", "size": 256})])
    # defined, so not an error of its own; the row reaches nothing beyond ir.core
    C.build_suite(corpus, row | {"requires": ["ir.core", "ir.fixed"]})


@pytest.mark.parametrize("levels", [["ir.bogus"], ["ir.core", "ir.bogus"]])
def test_a_fault_requires_with_an_undefined_level_is_corpus_never_level_missing(levels):
    got = outcome_fault(fault_case(fault=fault_row(requires=levels)), STANDARD)
    assert got == {"error": "corpus"}
    # `level-missing` is for a defined level the runner lacks
    got = outcome_fault(fault_case(fault=fault_row(requires=["ir.fixed"])), STANDARD)
    assert got == {"error": "level-missing"}


def test_an_op_requires_with_an_undefined_level_is_refused():
    ops = {"bump": {"params": [["a", "U8"]], "ret": "U8", "requires": ["ir.bogus"]}}
    assert outcome_ir(case(["lit", True], ops=ops), STANDARD) == {"error": "ill-typed"}
    # and as a corpus error when a suite reads the ops
    corpus, row = bare([("b", "U8", {"d": "u8_all", "size": 256})])
    bad = {"f": OPS["f"] | {"requires": ["ir.bogus"]}}
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus | {"ops": bad}, row)
    assert caught.value.code == "corpus"


def test_compile_fault_refuses_an_undefined_level_before_the_level_check():
    op = {"params": [["a", "U8"]], "ret": "U8", "requires": ["ir.core"]}
    with pytest.raises(Refusal) as caught:
        compile_fault(fault_row(requires=["ir.nope"]), op, {})
    assert caught.value.code == "corpus"


# -- disagreement 10: claim names (section 5.5) ---------------------------------------------------
LEGAL_CLAIMS = ["claim", "a/b", "_x1", "A" * 64, "a" * 64 + "/" + "b" * 64]
ILLEGAL_CLAIMS = [
    "a/b/c",
    "../x",
    "..",
    "a/..",
    "",
    "/",
    "a/",
    "/a",
    "a" * 65,
    "a" * 64 + "/" + "b" * 65,
    "1a",
    "a b",
    "a.b",
    "a\\b",
    "a\x00b",
    ".hidden",
    "café",
    "con",
    "a/NUL",
    "com1/x",
    "LPT9",
    7,
    None,
    ["a"],
]


@pytest.mark.parametrize("claim", LEGAL_CLAIMS)
def test_a_legal_claim_derives_its_file_name(claim):
    assert C.chunk_file_name(claim) == claim.replace("/", ".") + ".txt"


def test_the_longest_legal_claim_is_a_133_byte_file_name():
    assert len(C.chunk_file_name("a" * 64 + "/" + "b" * 64)) == C.MAX_CHUNK_FILE_NAME


@pytest.mark.parametrize("claim", ILLEGAL_CLAIMS)
def test_an_illegal_claim_is_refused_as_corpus(claim):
    with pytest.raises(Refusal) as caught:
        C.chunk_file_name(claim)
    assert caught.value.code == "corpus"


@pytest.mark.parametrize("claim", ILLEGAL_CLAIMS)
def test_a_suite_row_with_an_illegal_claim_is_corpus(claim):
    corpus, row = bare([("b", "U8", {"d": "u8_all", "size": 256})])
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus, row | {"claim": claim})
    assert caught.value.code == "corpus"


def test_a_two_name_claim_is_a_legal_suite():
    corpus, row = bare([("b", "U8", {"d": "u8_all", "size": 256})])
    C.build_suite(corpus, row | {"claim": "op/post"})


# -- disagreement 1: the static type of a bytes literal ---------------------------------------
def hexlit(text):
    return ["lit", {"type": "Bytes", "value": text}]


@pytest.mark.parametrize(
    ("tree", "want"),
    [
        (hexlit("abcd"), "Bytes<2>"),
        (hexlit(""), "Bytes<0>"),
        (["concat", hexlit("ab"), hexlit("cdef")], "Bytes<3>"),
        (["if", ["lit", True], hexlit("abcd"), hexlit("0123")], "Bytes<2>"),
        (["if", ["lit", True], hexlit("ab"), hexlit("0123")], "Bytes"),
        (["take", hexlit("abcd"), ["lit", {"type": "Nat", "value": "1"}]], "Bytes"),
    ],
)
def test_expect_type_is_the_static_type_not_the_written_one(tree, want):
    got = outcome_ir(case(tree, subjects={}), STANDARD)
    assert got["type"] == want


def test_a_bytes_variable_reports_its_declared_width():
    got = outcome_ir(
        case(
            var("k"),
            env=[["k", {"type": "Bytes<2>", "value": "abcd"}]],
            subjects={},
        ),
        STANDARD,
    )
    assert got["type"] == "Bytes<2>"


# -- disagreement 5: duplicate model parameter names ------------------------------------------
def test_a_model_with_two_parameters_of_one_name_is_ill_typed():
    models = {
        "m": {"params": [["a", "U8"], ["a", "U8"]], "body": var("a")},
    }
    got = outcome_ir(
        case(["lit", True], models=models, model_order=["m"], subjects={}), STANDARD
    )
    assert got == {"error": "ill-typed"}
    # even when nothing calls it, and when the types differ
    models = {"m": {"params": [["a", "U8"], ["a", "Bool"]], "body": ["lit", True]}}
    with pytest.raises(Refusal) as caught:
        Program(models=models, model_order=["m"])
    assert caught.value.code == "ill-typed"


def test_distinct_parameter_names_are_fine():
    models = {"m": {"params": [["a", "U8"], ["b", "U8"]], "body": var("b")}}
    Program(models=models, model_order=["m"])


# -- disagreement 16: a tuple descriptor against a scalar type ---------------------------------
def test_a_tuple_descriptor_against_a_scalar_type_is_descriptor():
    leaf = {"d": "u8_all", "size": 256}
    tup = {"d": "tuple", "of": [leaf, leaf], "size": 65536}
    # section 11, `v2-domain`: `descriptor` is "a descriptor that is not one for `type`"
    for type_text in ("U8", "Bool", "Nat", "Bytes"):
        got = outcome_domain(
            {"descriptor": tup, "type": type_text, "index": 0}, STANDARD
        )
        assert got == {"error": "descriptor"}


def test_the_json_of_the_bad_tables_is_plain_json():
    # the kit stores tables as JSON; make sure the fixtures above round-trip
    for table in BAD_TABLES.values():
        assert json.loads(json.dumps(table)) == table


# -- duplicate op parameter names (section 2.2, as for model parameters) ---------------------------
DUPLICATE_OP = {
    "dup": {"params": [["a", "U8"], ["a", "U8"]], "ret": "U8", "requires": ["ir.core"]}
}


def test_an_op_with_a_repeated_parameter_name_is_ill_typed_in_kit_ir():
    ir = ["subject_op", "dup", [u8(1), u8(2)]]
    assert outcome_ir(case(ir, ops=DUPLICATE_OP), STANDARD) == {"error": "ill-typed"}


def test_an_op_with_a_repeated_parameter_name_is_corpus_in_a_corpus():
    corpus, row = bare([("b", "U8", {"d": "u8_all", "size": 256})])
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus | {"ops": {"f": DUPLICATE_OP["dup"]}}, row)
    assert caught.value.code == "corpus"


# -- the body of a reference suite (section 4.3) ---------------------------------------------------
def reference_suite():
    from test_v2_chunks import SUBJECT

    corpus, row = bare(
        [("b", "U8", {"d": "u8_all", "size": 256})],
        expected=["model_call", "m", [["var", "b"]]],
        result_type="U8",
    )
    return corpus, row, SUBJECT, row["oracle"]["expected"]


def test_a_reference_body_is_eq_of_subject_and_expected_in_either_order():
    corpus, row, subject, expected = reference_suite()
    C.build_suite(corpus, row)
    swapped = row | {"body": ["eq", expected, subject]}
    assert C.build_suite(corpus, swapped).expected is not None


@pytest.mark.parametrize(
    "make_body",
    [
        lambda subject, expected: ["ne", subject, expected],
        lambda subject, expected: ["eq", subject, subject],
        lambda subject, expected: ["eq", subject, ["lit", True]],
        lambda subject, expected: ["lit", True],
        lambda subject, expected: ["not", ["ne", subject, expected]],
        lambda subject, expected: ["eq", subject],
        lambda subject, expected: ["eq", subject, expected, expected],
        lambda subject, expected: "eq",
    ],
    ids=[
        "ne",
        "subject twice",
        "literal",
        "constant",
        "not ne",
        "short",
        "long",
        "text",
    ],
)
def test_a_reference_body_of_any_other_shape_is_corpus(make_body):
    corpus, row, subject, expected = reference_suite()
    with pytest.raises(Refusal) as caught:
        C.build_suite(corpus, row | {"body": make_body(subject, expected)})
    assert caught.value.code == "corpus"


def test_a_predicate_suite_body_is_not_constrained_by_the_reference_rule():
    corpus, row = bare([("b", "U8", {"d": "u8_all", "size": 256})], body=["lit", True])
    assert C.build_suite(corpus, row).expected is None
