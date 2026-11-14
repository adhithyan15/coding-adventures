"""Whole runs against the real, Rust-emitted corpus."""

from __future__ import annotations

import json

import pytest
from helpers import CORPUS_DIR, REPO_ROOT, mini_corpus, new_library

from coding_adventures_warrant_corpus import RunConfig, execute, prepare
from coding_adventures_warrant_corpus.errors import BindingError, UsageError
from coding_adventures_warrant_corpus.session import (
    check_filter_rules,
    environment,
    parse_fault_spec,
)


def config_for(package_dir, binding_path, **kwargs) -> RunConfig:
    return RunConfig(REPO_ROOT, CORPUS_DIR, binding_path, package_dir, **kwargs)


def test_a_correct_library_passes_everything(full_run) -> None:
    _prepared, outcome, lines, _bytes = full_run
    assert outcome.passed
    assert len(outcome.examples) == 12 and len(outcome.suites) == 9
    assert len(outcome.controls) == 8
    assert not [line for line in lines if line.startswith("not ok")]
    assert sum(line.startswith("ok ") for line in lines) == 12 + 9 + 8


def test_every_digest_equals_the_rust_corpus(full_run) -> None:
    prepared, outcome, _lines, _bytes = full_run
    by_id = {s.id: s for s in prepared.corpus.suites}
    for result in outcome.suites:
        suite = by_id[result.id]
        assert result.inputs_sha256 == suite.inputs_sha256
        assert result.expected_sha256 == suite.expected_sha256
        assert result.cases == suite.cases and result.discarded == suite.discarded
        assert result.calls == suite.expected_calls
    assert sum(r.cases for r in outcome.suites) == 572_109


def test_every_control_is_caught_by_a_required_claim(full_run) -> None:
    prepared, outcome, _lines, _bytes = full_run
    for control, spec in zip(outcome.controls, prepared.corpus.controls, strict=True):
        assert control.reached_export
        assert control.caught_by and set(control.caught_by) <= set(spec.required_claims)


def test_the_manifest_is_complete_and_sorted(full_run) -> None:
    _prepared, outcome, _lines, _bytes = full_run
    manifest = outcome.manifest
    assert manifest is not None and manifest["faulted"] is False
    ids = [row["id"] for row in manifest["tests"]]
    assert len(ids) == 29 and ids == sorted(ids)
    assert manifest["evidence_seed"] == "6287397068022371361"
    assert manifest["package"] == "python/ct-compare"


def test_a_second_run_produces_the_same_manifest(tmp_path, monkeypatch) -> None:
    root, corpus_dir = mini_corpus(tmp_path)
    manifests = []
    for index in range(2):
        _name, package_dir, binding_path = new_library(
            tmp_path / f"lib{index}", monkeypatch
        )
        config = RunConfig(root, corpus_dir, binding_path, package_dir)
        manifests.append(execute(prepare(config), lambda _line: None).manifest)
    first, second = (dict(m) for m in manifests)
    # Two different temporary libraries: only these two digests may differ.
    for key in ("source_sha256", "binding_sha256"):
        first.pop(key)
        second.pop(key)
    assert (
        first == second and len(first["tests"]) == 7
    )  # 3 examples, 2 suites, 2 controls


def mini_config(tmp_path, monkeypatch, overrides=None, **kwargs) -> RunConfig:
    root, corpus_dir = mini_corpus(tmp_path)
    _name, package_dir, binding_path = new_library(tmp_path, monkeypatch, overrides)
    return RunConfig(root, corpus_dir, binding_path, package_dir, **kwargs)


def test_a_faulted_run_fails_fast_and_writes_no_manifest(tmp_path, monkeypatch) -> None:
    lines: list[str] = []
    config = mini_config(tmp_path, monkeypatch, fault="invert", filter="/exhaustive/")
    outcome = execute(prepare(config), lines.append)
    assert not outcome.passed and outcome.manifest is None and not outcome.controls
    assert any(line.startswith("not ok ") and "@" in line for line in lines)
    assert all("exhaustive" in s.id for s in outcome.suites)
    assert not outcome.examples  # the filter excluded them
    assert all(s.failures == 1 for s in outcome.suites)  # stopped at the first case


def test_a_filter_matching_nothing_is_refused(tmp_path, monkeypatch) -> None:
    prepared = prepare(
        mini_config(tmp_path, monkeypatch, fault="invert", filter="/nope/")
    )
    with pytest.raises(UsageError):
        execute(prepared, lambda _line: None)


def test_a_wrong_library_fails_a_clean_run_without_a_manifest(
    tmp_path, monkeypatch
) -> None:
    lines: list[str] = []
    wrong = {"ct_eq_fixed": "def ct_eq_fixed(a, b):\n    return True\n"}
    outcome = execute(prepare(mini_config(tmp_path, monkeypatch, wrong)), lines.append)
    suite = "warrant/ct_compare/exhaustive/eq_fixed_agrees_w2"
    assert not outcome.passed and outcome.manifest is None
    assert any(line.startswith(f"not ok {suite}@") for line in lines)
    assert any("failing cases in total" in line for line in lines)
    assert f"not ok {suite} result-mismatch" in lines
    assert "not ok warrant/ct_compare/example/fixed_16_diff result-mismatch" in lines


def test_fault_spec_parsing() -> None:
    assert parse_fault_spec(None) == (None, None)
    assert parse_fault_spec("") == (None, None)
    assert parse_fault_spec("none") == (None, None)
    assert parse_fault_spec("invert") == ("invert", None)
    assert parse_fault_spec("invert:eq") == ("invert", "eq")
    with pytest.raises(UsageError):
        parse_fault_spec("no-such-fault")


def test_filter_rules() -> None:
    check_filter_rules(None, None)
    check_filter_rules("invert", "/exhaustive/")
    check_filter_rules("invert", None)
    for fault in (None, "", "none"):
        with pytest.raises(UsageError):
            check_filter_rules(fault, "/exhaustive/")


def test_environment_reads_the_three_variables(monkeypatch) -> None:
    for name in (
        "WARRANT_EVIDENCE_FAULT",
        "WARRANT_EVIDENCE_FILTER",
        "WARRANT_EVIDENCE_OUT",
    ):
        monkeypatch.delenv(name, raising=False)
    assert environment() == (None, None, None)
    monkeypatch.setenv("WARRANT_EVIDENCE_FAULT", "invert")
    monkeypatch.setenv("WARRANT_EVIDENCE_FILTER", "x")
    monkeypatch.setenv("WARRANT_EVIDENCE_OUT", "m.json")
    assert environment() == ("invert", "x", "m.json")
    monkeypatch.setenv("WARRANT_EVIDENCE_FILTER", "")
    assert environment()[1] is None


def edited_binding(tmp_path, monkeypatch, old: str, new: str):
    _name, package_dir, binding_path = new_library(tmp_path, monkeypatch)
    binding_path.write_bytes(
        binding_path.read_bytes().replace(old.encode(), new.encode())
    )
    return package_dir, binding_path


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ("module ct_compare", "module other"),
        ("lane python", "lane ruby"),
        ("op eq export=ct_eq\n", ""),  # missing op
        (
            "op eq_u64 export=ct_eq_u64\n",
            "op eq_u64 export=ct_eq_u64\nop extra export=ct_eq\n",
        ),
        (" width=implicit", ""),  # width parameter not bound
        (
            "op eq export=ct_eq\n",
            "op eq export=ct_eq width=implicit\n",
        ),  # width claimed
        (" errors=CT_SELECT_LENGTH_MISMATCH:ValueError", ""),  # error variant unmapped
        (
            "CT_SELECT_LENGTH_MISMATCH:ValueError",
            "CT_SELECT_LENGTH_MISMATCH:ValueError,OTHER_ERROR:KeyError",
        ),
    ],
)
def test_the_binding_must_be_total_and_consistent(
    tmp_path, monkeypatch, old, new
) -> None:
    package_dir, binding_path = edited_binding(tmp_path, monkeypatch, old, new)
    with pytest.raises(BindingError):
        prepare(config_for(package_dir, binding_path))


def test_a_tampered_digest_is_reported_as_a_suite_level_failure(
    tmp_path, monkeypatch
) -> None:
    root, corpus_dir = mini_corpus(tmp_path)
    document = json.loads((corpus_dir / "corpus.json").read_text())
    document["suites"][0]["inputs_sha256"] = "0" * 64
    (corpus_dir / "corpus.json").write_bytes((json.dumps(document)).encode())
    _name, package_dir, binding_path = new_library(tmp_path, monkeypatch)
    lines: list[str] = []
    outcome = execute(
        prepare(RunConfig(root, corpus_dir, binding_path, package_dir)), lines.append
    )
    tampered = document["suites"][0]["id"]
    assert not outcome.passed and outcome.manifest is None
    assert f"not ok {tampered} stream-mismatch" in lines
