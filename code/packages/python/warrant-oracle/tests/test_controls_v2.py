"""Faults, controls and the derivation of a whole v2 fixture (VER04r 5.6, 8, 10.3 checks 1-3)."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle import tfv
from coding_adventures_warrant_oracle.corpus_v2 import (
    MAX_SLIM_CHUNKS,
    canonical,
    chunk_file_name,
    derive,
    value_json,
)
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.faults import REGISTRY, fault_row
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

GEN = {"name": "test", "version": "0"}

BASE = """warrant d v1
  error E = BAD
  op id8(a: U8) -> U8
  op pair(a: U8, b: U8) -> U8
  op text(a: Bytes, b: Bytes) -> Bytes
  op flag(a: Bytes, choice: Bool) -> Bool
  op safe(a: Bytes) -> Result<Bytes, E>
  op wide(a: U16, b: U16) -> U16
  model same(a: U8, b: U8) -> U8 = a + b
"""


def build(extra: str, profile: str = "", base: str = BASE) -> object:
    return derive(
        (base + extra).encode(), profile.encode(), spec_ref="d.warrant", generator=GEN,
        implemented=("ir.core",),  # the closed gate, explicit: these tests assert ``deferred``
    )


def refused(extra: str, profile: str = "", base: str = BASE, code: str = "generation") -> str:
    with pytest.raises(OracleError) as err:
        build(extra, profile, base)
    assert err.value.code == code
    return err.value.message


# -- the registry, by signature ----------------------------------------------------------------


def ops_of(src: str = BASE):
    return lower_document(parse_source(src.encode())).ops


def test_every_registry_name_instantiates_for_a_matching_signature():
    ops = ops_of()
    assert fault_row(ops["flag"], "invert") == {
        "fault": "invert", "op": "flag", "kind": "post", "requires": ["ir.core"],
        "expr": ["not", ["var", "result"]],
    }
    assert fault_row(ops["safe"], "missing-error")["kind"] == "replace"
    assert fault_row(ops["safe"], "missing-error")["expr"][2] == ["ok", ["var", "a"]]
    swap = fault_row(ops["flag"], "swap-choice")
    assert swap["rewrites"] == [{"param": "choice", "expr": ["not", ["var", "choice"]]}]
    off = fault_row(ops["id8"], "off-by-one")
    assert off["expr"] == ["add", ["var", "result"], ["lit", {"type": "U8", "value": "1"}]]
    trunc = fault_row(ops["text"], "truncate-last")
    assert trunc["expr"][0] == "if" and trunc["expr"][2][0] == "take"
    ignore = fault_row(ops["text"], "ignore-last-byte")
    assert [r["param"] for r in ignore["rewrites"]] == ["a", "b"]  # parameter order
    xor = fault_row(ops["id8"], "xor-lsb")
    assert xor["requires"] == ["ir.fixed"] and xor["expr"][0] == "bit_xor"
    ops64 = ops_of(BASE + "  op eq64(a: U64, b: U64) -> Bool\n")
    high = fault_row(ops64["eq64"], "ignore-high-bit")
    assert high["requires"] == ["ir.fixed"]
    assert high["rewrites"][0]["expr"] == [
        "bit_and", ["var", "a"], ["lit", {"type": "U64", "value": "9223372036854775807"}]
    ]
    assert len(high["rewrites"]) == 2


@pytest.mark.parametrize(
    ("op", "fault"),
    [
        ("id8", "invert"),  # not a Bool result
        ("id8", "missing-error"),
        ("flag", "missing-error"),
        ("id8", "swap-choice"),  # no choice parameter
        ("flag", "off-by-one"),  # Bool is not a number
        ("id8", "truncate-last"),
        ("id8", "ignore-last-byte"),
        ("flag", "xor-lsb"),
        ("flag", "ignore-high-bit"),
        ("id8", "no-such-fault"),
    ],
)
def test_a_template_the_signature_cannot_take_is_an_error(op, fault):
    with pytest.raises(OracleError) as err:
        fault_row(ops_of()[op], fault)
    assert err.value.code == "generation" and err.value.message.startswith("fault:")


def test_registry_covers_the_names_of_section_8_2():
    assert set(REGISTRY) == {
        "invert", "missing-error", "swap-choice", "off-by-one",
        "truncate-last", "ignore-last-byte", "xor-lsb", "ignore-high-bit",
        # S10t-1 (VER04r 8.2, D27, D35, R3): the tuple templates and the Ok forms
        "flip-result-bit", "drop-last-result-byte", "ok-as-err", "drop-consumed",
        "swap-components",
        # S10o-1 (VER04r 8.2, D36): the second component of a tuple, exchanged 2k <-> 2k+1
        "off-by-one-consumed",
        # S11f-1 (VER04r 8.2): the scalar faults of the Rust harness
        "saturate-add", "saturate-sub", "shift-mask",
    }


# -- witnesses: lowest admitted product ordinal, chunk, three claim states -------------------------


def test_witness_is_the_lowest_admitted_ordinal_in_chunk_units():
    out = build(
        "  claim c:\n    forall a: U8 | a >= 3u8, b: U8. pair(a, b) == same(a, b)\n",
        "chunk_size 1024\nfault pair off-by-one\n",
    )
    ctl = out.corpus["controls"][0]
    # the lowest admitted ordinal is 3 * 256 = 768, chunk 0; the fault is visible there
    assert ctl["slim_witness"] == [{"claim": "warrant/d/exhaustive/c", "chunk": 0}]
    out = build(
        "  claim c:\n    forall a: U8 | a >= 5u8, b: U8. pair(a, b) == same(a, b)\n",
        "chunk_size 1024\nfault pair off-by-one\n",
    )
    assert out.corpus["controls"][0]["slim_witness"][0]["chunk"] == 1  # ordinal 1280


def test_a_pre_fault_is_evaluated_by_re_evaluating_expected_at_the_rewritten_binders():
    # flag(a, choice) is compared with a model that ignores `choice`: swapping it is invisible
    out = build(
        "  model const(a: Bytes) -> Bool = a == a\n"
        "  claim blind:\n    forall a: Bytes, c: Bool. flag(a, c) == const(a)\n"
        "  claim seen:\n    forall a: Bytes, c: Bool. flag(a, c) == c\n",
        "fault flag swap-choice\n",
    )
    ctl = out.corpus["controls"][0]
    assert ctl["blind_claims"] == ["warrant/d/exhaustive/blind"]
    assert ctl["required_claims"] == ["warrant/d/exhaustive/seen"]
    assert ctl["slim_witness"] == [{"claim": "warrant/d/exhaustive/seen", "chunk": 0}]


def test_unproven_claims_have_no_stand_in():
    out = build(
        "  claim pred:\n    forall a: U8, b: U8. pair(a, b) == pair(b, a)\n"
        "  claim computed:\n    forall a: U8, b: U8. pair(a, id8(b)) == same(a, b)\n"
        "  claim plain:\n    forall a: U8, b: U8. pair(a, b) == same(a, b)\n"
        "  claim twice:\n    forall a: U8. pair(a, a) == same(a, a)\n",
        "fault pair off-by-one\n",
    )
    ctl = out.corpus["controls"][0]
    states = {w["claim"].rsplit("/", 1)[1]: w for w in ctl["slim_witness"]}
    assert states["pred"] == {"claim": "warrant/d/exhaustive/pred", "unproven": True}
    assert states["computed"].get("unproven") is True
    assert states["plain"]["chunk"] == 0
    assert states["twice"]["chunk"] == 0  # a binder at two positions: a *post* fault is fine
    assert ctl["blind_claims"] == []


def test_a_binder_rewritten_at_one_of_its_positions_is_unproven():
    base = BASE + "  model eqm(a: Bytes, b: Bytes) -> Bytes = a\n"
    out = build(
        "  claim twice:\n    forall a: Bytes, c: Bool. flag(a, c) == (a == b\"00\")\n",
        "fault flag ignore-last-byte\n",
        base,
    )
    assert out.corpus["controls"][0]["slim_witness"][0]["chunk"] == 0
    out = build(
        "  claim twice:\n    forall a: Bytes. text(a, a) == a\n"
        "  claim once:\n    forall a: Bytes, b: Bytes. text(a, b) == a\n",
        "fault text ignore-last-byte\n",
        base,
    )
    states = {w["claim"].rsplit("/", 1)[1]: w for w in out.corpus["controls"][0]["slim_witness"]}
    assert states["twice"]["chunk"] == 0  # rewritten at both positions to one value
    assert states["once"]["chunk"] == 0


def test_a_binder_rewritten_at_only_some_of_its_positions_has_no_stand_in():
    # `flag(a, choice)` with the binder `c` at the position of a *Bool* parameter named
    # `choice`, rewritten, and no other position: stand-in exists.  Here the same binder
    # feeds two parameters of different rewrite status -> unproven.
    src = BASE + "  op both(a: Bool, choice: Bool) -> Bool\n"
    out = build(
        "  claim shared:\n    forall x: Bool. both(x, x) == x\n",
        "fault both swap-choice\n",
        src,
    )
    assert out.corpus["controls"][0]["slim_witness"] == [
        {"claim": "warrant/d/exhaustive/shared", "unproven": True}
    ]


def test_the_stand_in_follows_the_static_widths():
    src = BASE + "  op fixed<N: Nat>(a: Bytes<N>, b: Bytes<N>) -> Bool\n"
    out = build(
        "  claim w3:\n    forall a: Bytes<3>, b: Bytes<3>. fixed<3>(a, b) == (a == b)\n",
        "fault fixed ignore-last-byte\nfault fixed invert\n",
        src,
    )
    assert [c["fault"] for c in out.corpus["controls"]] == ["ignore-last-byte", "invert"]
    assert all(w["chunk"] == 0 for c in out.corpus["controls"] for w in c["slim_witness"])


def test_a_fault_of_an_unimplemented_level_is_data_and_deferred():
    out = build(
        "  claim c:\n    forall a: U8. id8(a) == same(a, 0u8)\n"
        "  claim pred:\n    forall a: U8. id8(a) == id8(a)\n",
        "fault id8 xor-lsb\n",
    )
    ctl = out.corpus["controls"][0]
    assert ctl["requires"] == ["ir.fixed"]
    assert ctl["slim_witness"] == [
        {"claim": "warrant/d/exhaustive/c", "deferred": "ir.fixed"},
        {"claim": "warrant/d/exhaustive/pred", "unproven": True},
    ]
    assert out.corpus["faults"][0]["expr"][0] == "bit_xor"


def test_faults_and_controls_are_sorted_by_op_then_fault_whatever_the_profile_order():
    out = build(
        "  claim a:\n    forall a: U8. id8(a) == same(a, 0u8)\n"
        "  claim b:\n    forall a: U8, b: U8. pair(a, b) == same(a, b)\n",
        "fault pair off-by-one\nfault id8 xor-lsb\nfault id8 off-by-one\n",
    )
    assert [(f["op"], f["fault"]) for f in out.corpus["faults"]] == [
        ("id8", "off-by-one"), ("id8", "xor-lsb"), ("pair", "off-by-one"),
    ]
    assert [c["id"] for c in out.corpus["controls"]] == [
        "warrant/d/negctl/exhaustive/id8/off-by-one",
        "warrant/d/negctl/exhaustive/id8/xor-lsb",
        "warrant/d/negctl/exhaustive/pair/off-by-one",
    ]


# -- generation failures -----------------------------------------------------------------------


def test_a_fault_every_candidate_is_blind_to_fails_generation():
    msg = refused(
        "  model k(a: U8) -> U8 = a\n  claim c:\n    forall a: U8. id8(a) == k(a)\n",
        "fault id8 invert\n",
    )  # invert needs a Bool result
    assert "fault" in msg
    msg = refused(
        "  model k(a: Bytes, c: Bool) -> Bool = a == a\n"
        "  claim c:\n    forall a: Bytes, c: Bool. flag(a, c) == k(a, c)\n",
        "fault flag swap-choice\n",
    )
    assert "blind" in msg


def test_a_fault_on_an_operation_no_claim_calls_fails():
    assert "no claim calls" in refused(
        "  claim c:\n    forall a: U8. id8(a) == same(a, 0u8)\n", "fault pair off-by-one\n"
    )


def test_profile_fault_and_slim_chunk_lines_are_checked_against_the_spec():
    claim = "  claim c:\n    forall a: U8. id8(a) == same(a, 0u8)\n"
    assert "undeclared operation" in refused(claim, "fault nope invert\n")
    assert "undeclared claim" in refused(claim, "slim_chunk missing 0\n")
    assert "past the suite" in refused(claim, "slim_chunk c 1\n")
    refused(claim + "  claim d:\n    forall a: U8. id8(a) == same(a, 0u8)\n", "chunk_size 1024\n"
            "slim_chunk c 1\n")  # a one-chunk suite
    assert "W0701" in refused(
        "  op bad(result: U8) -> U8\n" + claim, ""
    )


def test_more_than_32_slim_chunks_is_a_generation_error():
    claim = "  claim big:\n    forall a: U8, b: U8. pair(a, b) == same(a, b)\n"
    lines = "".join(f"slim_chunk big {i}\n" for i in range(5, 5 + MAX_SLIM_CHUNKS))
    assert "32 slim chunks" in refused(claim, "chunk_size 1024\n" + lines)
    out = build(claim, "chunk_size 1024\n" + "".join(f"slim_chunk big {i}\n" for i in (7, 8)))
    assert {7, 8} <= set(out.corpus["suites"][0]["slim_chunks"])


def test_a_stride_denser_than_automatic_adds_chunks_and_never_removes_the_ends():
    claim = "  claim big:\n    forall a: U8, b: U8. pair(a, b) == same(a, b)\n"
    auto = build(claim, "chunk_size 1024\n").corpus["suites"][0]["slim_chunks"]
    dense = build(claim, "chunk_size 1024\nslim_stride 4\n").corpus["suites"][0]["slim_chunks"]
    huge = build(claim, "chunk_size 1024\nslim_stride 999\n").corpus["suites"][0]["slim_chunks"]
    assert huge == auto  # clamped, never an error
    assert {0, 63} <= set(auto) and {0, 63} <= set(dense)
    assert len(dense) > len(auto)


def test_a_suite_that_admits_nothing_and_a_document_above_ir_core_are_refused():
    assert "zero" in refused("  claim z:\n    forall a: U8 | a > 255u8. id8(a) == a\n")
    # ir.fixed is evaluated since S10c-1: the document above ir.core is an ir.data one
    refused("  claim z:\n    forall a: U8. id8(a) == [a]\n", code="level")
    refused("  model m(a: U8) -> Nat = len([a])\n", code="level")
    refused(
        "  claim z:\n    forall a: U8. id8(a) == a\n"
        "  example e:\n    id8(0u8) == len([0u8])\n",
        code="level",
    )
    refused("  assume platform:\n    memory.bounded\n  claim z:\n    forall a: U8. id8(a) == a\n")
    refused(
        "  op bit(a: U8 | len([a]) == 1) -> U8\n  claim z:\n    forall a: U8. id8(a) == a\n",
        code="level",
    )


def test_an_example_is_one_subject_call_compared_with_a_pure_value():
    assert "subject call" in refused(
        "  claim z:\n    forall a: U8. id8(a) == a\n  example e:\n    1u8 == 1u8\n"
    )
    assert "subject call" in refused(
        "  claim z:\n    forall a: U8. id8(a) == a\n  example e:\n    id8(1u8) == id8(1u8)\n"
    )
    src = BASE + "  op fx<N: Nat, M: Nat>(a: Bytes<N>, b: Bytes<M>) -> Bool\n"
    assert "static widths" in refused(
        '  claim z:\n    forall a: U8. id8(a) == a\n  example e:\n    fx<1, 2>(b"00", b"0000") == true\n',
        base=src,
    )


def test_claim_file_names_follow_section_5_5():
    assert chunk_file_name("a_b") == "a_b.txt"
    assert chunk_file_name("op/post") == "op.post.txt"
    for bad in ("", "1a", "a/b/c", "a b", "a..b", "x" * 65, "con", "nul/post", "LPT3", "a/com1"):
        with pytest.raises(OracleError):
            chunk_file_name(bad)


def test_two_claims_equal_ignoring_case_are_a_generation_error():
    assert "file name" in refused(
        "  claim Same:\n    forall a: U8. id8(a) == a\n  claim same:\n    forall a: U8. id8(a) == a\n"
    )


# -- examples and values ---------------------------------------------------------------------------


def test_examples_values_and_tfv_follow_the_declared_types():
    out = build(
        "  claim z:\n    forall a: U8. id8(a) == a\n"
        '  example r:\n    safe(b"01") == Ok(b"01")\n'
        "  example e:\n    safe(b\"\") == Err(E.BAD)\n"
        "  example u:\n    wide(65535u16, 0u16) == 0u16\n",
    )
    rows = {r["id"].rsplit("/", 1)[1]: r for r in out.corpus["examples"]}
    assert rows["r"]["expected"] == {"ok": "01"} and rows["e"]["expected"] == {"err": "BAD"}
    assert rows["u"]["args"] == ["65535", "0"] and "width" not in rows["u"]
    text = out.tfv.decode()
    assert text.startswith("tfv 1\n") and text.endswith("\n")
    assert "Kexpect M1 S6f6b X01" in text and "Kexpect M1 S657272 S424144" in text
    assert "Ka I65535" in text


def test_value_json_and_tfv_refuse_types_above_ir_core():
    with pytest.raises(OracleError) as err:
        value_json((), A.Ty("List", (A.Ty("U8"),)))
    assert err.value.level == "ir.data"
    with pytest.raises(OracleError):
        tfv.value((), A.Ty("List", (A.Ty("U8"),)))
    assert value_json(True, A.Ty("Bool")) is True
    assert value_json(b"\x01", A.Ty("Bytes", (1,))) == "01"
    assert tfv.value(False, A.Ty("Bool")) == "F"
    assert tfv.document(["a"]) == b"tfv 1\na\n"


def test_types_section_covers_errors_records_and_variants():
    out = build(
        "  claim z:\n    forall a: U8. id8(a) == a\n",
        base=BASE.replace(
            "  op id8", "  record P(x: U8, y: Bytes)\n  variant V = A | B(n: Nat, y: Bytes)\n  op id8"
        ),
    )
    assert out.corpus["types"]["P"] == {"kind": "record", "fields": [["x", "U8"], ["y", "Bytes"]]}
    assert out.corpus["types"]["V"] == {
        "kind": "variant", "ctors": [["A", []], ["B", ["Nat", "Bytes"]]]
    }
    assert out.corpus["types"]["E"] == {"kind": "error", "cases": ["BAD"]}


def test_canonical_writer_is_sorted_two_space_lf_and_ascii():
    assert canonical({"b": [1, {"y": "é"}], "a": {}}) == (
        b'{\n  "a": {},\n  "b": [\n    1,\n    {\n      "y": "\\u00e9"\n    }\n  ]\n}\n'
    )
