"""S10d (VER04r section 13, R9, R17, R67, R76, R96 to R98, R100, R110, R111, R119 to R121, R125): the kit vectors at
``ir.fixed`` and ``ir.tuple``, and the two things the oracle had to learn for them.

The case tables of S10d carry the ``requires`` they expect (``Row.requires``) and the build refuses a row whose computed
``requires`` differs (``finish_case``).  This file reads the ``kit`` fixture (``conftest.py``) and pins what each group of
vectors is, so that a table edit that drops a vector or changes an outcome fails here and not silently in a lane.

* the 958 cases of S5 are byte for byte what they were (a digest of their canonical documents per category);
* every S10d case names ``ir.fixed`` or ``ir.tuple`` in its ``requires`` except 57 that the computed rule leaves at
  ``ir.core`` (a ``type`` refusal has no type to level, a ``nat_shift`` or ``nat_0_64`` leaf is a ``Nat`` and the binder type is
  one too, and the vectors of R111, R119 to R121 and R125 are ``ir.core`` forms): the allowlist below is exact, and a lane that
  implements ``ir.core`` runs those 57;
* the vectors the section 13 row names are present with the outcome the spec states;
* a wrong evaluator fails an S10d case (the mutants below), and the two changes made to the oracle have tests.
"""

from __future__ import annotations

import hashlib
import json

import pytest

from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle import domains as D
from coding_adventures_warrant_oracle import ir_eval as E
from coding_adventures_warrant_oracle import levels as L
from coding_adventures_warrant_oracle.corpus_v2 import canonical
from coding_adventures_warrant_oracle.irtree import children, is_node

CATEGORIES = ("v2-value", "v2-arith", "v2-ir", "v2-domain", "v2-chunk", "v2-fault")
FIXED_OR_TUPLE = {"ir.fixed", "ir.tuple"}

#: ``(cases, sha256 of the canonical documents concatenated in order)`` of the S5 cases of each category, as committed before S10d.
S5_DIGESTS = {
    "v2-value": (128, "70374742e7cf7d9af9e5aaf9969dc5381fd93d65f030cd347135017d998e227c"),
    "v2-arith": (129, "4af24538ea8a717952c22f990cbb2c8b930293d414f70f94ee20afab83b812a7"),
    "v2-ir": (248, "e3fc7fc139bf80373b8a9842607423945b70da25b4513140218264b4dc724044"),
    "v2-domain": (162, "def878c71c874c5a3bc67a260b0a0ee2ed5dcf6076e0e7f3e7567936982e4d37"),
    "v2-chunk": (74, "cc1a9ff92fef2f1b1617d904f0f09bf2d06b42df895fca86ab32846ee9250c31"),
    "v2-fault": (41, "ef41affcfe104f52d64bdbd153a5c30e6ff4e33650d8f9069042d1bd2fa9889f"),
}
#: How many cases S10d added to each category (670; ``v2-arith`` has none: its closed ``op`` set is ``ir.core`` only).
S10D_COUNTS = {"v2-value": 49, "v2-arith": 0, "v2-ir": 469, "v2-domain": 57, "v2-chunk": 23, "v2-fault": 72}

#: The S10d cases whose computed ``requires`` is ``["ir.core"]``, and why.
CORE_ONLY = {
    # a ``type`` refusal: the text is no type, so there is no level to compute
    "v2-value": {
        "tuple-type-depth-17-nested-tuples", "tuple-type-depth-17-through-mixed-wrappers", "tuple-type-of-one-component",
        "tuple-type-of-nine-components", "tuple-type-empty", "tuple-type-with-int", "tuple-type-with-an-unknown-name",
        "tuple-type-unclosed", "tuple-type-trailing-comma", "tuple-type-with-a-space",
    },
    # ``nat_shift`` is a leaf over Nat (what selects it is an ir.fixed count operand in a claim, which a v2-domain case does not hold)
    "v2-domain": {
        "nat-shift-index-0", "nat-shift-range-first-two", "nat-shift-range-last-two", "nat-shift-index-last", "nat-shift-index-size",
        "nat-shift-at-the-width-boundaries", "nat-shift-in-full", "nat-nat-count-64-is-not-nat-shift", "descriptor-nat-shift-wrong-size",
        "descriptor-nat-shift-with-a-parameter", "descriptor-nat-shift-missing-size", "descriptor-for-another-type-nat-shift-for-u8",
        "descriptor-for-another-type-nat-shift-for-bytes",
    },
    # R125: the ops and models of a chunk corpus part, over the ir.core suite ``ragged``
    "v2-chunk": {
        "corpus-op-int-parameter", "corpus-op-int-result", "corpus-op-error-type-as-a-parameter", "corpus-op-undeclared-type",
        "corpus-op-unparseable-type", "corpus-model-parameter-names-an-error-type",
        "corpus-model-order-omits-a-model", "corpus-model-int-parameter-is-accepted",
    },
    "v2-ir": {
        # R111 (d), [D75]: a model with an Int parameter; an op with one is ill-typed whether or not ir calls it (R122)
        "int-model-parameter-compared-with-an-int-literal", "int-model-parameter-negative-argument",
        "int-model-parameter-with-a-nat-operand", "ill-typed-op-int-parameter-never-called",
        "ill-typed-op-int-result-never-called",
        # R120 (d): an env type the reader refuses
        "ill-typed-env-entry-of-type-int", "ill-typed-env-entry-of-an-undeclared-type",
        "ill-typed-env-entry-result-with-a-record-as-error", "ill-typed-tuple-type-seventeen-deep-env",
        # R76: at levels ir.core a match is level-missing whatever it matches
        "level-missing-match-on-a-result-at-the-core-level",
        # R100: a stand-in of an op nothing reaches counts for nothing
        "stand-in-of-an-op-nothing-reaches-counts-for-nothing",
    },
    "v2-fault": {
        # R96 to R98, R110: a row ill-typed at ir.core alone
        "ill-typed-r98-if-branch-ok-true-against-a-u8-result", "ill-typed-not-of-a-u8-result", "ill-typed-bool-for-a-u8-result",
        "ill-typed-bytes-for-a-u8-result", "ill-typed-bool-for-a-bytes-parameter",
        # R121: the op row is read with the table
        "ill-typed-op-row-int-parameter", "ill-typed-op-row-int-result", "ill-typed-op-row-error-type-as-a-parameter",
        "ill-typed-op-row-undeclared-type",
        # R119 (e): unknown-tag, depth, model-order
        "unknown-tag-in-a-post-expression", "unknown-tag-in-a-pre-rewrite", "depth-65-nodes-is-refused", "depth-64-nodes-is-accepted",
        "model-order-a-model-call-in-a-fault-expression",
        # a level-missing case at levels ir.core: its requires is its levels
        "level-missing-missing-error-tuple-payload-at-core",
    },
    "v2-arith": set(),
}


def cases_of(kit, category):
    return json.loads(kit.files[f"{category}.json"])["cases"]


def s10d(kit, category):
    """``[(case, row)]`` of the S10d rows of a category: the rows that state a ``requires``."""
    return [(c, r) for c, r in zip(cases_of(kit, category), kit.tables[category]) if r.requires is not None]


def s10d_cases(kit, category):
    return {c["id"].split("/", 1)[1]: c for c, _ in s10d(kit, category)}


def u32le(n):
    return n.to_bytes(4, "little").hex()


def word(v):
    return (v % 2**64).to_bytes(8, "little").hex()


# ---------------------------------------------------------------------------------------------------
# the S5 cases are untouched, and S10d's are counted
# ---------------------------------------------------------------------------------------------------


def test_the_958_cases_of_s5_are_byte_for_byte_what_they_were(kit):
    total = 0
    for category in CATEGORIES:
        old = [c for c, r in zip(cases_of(kit, category), kit.tables[category]) if r.requires is None]
        count, digest = S5_DIGESTS[category]
        assert len(old) == count, category
        assert hashlib.sha256(b"".join(canonical(c) for c in old)).hexdigest() == digest, category
        # and S5's rows are the first rows of the table, in their old order, so no id moved
        assert all(r.requires is None for r in kit.tables[category][:count]), category
        assert all(r.requires is not None for r in kit.tables[category][count:]), category
        total += count
    assert total == 782  # the six categories; the 176 of v2-binding are not touched by any table here


def test_s10d_added_670_cases_in_the_stated_places(kit):
    for category in CATEGORIES:
        assert len(s10d(kit, category)) == S10D_COUNTS[category], category
    assert sum(S10D_COUNTS.values()) == 670


def test_every_s10d_case_names_ir_fixed_or_ir_tuple_except_the_exact_core_only_allowlist(kit):
    for category in CATEGORIES:
        core_only = set()
        for case, _row in s10d(kit, category):
            slug = case["id"].split("/", 1)[1]
            if FIXED_OR_TUPLE & set(case["requires"]):
                assert slug not in CORE_ONLY[category], case["id"]
            else:
                assert case["requires"] == ["ir.core"], case["id"]
                core_only.add(slug)
        assert core_only == CORE_ONLY[category], category
    assert sum(len(v) for v in CORE_ONLY.values()) == 57


def test_a_case_that_is_level_missing_has_its_levels_as_requires(kit):
    for category in ("v2-ir", "v2-fault"):
        seen = 0
        for case, _row in s10d(kit, category):
            if case["expect"].get("error") in ("level-missing", "unknown-tag"):
                assert case["requires"] == sorted(case["levels"]), case["id"]
                seen += 1
        assert seen >= 5, category


# ---------------------------------------------------------------------------------------------------
# the table's expectation of requires is enforced
# ---------------------------------------------------------------------------------------------------


def test_a_row_whose_computed_requires_is_not_the_stated_one_fails_the_build():
    row = C.Row("t", "a tuple value", {"type": "(U8,Bool)", "types": {}, "input_hex": b'["1",true]'.hex()},
                {"outcome": "accept"})  # no requires stated: an S5 row, which must compute ["ir.core"]
    expect = C.derive_value(row.fields)
    with pytest.raises(C.KitError, match="requires is"):
        C.finish_case("v2-value", row, expect)
    stated = C.Row("t", "a tuple value", row.fields, {"outcome": "accept"}, ["ir.core", "ir.tuple"])
    assert C.finish_case("v2-value", stated, expect)["requires"] == ["ir.core", "ir.tuple"]
    wrong = C.Row("t", "a tuple value", row.fields, {"outcome": "accept"}, ["ir.core", "ir.fixed"])
    with pytest.raises(C.KitError, match="requires is"):
        C.finish_case("v2-value", wrong, expect)


# ---------------------------------------------------------------------------------------------------
# v2-value: tuples (stream tag 64)
# ---------------------------------------------------------------------------------------------------


def test_tuple_values_are_tag_64_a_count_and_the_components(kit):
    cases = s10d_cases(kit, "v2-value")
    expect = cases["tuple-pair"]["expect"]
    assert expect["stream_hex"] == "64" + u32le(2) + "10" + word(1) + "01"
    assert cases["tuple-pair"]["requires"] == ["ir.core", "ir.tuple"]
    for n in range(2, 9):  # every arity of section 14
        arity = [c for slug, c in cases.items() if slug == f"tuple-arity-{n}" or (n == 2 and slug == "tuple-pair")]
        assert arity and arity[0]["expect"]["outcome"] == "accept", n
        assert arity[0]["expect"]["stream_hex"].startswith("64" + u32le(n)), n
    nested = cases["tuple-nested-first"]["expect"]["stream_hex"]
    assert nested == "64" + u32le(2) + "64" + u32le(2) + "10" + word(1) + "10" + word(2) + "01"
    assert "stream_hex" not in cases["tuple-with-a-long-bytes-component"]["expect"]
    assert len(cases["tuple-with-a-long-bytes-component"]["expect"]["stream_sha256"]) == 64
    assert cases["tuple-with-an-err-component"]["expect"]["stream_hex"] == (
        "64" + u32le(2) + "51" + u32le(3) + b"BAD".hex() + "01")


def test_tuple_refusals_carry_the_code_of_the_first_defect(kit):
    cases = s10d_cases(kit, "v2-value")
    for slug in ("tuple-too-few-components", "tuple-too-many-components", "tuple-is-an-object", "tuple-component-out-of-range"):
        assert cases[slug]["expect"] == {"outcome": "reject", "error": "value"}, slug
        assert cases[slug]["requires"] == ["ir.core", "ir.tuple"], slug
    for slug in ("tuple-type-of-one-component", "tuple-type-of-nine-components", "tuple-type-empty", "tuple-type-with-int",
                 "tuple-type-depth-17-nested-tuples"):
        assert cases[slug]["expect"] == {"outcome": "reject", "error": "type"}, slug
    assert cases["tuple-malformed-wins-over-value"]["expect"]["error"] == "malformed-json"
    assert cases["tuple-type-depth-16-nested-tuples"]["expect"]["outcome"] == "accept"


# ---------------------------------------------------------------------------------------------------
# v2-domain: tuple and nat_shift
# ---------------------------------------------------------------------------------------------------


def test_nat_shift_is_the_73_values_of_d26(kit):
    cases = s10d_cases(kit, "v2-domain")
    want = list(range(65)) + [65, 127, 128, 129, 255, 256, 65535, 65536]
    assert len(want) == 73
    full = cases["nat-shift-in-full"]["expect"]
    assert full["size"] == 73 and len(full["values_hex"]) == 73
    assert full["values_hex"] == ["10" + word(v) for v in want]
    assert cases["nat-shift-index-size"]["expect"] == {"error": "index"}
    assert cases["nat-nat-count-64-is-not-nat-shift"]["expect"] == {"error": "index"}  # nat_0_64 stops at 64
    for slug in ("descriptor-nat-shift-wrong-size", "descriptor-nat-shift-with-a-parameter", "descriptor-nat-shift-missing-size",
                 "descriptor-for-another-type-nat-shift-for-u8", "descriptor-for-another-type-nat-shift-for-bytes"):
        assert cases[slug]["expect"] == {"error": "descriptor"}, slug


def test_a_tuple_descriptor_varies_its_last_component_fastest(kit):
    cases = s10d_cases(kit, "v2-domain")
    pair = cases["tuple-bool-bool-in-full"]["expect"]["values_hex"]
    assert pair == ["64" + u32le(2) + a + b for a, b in (("00", "00"), ("00", "01"), ("01", "00"), ("01", "01"))]
    across = cases["tuple-bool-u8-across-the-first-component"]["expect"]["values_hex"]
    assert across == ["64" + u32le(2) + "00" + "10" + word(255), "64" + u32le(2) + "01" + "10" + word(0)]
    assert cases["tuple-eight-components-second"]["expect"]["values_hex"] == ["64" + u32le(8) + "00" * 7 + "01"]
    assert cases["tuple-u8-bool-index-size"]["expect"] == {"error": "index"}
    assert cases["tuple-nested-15-accepted"]["expect"]["size"] == 2**16
    assert cases["tuple-nested-16-refused"]["expect"] == {"error": "descriptor"}  # 17 levels
    assert all(c["requires"] == ["ir.core", "ir.tuple"] for slug, c in cases.items() if slug.startswith("tuple-"))


# ---------------------------------------------------------------------------------------------------
# v2-chunk: a v2-stream chunk with a tuple binder
# ---------------------------------------------------------------------------------------------------


def test_a_chunk_with_a_tuple_binder_is_a_warrant_stream_v2_suite(kit):
    cases = s10d_cases(kit, "v2-chunk")
    for slug in ("tuple-pair-binder-chunk", "tuple-triples-middle-chunk", "tuple-nested-first-chunk", "tuple-two-tuple-binders"):
        case = cases[slug]
        assert case["suite"]["stream"] == "warrant-stream-v2", slug
        assert case["requires"] == ["ir.core", "ir.tuple"], slug
        assert case["suite"]["binders"][0][1]["domain"]["d"] in ("tuple", "result"), slug
        assert "error" not in case["expect"], slug
    assert cases["tuple-triples-middle-chunk"]["expect"]["first_admitted"] == 64 * 1024
    assert cases["tuple-triples-middle-chunk"]["also_sequential"] is True
    assert cases["tuple-pair-binder-chunk"]["suite"]["binders"][0][1]["type"] == "(U8,Bool)"
    for slug in ("tuple-chunk-file-first-admitted-off-by-one", "tuple-chunk-file-inputs-digest-wrong"):
        assert cases[slug]["expect"] == {"error": "chunk-file"}
    for slug in ("tuple-corpus-wrong-descriptor-size", "tuple-corpus-descriptor-with-too-few-components",
                 "tuple-corpus-descriptor-component-of-another-type", "tuple-corpus-unknown-key-in-a-tuple-row"):
        assert cases[slug]["expect"] == {"error": "corpus"}


def test_r125_the_ops_and_models_of_a_chunk_corpus_are_read(kit):
    cases = s10d_cases(kit, "v2-chunk")
    refused = [s for s in cases if s.startswith("corpus-op-") or s in ("corpus-model-parameter-names-an-error-type",
                                                                       "corpus-model-order-omits-a-model")]
    assert len(refused) == 7
    assert all(cases[s]["expect"] == {"error": "corpus"} for s in refused)
    assert "error" not in cases["corpus-model-int-parameter-is-accepted"]["expect"]  # [D75]: an Int model parameter is accepted
    assert all(cases[s]["requires"] == ["ir.core"] for s in refused)


# ---------------------------------------------------------------------------------------------------
# v2-fault
# ---------------------------------------------------------------------------------------------------


def test_off_by_one_consumed_on_a_result_of_a_tuple_is_accepted_r96(kit):
    cases = s10d_cases(kit, "v2-fault")
    case = cases["off-by-one-consumed-in-an-ok-even-nat"]
    assert case["op"]["ret"] == "Result<(Bytes,Nat),E>"
    assert case["requires"] == ["ir.core", "ir.fixed", "ir.tuple"] and case["fault"]["fault"] == "off-by-one-consumed"
    assert case["expect"] == {"result": {"ok": ["0a", "5"]}}
    assert cases["off-by-one-consumed-in-an-ok-odd-nat"]["expect"] == {"result": {"ok": ["0a", "4"]}}
    assert cases["off-by-one-consumed-largest-nat-goes-down"]["expect"] == {"result": {"ok": ["0a", str(2**64 - 2)]}}
    assert cases["off-by-one-consumed-leaves-an-err"]["expect"] == {"result": {"err": "WORSE"}}
    assert cases["off-by-one-consumed-signed-component-minimum"]["expect"] == {"result": {"ok": ["0a", "-127"]}}


def test_r96_second_vector_and_the_refusals_of_r98_are_ill_typed(kit):
    cases = s10d_cases(kit, "v2-fault")
    for slug in ("ill-typed-r67-one-byte-literal-in-a-bytes-2-component", "ill-typed-r98-if-branch-ok-true-against-a-u8-result",
                 "ill-typed-r98-wrong-second-match-arm", "ill-typed-r98-tuple-component-if", "ill-typed-not-of-a-u8-result",
                 "ill-typed-bool-for-a-u8-result", "ill-typed-bytes-for-a-u8-result", "ill-typed-bool-for-a-bytes-parameter",
                 "ill-typed-tuple-for-a-scalar-result"):
        assert cases[slug]["expect"] == {"error": "ill-typed"}, slug
    assert cases["ill-typed-r67-one-byte-literal-in-a-bytes-2-component"]["op"]["ret"] == "(Bytes<2>,Bool)"
    assert cases["ill-typed-r98-wrong-second-match-arm"]["requires"] == ["ir.core", "ir.tuple"]


def test_missing_error_with_a_tuple_payload_and_no_bytes_parameter_r67(kit):
    cases = s10d_cases(kit, "v2-fault")
    case = cases["missing-error-tuple-payload-no-bytes-parameter"]
    assert case["op"]["params"] == [["a", "U8"]] and case["op"]["ret"] == "Result<(Bytes,Bool),E>"
    assert case["fault"]["fault"] == "missing-error" and case["fault"]["requires"] == ["ir.tuple"]
    assert case["expect"] == {"result": {"ok": ["", False]}}
    assert case["fault"]["expr"][2] == ["ok", ["tuple", [["lit", {"type": "Bytes", "value": ""}], ["lit", False]]]]
    assert cases["missing-error-tuple-payload-sized-bytes-parameter-is-not-a-match"]["expect"] == {"result": {"ok": ["", True]}}


def test_the_three_static_codes_and_the_op_row_r119_r121(kit):
    cases = s10d_cases(kit, "v2-fault")
    assert cases["unknown-tag-in-a-post-expression"]["expect"] == {"error": "unknown-tag"}
    assert cases["depth-65-nodes-is-refused"]["expect"] == {"error": "depth"}
    assert cases["depth-64-nodes-is-accepted"]["expect"] == {"result": "0a" * 64}
    assert cases["model-order-a-model-call-in-a-fault-expression"]["expect"] == {"error": "model-order"}
    for slug, case in cases.items():
        if slug.startswith("ill-typed-op-row-"):
            assert case["expect"] == {"error": "ill-typed"}, slug
    assert sum(1 for s in cases if s.startswith("ill-typed-op-row-")) == 4
    codes = {c["expect"].get("error") for c in cases.values()}
    assert {"ill-typed", "unknown-tag", "depth", "model-order", "level-missing"} <= codes


def test_the_ir_fixed_templates_and_their_levels(kit):
    cases = s10d_cases(kit, "v2-fault")
    assert cases["xor-lsb-u8"]["expect"] == {"result": "7"} and cases["xor-lsb-u8"]["requires"] == ["ir.core", "ir.fixed"]
    assert cases["ignore-high-bit-clears-bit-63"]["expect"] == {"args": [str(2**63 - 1)]}
    assert cases["flip-result-bit-empty-bytes"]["expect"] == {"result": "01"}
    assert cases["flip-result-bit-ok-bytes"]["requires"] == ["ir.core", "ir.fixed", "ir.tuple"]
    assert cases["drop-last-result-byte-ok-form"]["requires"] == ["ir.core", "ir.tuple"]
    assert cases["drop-last-result-byte-ok-form"]["expect"] == {"result": {"ok": "0a0b"}}
    assert cases["ok-as-err-first-case"]["expect"] == {"result": {"err": "BAD"}}
    assert cases["drop-consumed-in-an-ok"]["expect"] == {"result": {"ok": ["0a", "0"]}}
    assert cases["swap-components-bare-pair"]["expect"] == {"result": ["2", "1"]}
    assert cases["pre-rewrites-a-tuple-parameter"]["expect"] == {"args": [["2", "1"]]}
    for slug, levels in (("level-missing-flip-result-bit-ok-form-without-tuple", ["ir.core", "ir.fixed"]),
                         ("level-missing-off-by-one-consumed-without-fixed", ["ir.core", "ir.tuple"]),
                         ("level-missing-off-by-one-consumed-without-tuple", ["ir.core", "ir.fixed"])):
        assert cases[slug]["expect"] == {"error": "level-missing"} and cases[slug]["levels"] == levels, slug


# ---------------------------------------------------------------------------------------------------
# v2-ir
# ---------------------------------------------------------------------------------------------------


def tags_of(ir):
    out, stack = set(), [ir]
    while stack:
        node = stack.pop()
        if is_node(node):
            out.add(node[0])
            stack.extend(children(node))
    return out


def test_every_tag_of_appendix_a1_occurs_in_an_s10d_ir_case(kit):
    seen = set()
    for case, _row in s10d(kit, "v2-ir"):
        seen |= tags_of(case["ir"]).union(*(tags_of(m["body"]) for m in case["models"].values()))
    missing = (set(L.FIXED_TAGS) | set(L.TUPLE_TAGS)) - seen
    assert not missing, sorted(missing)
    assert {"match"} <= seen


def test_r76_the_level_of_match_is_decided_by_the_scrutinee(kit):
    cases = s10d_cases(kit, "v2-ir")
    runs = cases["match-ok-bind-on-ok"]
    assert runs["levels"] == ["ir.core", "ir.tuple"] and runs["expect"]["value"] == "7"
    bare = cases["level-missing-match-on-a-bare-tuple-needs-ir-data"]
    assert bare["expect"] == {"error": "level-missing"} and bare["levels"] == ["ir.core", "ir.tuple"]
    assert bare["requires"] == ["ir.core", "ir.tuple"]
    assert bare["ir"][1] == ["var", "t"] and bare["env"][0][1]["type"] == "(U8,U8)"
    assert cases["level-missing-match-on-a-bool-needs-ir-data"]["expect"] == {"error": "level-missing"}
    core = cases["level-missing-match-on-a-result-at-the-core-level"]
    assert core["expect"] == {"error": "level-missing"} and core["levels"] == ["ir.core"] and core["requires"] == ["ir.core"]
    assert cases["level-missing-match-on-a-result-at-the-fixed-level"]["levels"] == ["ir.core", "ir.fixed"]
    taken_ok = cases["match-only-the-taken-arm-counts-ok"]["expect"]
    taken_err = cases["match-only-the-taken-arm-counts-err"]["expect"]
    assert (taken_ok["calls"], taken_err["calls"], taken_err["value"]) == (0, 1, "6")


def test_r100_a_stand_in_counts_for_requires(kit):
    case = s10d_cases(kit, "v2-ir")["stand-in-body-needs-ir-fixed"]
    assert tags_of(case["ir"]) <= L.CORE_TAGS  # the tree is ir.core
    assert any(tags_of(m["body"]) & L.FIXED_TAGS for m in case["models"].values())  # the stand-in is not
    assert case["levels"] == ["ir.core", "ir.fixed"] and case["requires"] == ["ir.core", "ir.fixed"]
    assert case["expect"] == {"type": "U8", "value": "4", "calls": 1}
    unreached = s10d_cases(kit, "v2-ir")["stand-in-of-an-op-nothing-reaches-counts-for-nothing"]
    assert unreached["requires"] == ["ir.core"] and unreached["expect"]["value"] is True


def test_a_binder_type_levels_a_domain_case_and_an_op_row_has_its_computed_requires(kit):
    """B1: the type of a v2-domain case is levelled like a v2-value type; N1: a hand-written op row carries the requires its signature has."""
    domain = s10d_cases(kit, "v2-domain")
    assert domain["descriptor-scalar-for-a-tuple-type"]["requires"] == ["ir.core", "ir.tuple"]
    assert C.requires_of("v2-domain", {"descriptor": {"d": "bool", "size": 2}, "type": "(Bool,Bool)"}, {}) == ["ir.core", "ir.tuple"]
    assert C.requires_of("v2-domain", {"descriptor": {"d": "bool", "size": 2}, "type": "Frob"}, {}) == ["ir.core"]
    for category in ("v2-ir", "v2-fault"):
        for case, _row in s10d(kit, category):
            rows = case["ops"].values() if category == "v2-ir" else [case["op"]]
            for op in rows:
                tuple_typed = any("(" in t for t in [*(t for _, t in op["params"]), op["ret"]])
                if isinstance(op["ret"], str) and not any(t in op["ret"] for t in ("Int", "Frob")):
                    assert (op["requires"] == ["ir.core", "ir.tuple"]) == tuple_typed, case["id"]


def test_the_ir_fixed_cases_at_16_32_and_64_bits(kit):
    cases = s10d_cases(kit, "v2-ir")
    for t, w in (("u16", 16), ("u32", 32), ("u64", 64), ("i16", 16), ("i32", 32), ("i64", 64)):
        assert cases[f"shl-{t}-by-the-width"]["expect"]["value"] == "0"
        assert cases[f"rotl-{t}-by-width-plus-one"]["expect"]["value"] == "2"
        assert cases[f"clz-{t}-zero"]["expect"]["value"] == str(w)
        assert cases[f"popcount-{t}-all-ones"]["expect"]["value"] == str(w)
        assert cases[f"bit-{t}-index-equal-to-the-width"]["expect"]["value"] is False
        assert cases[f"shr-{t}-top-pattern-by-width-minus-one"]["expect"]["value"] == "1"
    for t, w in (("i16", 16), ("i32", 32), ("i64", 64)):
        assert cases[f"sar-{t}-minimum-by-width-minus-one"]["expect"]["value"] == "-1"
        assert cases[f"div-trunc-{t}-minimum-by-minus-one"]["expect"]["value"] == "99"
        assert cases[f"rem-trunc-{t}-minimum-by-three"]["expect"]["value"] == str(-(2 ** (w - 1) % 3))


def test_the_cost_of_a_match_is_the_maximum_and_an_arm_overflow_has_the_class_of_its_guard(kit):
    cases = s10d_cases(kit, "v2-ir")
    assert cases["cost-match-two-arms-of-1023-are-accepted"]["expect"]["value"] is True
    assert cases["cost-match-the-larger-arm-at-1998-is-accepted"]["expect"]["value"] is True
    assert cases["cost-match-the-larger-arm-at-1999-is-refused"]["expect"] == {"error": "ir-cost"}
    assert cases["overflow-in-the-taken-match-arm-pure-scrutinee-corpus-class"]["expect"] == {"error": "ir-overflow", "class": "corpus"}
    assert cases["overflow-in-the-taken-match-arm-subject-scrutinee-case-class"]["expect"] == {"error": "ir-overflow", "class": "case"}
    assert cases["match-arm-overflow-not-taken-is-a-value"]["expect"]["value"] == "0"
    assert all(c["requires"] == ["ir.core", "ir.tuple"] for s, c in cases.items() if s.startswith(("cost-match-", "overflow-in-the-taken")))


def test_r111_an_int_model_parameter_is_accepted_and_an_int_env_entry_is_not(kit):
    cases = s10d_cases(kit, "v2-ir")
    case = cases["int-model-parameter-compared-with-an-int-literal"]
    assert case["models"]["is_five"]["params"] == [["a", "Int"]]
    assert case["expect"] == {"type": "Bool", "value": True, "calls": 0}
    for slug in ("ill-typed-env-entry-of-type-int", "ill-typed-env-entry-of-an-undeclared-type",
                 "ill-typed-env-entry-result-with-a-record-as-error", "ill-typed-op-int-parameter-never-called"):
        assert cases[slug]["expect"] == {"error": "ill-typed"}, slug


def test_the_ir_fixed_cases_pin_the_edges_of_section_a1(kit):
    cases = s10d_cases(kit, "v2-ir")
    value = lambda slug: cases[slug]["expect"]["value"]  # noqa: E731
    assert value("shr-i8-is-logical") == "127" and value("sar-i8-negative-past-the-width") == "-1"
    assert value("shl-count-is-the-largest-nat") == "0" and value("rotl-count-is-the-largest-nat") == "128"
    assert value("clz-u8-zero") == "8" and value("ctz-u8-zero") == "8" and value("bit-u8-index-past-the-width") is False
    assert value("div-int-floors") == "-4" and value("rem-int-has-the-divisors-sign-negative") == "-1"
    assert value("div-trunc-i8-toward-zero") == "-3" and value("div-trunc-i8-minimum-by-minus-one") == "99"
    assert value("checked-mul-u64-maximum-squared") == "5" and value("try-nat-out-of-range") == "7"
    assert value("from-le-concat-of-a-sized-operand") == "1"
    assert cases["unwrap-or-evaluates-both-operands"]["expect"]["calls"] == 1  # the default's subject call is made and counted
    assert cases["ill-typed-from-be-of-a-plain-bytes-operand"]["expect"] == {"error": "ill-typed"}
    assert cases["ill-typed-div-of-signed-operands"]["expect"] == {"error": "ill-typed"}
    assert cases["overflow-fixed-to-int-product-corpus-class"]["expect"] == {"error": "ir-overflow", "class": "corpus"}
    assert cases["overflow-fixed-to-int-product-below-a-subject-case-class"]["expect"] == {"error": "ir-overflow", "class": "case"}
    assert cases["cost-fixed-doubling-chain-refused"]["expect"] == {"error": "ir-cost"}


def test_the_corpus_errors_of_a_match_have_no_kit_code_and_stay_out(kit):
    """A match that falls through, an unreachable arm, a bind twice or a shadowing bind are corpus errors (A.1): not vectors."""
    slugs = set(s10d_cases(kit, "v2-ir"))
    assert not {s for s in slugs if "not-exhaustive" in s or "no-value-can-reach" in s or "shadows" in s or "same-bind" in s}


# ---------------------------------------------------------------------------------------------------
# what the oracle learned
# ---------------------------------------------------------------------------------------------------


def test_a_subject_call_in_a_match_arm_is_rewritten_to_its_stand_in():
    ir = ["match", ["var", "r"], [[["ok", ["bind", "x"]], ["var", "x"]], [["err_any"], ["subject_op", "bump", [["lit", True]]]]]]
    out = C._rewrite_subjects(ir, {"bump": "inc"})
    assert out == ["match", ["var", "r"], [[["ok", ["bind", "x"]], ["var", "x"]], [["err_any"], ["model_call", "inc", [["lit", True]]]]]]
    scrutinee = ["match", ["subject_op", "f", []], [[["wild"], ["lit", True]]]]
    assert C._rewrite_subjects(scrutinee, {"f": "g"}) == ["match", ["model_call", "g", []], [[["wild"], ["lit", True]]]]


# ---------------------------------------------------------------------------------------------------
# a wrong evaluator fails an S10d case (the mutant test of [D42], for what S10d adds)
# ---------------------------------------------------------------------------------------------------


def derived(kit, category):
    """``{slug: derived expect}`` of the S10d rows of a category, computed from the tables with no intent check."""
    out = {}
    for _case, row in s10d(kit, category):
        try:
            out[row.slug] = C.derive_expect(category, row.fields, kit.kits)
        except C.KitError as exc:  # a mutant that makes the kit refuse to build the case is killed by it
            out[row.slug] = {"kit-error": str(exc)}
        except Exception as exc:  # noqa: BLE001  (a crash is a kill too)
            out[row.slug] = {"crash": type(exc).__name__}
    return out


@pytest.fixture(scope="module")
def baseline(kit):
    return {c: derived(kit, c) for c in ("v2-ir", "v2-domain", "v2-chunk")}


def killed(kit, baseline, category):
    now = derived(kit, category)
    return sorted(s for s, expect in now.items() if expect != baseline[category][s])


def test_a_tuple_enumerated_first_component_fastest_is_killed(kit, baseline, monkeypatch):
    def nth(self, i):
        if not 0 <= i < self.size():
            raise IndexError("domain index out of range")
        out = []
        for p in self.parts:  # the mutant: the first component varies fastest
            i, r = divmod(i, p.size())
            out.append(p.nth(r))
        return tuple(out)

    monkeypatch.setattr(D.TupleDesc, "nth", nth)
    assert killed(kit, baseline, "v2-domain")
    assert killed(kit, baseline, "v2-chunk")  # the suites' inputs digests change (kit-tuple.warrant)


def test_a_shr_that_fills_with_the_sign_is_killed(kit, baseline, monkeypatch):
    real = E._shift_fn

    def shift(tag, bits, signed, mask, a, b):
        if tag == "shr":
            return lambda env: 0 if b(env) >= bits else a(env) >> b(env)  # arithmetic: not the logical shift of section A.1
        return real(tag, bits, signed, mask, a, b)

    monkeypatch.setattr(E, "_shift_fn", shift)
    dead = killed(kit, baseline, "v2-ir")
    assert "shr-i8-is-logical" in dead and "shr-i8-minimum-by-seven" in dead


def test_a_shl_that_masks_the_count_like_a_host_is_killed(kit, baseline, monkeypatch):
    real = E._shift_fn

    def shift(tag, bits, signed, mask, a, b):
        if tag == "shl":
            return lambda env: E.wrap_to((a(env) & mask) << (b(env) % bits), bits, signed)  # the host's masking of the count
        return real(tag, bits, signed, mask, a, b)

    monkeypatch.setattr(E, "_shift_fn", shift)
    dead = killed(kit, baseline, "v2-ir")
    assert "shl-u8-count-equal-to-width" in dead and "shl-count-is-the-largest-nat" in dead


def test_a_rotate_that_does_not_reduce_the_count_first_is_killed(kit, baseline, monkeypatch):
    real = E._shift_fn

    def shift(tag, bits, signed, mask, a, b):
        if tag == "rotl":
            return lambda env: 0 if b(env) >= bits else real(tag, bits, signed, mask, a, b)(env)  # compares n with w, like a shift
        return real(tag, bits, signed, mask, a, b)

    monkeypatch.setattr(E, "_shift_fn", shift)
    dead = killed(kit, baseline, "v2-ir")
    assert "rotl-u8-count-reduced-mod-w" in dead and "rotl-count-is-the-largest-nat" in dead


def test_a_match_that_counts_every_arm_is_killed(kit, baseline, monkeypatch):
    # the mutant: a match arm is not a guard, so a subject call in any arm counts (a static count of the subject calls in the tree)
    monkeypatch.setattr(C, "calls_for", lambda prog, ir, tenv: sum(1 for n in C._nodes(ir) if n[0] == "subject_op"))
    dead = killed(kit, baseline, "v2-ir")
    assert "match-only-the-taken-arm-counts-ok" in dead
