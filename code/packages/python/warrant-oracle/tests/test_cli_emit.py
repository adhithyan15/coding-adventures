"""Command line, the v2 enumeration emitter, and the gf256 / not-live findings."""

from __future__ import annotations

import json
import os

import pytest
from conftest import SPECS, V1_CORPUS, adopter_bytes, write_lf

from coding_adventures_warrant_oracle.cli import build_parser, run
from coding_adventures_warrant_oracle.emit import census, dumps, emit
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.profile import parse_profile

PROFILE = parse_profile(b"max_ir_cost 32768\n")


def cli(*argv):
    return run([str(a) for a in argv])


# -- verify-v1 --------------------------------------------------------------------------


def test_verify_v1_passes_and_emits_v2_digests(tmp_path, capsys):
    out = tmp_path / "ct.json"
    code = cli(
        "verify-v1", "--spec", SPECS / "ct-compare.warrant", "--corpus", V1_CORPUS, "--emit-v2", out
    )
    assert code == 0
    assert "verify-v1: ok: 572109 cases across 9 suites" in capsys.readouterr().out
    tree = json.loads(out.read_text())
    assert tree["schema"] == "warrant-oracle-enumeration-v0" and tree["depth"] == "full"
    row = tree["suites"]["select_picks"]
    assert row["cases"] == 139_818 and row["chunks_total"] == 4 and len(row["chunk_lines"]) == 4
    assert (
        row["inputs_sha256"]
        and row["v1_inputs_sha256"]
        and row["stream_header"] == "warrant-stream-v1"
    )


def test_verify_v1_reports_a_mismatch_with_exit_1(tmp_path, capsys):
    corpus = json.loads(V1_CORPUS.read_text(encoding="utf-8"))
    corpus["suites"][0]["expected_sha256"] = "0" * 64
    bad = tmp_path / "corpus.json"
    write_lf(bad, json.dumps(corpus))
    code = cli("verify-v1", "--spec", SPECS / "ct-compare.warrant", "--corpus", bad)
    captured = capsys.readouterr()
    assert code == 1 and "MISMATCH enumeration suites.eq_agrees.expected_sha256" in captured.err
    assert "FAILED" in captured.out  # stdout carries the verdict
    # a mismatch message never contains stream bytes or digests
    assert "0" * 64 not in captured.err


def test_unreadable_oversized_and_symlinked_inputs_exit_2(tmp_path, capsys):
    assert cli("check", "--spec", tmp_path / "missing.warrant") == 2
    assert "source" in capsys.readouterr().err
    big = tmp_path / "big.warrant"
    big.write_bytes(b"x" * ((1 << 20) + 1))
    assert cli("check", "--spec", big) == 2
    link = tmp_path / "link.warrant"
    try:
        os.symlink(SPECS / "ct-compare.warrant", link)
    except (OSError, NotImplementedError):  # pragma: no cover - platform without symlinks
        pytest.skip("symlinks unavailable")
    assert cli("check", "--spec", link) == 2
    assert cli("check", "--spec", tmp_path) == 2  # a directory is not a regular file
    bad = tmp_path / "bad.warrant"
    write_lf(bad, "not a warrant file\n")
    assert cli("check", "--spec", bad) == 2
    assert cli("verify-v1", "--spec", SPECS / "ct-compare.warrant", "--corpus", bad) == 2


def test_check_prints_the_level_census(capsys):
    assert cli("check", "--spec", SPECS / "lzw.warrant") == 0
    tree = json.loads(capsys.readouterr().out)
    # S10c-1: ir.fixed is evaluated, so all 21 suites of lzw are live (was ir.core 4 + ir.fixed 17)
    assert tree["module"] == "lzw" and tree["suites_by_level"] == {"ir.core": 21}


def test_parser_requires_a_command():
    with pytest.raises(SystemExit):
        build_parser().parse_args([])


# -- emit-v2 ---------------------------------------------------------------------------------


def test_emit_v2_for_ct_compare_agrees_with_verify_v1(tmp_path):
    out = tmp_path / "e.json"
    assert (
        cli(
            "emit-v2",
            "--spec",
            SPECS / "ct-compare.warrant",
            "--out",
            out,
            "--depth",
            "full",
            "--budget",
            400000,
        )
        == 0
    )
    tree = json.loads(out.read_text())
    corpus = json.loads(V1_CORPUS.read_text())
    assert tree["summary"] == {"ok": 9, "not-live": 0, "refused": 0, "not_live_by_level": {}}
    for row in corpus["suites"]:
        mine = tree["suites"][row["claim"]]
        assert mine["v1_inputs_sha256"] == row["inputs_sha256"] and mine["cases"] == row["cases"]


def test_emit_is_deterministic():
    a = dumps(emit(adopter_bytes("gf256"), profile=PROFILE))
    b = dumps(emit(adopter_bytes("gf256"), profile=PROFILE))
    assert a == b


def test_gf256_under_the_spec_cost_bound_is_refused_not_skipped():
    tree = emit(adopter_bytes("gf256"))
    assert tree["summary"]["ok"] == 34 and tree["summary"]["refused"] == 4
    refused = {k for k, v in tree["suites"].items() if v["status"] == "refused"}
    assert refused == {
        "inverse_is_an_involution",
        "inverse_matches_peasant_model",
        "multiply_by_model_inverse_is_one",
        "multiply_matches_peasant_model",
    }
    assert all(tree["suites"][k]["code"] == "limit" for k in refused)
    assert tree["max_ir_cost"] == 2000


def test_gf256_enumerates_everything_under_its_declared_profile():
    tree = emit(adopter_bytes("gf256"), profile=PROFILE)
    assert tree["summary"] == {"ok": 38, "not-live": 0, "refused": 0, "not_live_by_level": {}}
    rows = tree["suites"]
    assert all(r["wraps"] == 0 for r in rows.values())  # the section 3.3 guard: gf256 never wraps
    big = rows["add_associative"]
    assert (big["product"], big["chunks_total"], big["cases"]) == (1 << 24, 256, 1 << 24)
    assert big["oracle"] == "relational" and big["expected_calls"] == 4 << 24
    assert big["slim_chunks"][0] == 0 and big["slim_chunks"][-1] == 255
    assert big["chunks_run"] == big["slim_chunks"] and big["inputs_sha256"] is None
    assert big["stream_header"] == "warrant-stream-v1"
    assert rows["multiply_matches_peasant_model"]["oracle"] == "reference"
    assert (
        rows["power_successor"]["demoted"] == []
        and rows["power_successor"]["domain_label"] == "bounded-set"
    )
    assert rows["divide_self_is_one"]["discarded"] == 1


def test_gf256_evaluator_agrees_with_the_documented_examples():
    # multiply(83, 140) == 1 and multiply(255, 255) == 226 are spec examples; the
    # oracle's model evaluation reproduces them (the peasant model is the oracle here).
    from coding_adventures_warrant_oracle import ast as A
    from coding_adventures_warrant_oracle.corpus_v1 import build_program

    spec = lower_document(parse_source(adopter_bytes("gf256")))
    prog = build_program(spec, max_cost=32768)
    f = prog.compile(
        ["model_call", "gmul", [["var", "a"], ["var", "b"]]], {"a": A.Ty("U8"), "b": A.Ty("U8")}
    ).fn
    assert (
        f({"a": 83, "b": 140}) == 1
        and f({"a": 255, "b": 255}) == 226
        and f({"a": 128, "b": 2}) == 29
    )
    g = prog.compile(["model_call", "ginv", [["var", "a"]]], {"a": A.Ty("U8")}).fn
    assert g({"a": 83}) == 140 and g({"a": 2}) == 142 and g({"a": 255}) == 253


@pytest.mark.parametrize(
    "name,levels",
    [
        # S10a: a suite is named by the first construct, in source order, that the lowerer cannot
        # lower.  S10t-1: ``ir.tuple`` is evaluated (was wasm-leb128 fixed 8 + tuple 24, logic-gates
        # fixed 55 + data 3 + tuple 2, arithmetic fixed 9 + data 45 + tuple 4).  S10c-1: ``ir.fixed``
        # is evaluated too, so only ``ir.data`` still blocks a suite (was wasm-leb128 fixed 24,
        # logic-gates fixed 57 + data 3, lzw fixed 17, arithmetic fixed 11 + data 45, x25519 fixed 2).
        # S11a: ``ir.data`` is lowered, typed and costed but not evaluated, so a suite that needs it is
        # still counted under ``ir.data`` here (arithmetic 55: the 45 that used to be refused at lowering
        # plus the 10 that only counted a record type in their ``requires``).  What changed is that none
        # is *refused at lowering* any more (``suites_refused_at_lowering``).
        # S11c-1: ``ir.data`` is evaluated too, so nothing blocks a suite of any adopter (was logic-gates
        # data 3, arithmetic data 55): every suite that lowers is counted under ``ir.core``.
        # This is the parse-and-lower census (``census``): enumerating these adopters is far too
        # heavy for a unit test (wasm-leb128 is 46 million cases), see ``test_fixed_lzw``.
        ("wasm-leb128", {}),
        ("logic-gates", {}),
        ("lzw", {}),
        ("arithmetic", {}),
        ("x25519", {}),
    ],
)
def test_later_adopters_report_not_live_by_level(name, levels):
    c = census(lower_document(parse_source(adopter_bytes(name))))
    blocked = {k: v for k, v in c["suites_by_level"].items() if k != "ir.core"}
    assert blocked == levels
    assert sum(c["suites_by_level"].values()) == c["suites"]
    # S11a (section 16.14 acceptance): every suite of these adopters lowers whole
    assert c["suites_lowered"] == c["suites"] and c["suites_refused_at_lowering"] == {}
    assert c["models_refused_at_lowering"] == 0 and c["examples_refused_at_lowering"] == 0


def test_emit_can_select_suites_and_depth(tmp_path):
    out = tmp_path / "s.json"
    assert (
        cli(
            "emit-v2",
            "--spec",
            SPECS / "gf256.warrant",
            "--out",
            out,
            "--suite",
            "add_commutative",
            "--suite",
            "add_self_inverse",
            "--depth",
            "full",
            "--chunk-size",
            1 << 10,
        )
        == 0
    )
    tree = json.loads(out.read_text())
    assert sorted(tree["suites"]) == ["add_commutative", "add_self_inverse"]
    row = tree["suites"]["add_commutative"]
    assert row["chunks_total"] == 64 and row["inputs_sha256"] and len(row["chunk_lines"]) == 64
    assert tree["chunk_size"] == 1024


def test_a_budget_refusal_is_recorded_per_suite():
    tree = emit(adopter_bytes("gf256"), profile=parse_profile(b"budget 1024\nmax_ir_cost 32768\n"))
    assert tree["summary"]["refused"] > 0
    assert any("W0706" in r["reason"] for r in tree["suites"].values() if r["status"] == "refused")


def test_census_counts_levels():
    c = census(lower_document(parse_source(adopter_bytes("arithmetic"))))
    assert (
        c["suites"] == 64  # was 63 before VER04r D76 split the 9-binder claim in two
        and c["suites_by_level"] == {"ir.core": 64}  # S11c-1: was {"ir.core": 8, "ir.data": 55} (S11a)
        and c["suites_lowered"] == 64
        and c["models_not_ir_core"] == 0  # was 19 (S11a)
        and c["models_refused_at_lowering"] == 0
    )


def test_oracle_errors_print_code_and_message():
    err = OracleError("level", "x", level="ir.data")
    assert str(err) == "level: x" and err.level == "ir.data"


def test_module_entry_point(monkeypatch, capsys):
    import runpy
    import sys

    monkeypatch.setattr(
        sys, "argv", ["warrant-oracle", "check", "--spec", str(SPECS / "ct-compare.warrant")]
    )
    with pytest.raises(SystemExit) as exc:
        runpy.run_module("coding_adventures_warrant_oracle", run_name="__main__")
    assert exc.value.code == 0 and "ct_compare" in capsys.readouterr().out


def test_emit_reports_a_nine_binder_claim_as_refused_limit():
    # CV2-F17 O3: VER04r 5.3 / 14 / R75, the cap holds when deriving, not only in the kit reader.
    names = ", ".join(f"b{i}: Bool" for i in range(9))
    src = (
        "warrant many v1\n  op f(a: Bool) -> Bool\n  model keep(a: Bool) -> Bool = a\n"
        f"  claim wide:\n    forall {names}. f(b0) == keep(b0)\n"
    ).encode()
    tree = emit(src)
    assert tree["summary"]["refused"] == 1
    assert tree["suites"]["wide"]["status"] == "refused" and tree["suites"]["wide"]["code"] == "limit"
