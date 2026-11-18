"""Slice S10t-1: the committed dumps of the synthetic ``ir.tuple`` document.

``tests/fixtures/tuple-forms-v1/`` holds what slice S10t-2 (the Rust generator) must reproduce byte
for byte (VER04r 13, 16.10 R11): the document (generated), the lowering dump, the evaluation dump,
the fault dump and the refusal dump.  The tests pin each file three ways: regenerated from the
script, regenerated from the oracle, and read for *coverage* (every tuple form, every pattern, every
fault template, every corpus-error case).
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import subprocess
import sys

import pytest

from coding_adventures_warrant_oracle.cli import run
from coding_adventures_warrant_oracle.corpus_v2 import canonical
from coding_adventures_warrant_oracle.faults import REGISTRY
from coding_adventures_warrant_oracle.irtree import is_node
from coding_adventures_warrant_oracle.lowering_dump import SCHEMA, dump
from coding_adventures_warrant_oracle.parity_dumps import (
    EVALUATION_SCHEMA,
    FAULT_SCHEMA,
    GATE_ON,
    dump_evaluation,
    dump_faults,
    evaluation_view,
    faults_view,
)

FIXTURES = pathlib.Path(__file__).resolve().parent / "fixtures" / "tuple-forms-v1"
MAKER = FIXTURES / "make_tuple_forms.py"
SOURCE = FIXTURES / "tuple_forms_v1.warrant"
PROFILE = FIXTURES / "tuple_forms_v1.profile"
LOWERING = FIXTURES / "tuple_forms_v1.oracle-lowering.json"
EVALUATION = FIXTURES / "tuple_forms_v1.oracle-evaluation.json"
FAULTS = FIXTURES / "tuple_forms_v1.oracle-faults.json"
REFUSALS = FIXTURES / "tuple_forms_v1.oracle-refusals.json"
FILES = [SOURCE, PROFILE, LOWERING, EVALUATION, FAULTS, REFUSALS]


def maker():
    spec = importlib.util.spec_from_file_location("make_tuple_forms", MAKER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def built():
    return maker().build()


def test_every_committed_file_is_what_the_script_writes(built):
    assert {p.name for p in built} == {p.name for p in FILES}
    for path, data in built.items():
        assert path.read_bytes() == data, path.name


def test_the_script_check_mode_passes_and_detects_a_difference(tmp_path, built):
    module = maker()
    assert module.run(["--check"]) == 0
    probe = tmp_path / "x.warrant"
    module.HERE, saved = tmp_path, module.HERE  # point the maker at an empty directory
    try:
        assert module.run(["--check"]) == 1  # nothing committed there: every file differs
        assert module.run([]) == 0 and (tmp_path / "tuple_forms_v1.warrant").exists()
        assert module.run(["--check"]) == 0
    finally:
        module.HERE = saved
    del probe


def test_the_script_runs_as_a_program():
    done = subprocess.run([sys.executable, str(MAKER), "--check"], capture_output=True, text=True)
    assert done.returncode == 0, done.stderr


def test_the_dumps_are_canonical_json_with_their_schema():
    for path, schema in (
        (LOWERING, SCHEMA),
        (EVALUATION, EVALUATION_SCHEMA),
        (FAULTS, FAULT_SCHEMA),
        (REFUSALS, "warrant-refusals-parity-v1"),
    ):
        raw = path.read_bytes()
        tree = json.loads(raw)
        assert raw == canonical(tree), path.name
        assert tree["schema"] == schema
        assert b"\r" not in raw and raw.endswith(b"\n") and not raw.endswith(b"\n\n")
    assert dump(SOURCE.read_bytes()) == LOWERING.read_bytes()
    assert dump_evaluation(SOURCE.read_bytes(), PROFILE.read_bytes()) == EVALUATION.read_bytes()
    assert dump_faults(SOURCE.read_bytes(), PROFILE.read_bytes()) == FAULTS.read_bytes()


def test_the_dumps_carry_no_generator_name_or_version():
    for path in (EVALUATION, FAULTS, LOWERING, REFUSALS):
        assert b"generator" not in path.read_bytes() or path is REFUSALS


def test_dump_lowering_cli_checks_the_committed_file():
    argv = ["dump-lowering", "--spec", str(SOURCE), "--check", str(LOWERING)]
    assert run(argv) == 0


# -- coverage: every tuple form ----------------------------------------------------------------------------------


def nodes(tree):
    stack = [tree]
    while stack:
        item = stack.pop()
        if is_node(item):
            yield item
        if isinstance(item, list):
            stack.extend(item)
        elif isinstance(item, dict):
            stack.extend(item.values())


def test_the_lowering_dump_covers_every_tuple_form_and_every_pattern():
    tree = json.loads(LOWERING.read_bytes())
    seen_tags: set = set()
    arities: set = set()
    projects: set = set()
    patterns: set = set()
    for node in nodes(tree):
        seen_tags.add(node[0])
        if node[0] == "tuple" and len(node) == 2 and isinstance(node[1], list):
            arities.add(len(node[1]))
        if node[0] == "project":
            projects.add(node[2])
        if node[0] == "match":
            for pattern, _arm in node[2]:
                patterns.update(p[0] for p in nodes(pattern))
    assert arities == set(range(2, 9))
    assert projects >= {0, 1, 7}
    assert {"tuple", "project", "match"} <= seen_tags
    assert patterns >= {"ok", "err", "err_any", "tuple", "bind", "wild"}
    for suite in tree["suites"]:
        assert "ir.tuple" in suite["requires"] or suite["requires"] == ["ir.core"]
    names = {s["claim"] for s in tree["suites"]}
    assert {"c_lit2", "c_lit8", "c_k_match", "c_arm_calls", "c_admit_match", "c_proj_chain"} <= names


def test_the_document_has_a_nested_match_a_match_returning_a_tuple_and_a_result():
    tree = json.loads(LOWERING.read_bytes())
    bodies = {name: m["body"] for name, m in tree["models"].items()}
    assert bodies["m_nested_match"][0] == "match"
    assert any(n[0] == "match" for n in nodes(bodies["m_inner_match"]) if n is not bodies["m_inner_match"])
    assert tree["models"]["m_returns_tuple"]["ret"] == "(Bool,Bool)"
    assert tree["models"]["m_returns_result"]["ret"] == "Result<(Bool,Bool),TupleError>"
    assert [x["name"] for x in tree["examples"] if x["body"][0] == "match"] == ["e_match_ok", "e_match_err"]


def test_the_evaluation_dump_covers_domains_streams_counts_and_tfv():
    tree = json.loads(EVALUATION.read_bytes())
    assert tree["levels"] == ["ir.core", "ir.fixed", "ir.tuple"]  # the fixed faults are data rows
    headers = {s["stream"] for s in tree["suites"].values()}
    assert headers == {"warrant-stream-v1", "warrant-stream-v2"}
    domains = json.dumps([s["binders"] for s in tree["suites"].values()])
    assert '"d": "tuple"' in domains and '"d": "result"' in domains
    # a tuple of eight, a nested tuple, a result binder over a tuple
    assert {"c_t8", "c_proj_last8", "c_lit8"} & set(tree["suites"]) == {"c_proj_last8", "c_lit8"}
    assert tree["suites"]["c_arm_calls"]["expected_calls"] == 256  # an arm is a guard
    assert tree["suites"]["c_k_match"]["expected_calls"] == 4  # a subject scrutinee counts once
    assert tree["suites"]["c_arm_calls"]["wraps"] == 1
    assert tree["suites"]["c_swap_involutive"]["expected_calls"] == 8
    assert tree["suites"]["c_mixed"]["chunks"] == 33 and len(tree["suites"]["c_mixed"]["chunk_lines"]) == 33
    assert "L2 " in tree["cases_tfv"] and "M1 S6f6b L2" in tree["cases_tfv"]
    assert tree["omitted_examples"] == [
        {"id": "warrant/tuple_forms/example/e_match_err", "reason": "not-op-eq-pure"},
        {"id": "warrant/tuple_forms/example/e_match_ok", "reason": "not-op-eq-pure"},
    ]
    assert evaluation_view(SOURCE.read_bytes(), PROFILE.read_bytes()) == tree


def test_every_ir_tuple_fault_template_is_in_the_fault_dump():
    tree = json.loads(FAULTS.read_bytes())
    seen = {(r["fault"]) for r in tree["faults"]}
    assert seen >= {
        "ok-as-err",
        "drop-consumed",
        "swap-components",
        "missing-error",
        "flip-result-bit",
        "drop-last-result-byte",
        "invert",
    }
    by = {(r["op"], r["fault"]): r for r in tree["faults"]}
    # the four R11-normative rows, and the Ok forms of the two result-byte faults
    assert by[("op_k", "ok-as-err")]["expr"][0] == "match"
    assert by[("op_dec", "drop-consumed")]["expr"][0] == "match"
    assert by[("op_pair", "drop-consumed")]["expr"][0] == "tuple"
    assert by[("op_lit3", "swap-components")]["expr"][0] == "tuple"
    assert by[("op_dec", "missing-error")]["kind"] == "replace"
    assert by[("op_zero", "missing-error")]["expr"][2][1][1] == [["lit", False], ["lit", {"type": "Nat", "value": "0"}]]
    assert by[("op_bytes", "flip-result-bit")]["requires"] == ["ir.fixed", "ir.tuple"]
    assert by[("op_bytes", "drop-last-result-byte")]["requires"] == ["ir.core", "ir.tuple"]
    assert by[("op_proj_last8", "invert")]["requires"] == ["ir.core", "ir.tuple"]
    assert {r["fault"] for r in tree["faults"]} <= set(REGISTRY)


def test_the_fault_dump_shows_both_gates():
    tree = json.loads(FAULTS.read_bytes())
    states = lambda controls: {  # noqa: E731
        (c["op"], c["fault"]): {tuple(sorted(w)) for w in c["slim_witness"]} for c in controls
    }
    shipped, on = states(tree["controls_shipped"]), states(tree["controls_gate_on"])
    assert set(shipped) == set(on)
    for control in tree["controls_shipped"]:
        for w in control["slim_witness"]:
            assert "chunk" not in w or control["requires"] == ["ir.core"]
    # under the shipped gate only the ir.core control (drop-last-result-byte on Bytes<2>) is evaluated
    assert any("chunk" in w for w in tree["controls_shipped"][0]["slim_witness"])
    chunks = [c for c in tree["controls_gate_on"] if any("chunk" in w for w in c["slim_witness"])]
    assert {c["fault"] for c in chunks} >= {"ok-as-err", "drop-consumed", "swap-components", "missing-error"}
    # ir.fixed faults stay deferred even with ir.tuple evaluated
    flips = [c for c in tree["controls_gate_on"] if c["fault"] == "flip-result-bit"]
    assert flips and all(w.get("deferred") == "ir.fixed" for c in flips for w in c["slim_witness"])
    assert tree["accepted_blind_faults"] == []
    assert faults_view(SOURCE.read_bytes(), PROFILE.read_bytes()) == tree
    assert GATE_ON == ("ir.core", "ir.tuple")


# -- every corpus-error case, as refusals -------------------------------------------------------------------------


def test_every_corpus_error_of_appendix_a1_is_refused_with_the_code_corpus():
    tree = json.loads(REFUSALS.read_bytes())
    by = {r["name"]: r for r in tree["refusals"]}
    for kind in (
        "fall-through-missing-err",
        "fall-through-missing-case",
        "fall-through-missing-ok",
        "unreachable-after-wild",
        "unreachable-after-err-any",
        "unreachable-duplicate-case",
        "unreachable-after-all-cases",
        "unreachable-tuple-after-bind",
        "duplicate-bind",
        "duplicate-bind-nested",
        "bind-shadows-a-parameter",
        "bind-shadows-a-binder",
        "bind-shadows-result",
        "bind-shadows-an-outer-bind",
    ):
        assert by[kind]["oracle_code"] == "corpus" and by[kind]["generator_code"] == "ir", kind
    # the rest are not corpus errors: another code, and no generator code to mirror
    for name in (
        "arms-have-no-common-type",
        "a-tuple-pattern-needs-a-tuple",
        "a-tuple-pattern-of-the-wrong-arity",
        "an-unknown-error-case",
        "a-projection-index-out-of-range",
    ):
        assert by[name]["oracle_code"] == "type" and by[name]["generator_code"] == ""
    assert by["a-projection-of-a-non-tuple"]["oracle_level"] == "ir.data"
    messages = " ".join(r["message"] for r in tree["refusals"])
    assert "fall through" in messages and "no value can reach" in messages
    assert "twice" in messages and "shadows" in messages


def test_every_refusal_document_is_refused_by_the_oracle_today():
    module = maker()
    for name, source, expected in module.refusal_documents():
        exc = module.refusal_of(source)
        assert exc is not None and exc.code == expected, name
    # and a document that is NOT a refusal says so
    assert module.refusal_of(module._BASE + "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(x) -> x | Err(_) -> false\n") is None
