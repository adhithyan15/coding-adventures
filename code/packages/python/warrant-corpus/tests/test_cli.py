"""The command line: exit codes, output, environment refusals, sabotage."""

from __future__ import annotations

import json
import os
import runpy
import sys

import pytest
from helpers import CORPUS_DIR, REGISTRY_PATH, REPO_ROOT, mini_corpus, new_library

from coding_adventures_warrant_corpus import main
from coding_adventures_warrant_corpus.cli import _compare
from coding_adventures_warrant_corpus.jsonio import render_json

for_env = ("WARRANT_EVIDENCE_FAULT", "WARRANT_EVIDENCE_FILTER", "WARRANT_EVIDENCE_OUT")


@pytest.fixture(autouse=True)
def clean_environment(monkeypatch):
    for name in for_env:
        monkeypatch.delenv(name, raising=False)


def flags(root, corpus_dir, package_dir, binding_path) -> list[str]:
    return [
        "--repo-root", str(root),
        "--corpus", str(corpus_dir),
        "--binding", str(binding_path),
        "--package-dir", str(package_dir),
    ]  # fmt: skip


def library(tmp_path, monkeypatch, overrides=None):
    """A reference library plus the two-suite mini corpus (fast whole runs)."""
    root, corpus_dir = mini_corpus(tmp_path)
    _name, package_dir, binding_path = new_library(tmp_path, monkeypatch, overrides)
    return flags(root, corpus_dir, package_dir, binding_path), package_dir, binding_path


def test_list_faults(capsys) -> None:
    assert (
        main(
            ["list-faults", "--repo-root", str(REPO_ROOT), "--corpus", str(CORPUS_DIR)]
        )
        == 0
    )
    lines = capsys.readouterr().out.split()
    assert (
        len(lines) == 8
        and "invert:eq_u64" in lines
        and "swap-choice:select_bytes" in lines
    )


def test_source_digest_command(tmp_path, capsys) -> None:
    (tmp_path / "a.py").write_bytes(b"x = 1\n")
    assert main(["source-digest", str(tmp_path)]) == 0
    assert len(capsys.readouterr().out.strip()) == 64
    assert main(["source-digest", str(tmp_path / "missing")]) == 2


def test_run_writes_a_manifest_and_check_accepts_it(
    tmp_path, monkeypatch, capsys
) -> None:
    common, _pkg, _b = library(tmp_path, monkeypatch)
    out = tmp_path / "evidence" / "test-manifest.json"
    assert main(["run", *common, "--out", str(out)]) == 0
    text = out.read_bytes().decode()
    assert text.endswith("}\n") and json.loads(text)["faulted"] is False
    assert "# wrote" in capsys.readouterr().out

    # The check command regenerates and compares: identical, so it passes ...
    assert main(["check", *common, str(out)]) == 0
    assert "matches the regenerated manifest" in capsys.readouterr().out

    # ... and a different interpreter version is not drift.
    document = json.loads(text)
    document["toolchain"]["version"] = "0.0.1"
    out.write_bytes((render_json(document)).encode())
    assert main(["check", *common, str(out)]) == 0

    # The same manifest, validated structurally, with the planned registry row.
    assert (
        main(["validate-manifest", *common, str(out), "--registry", str(REGISTRY_PATH)])
        == 0
    )
    assert (
        main(
            [
                "validate-manifest",
                *common,
                str(out),
                "--registry",
                str(REGISTRY_PATH),
                "--require-active",
            ]
        )
        == 2
    )


def test_check_reports_drift(tmp_path, capsys) -> None:
    fresh = {
        "schema": "x",
        "tests": [],
        "toolchain": {"name": "python", "version": "3"},
    }
    manifest = tmp_path / "m.json"

    manifest.write_bytes((render_json({**fresh, "tests": [1]})).encode())
    assert _compare(manifest, fresh) == 1
    assert "drifted (tests)" in capsys.readouterr().out

    manifest.write_bytes((json.dumps(fresh)).encode())  # same data, different bytes
    assert _compare(manifest, fresh) == 1
    assert "formatting" in capsys.readouterr().out

    assert _compare(tmp_path / "absent.json", fresh) == 1
    manifest.write_bytes(b"[]")
    assert _compare(manifest, fresh) == 1
    manifest.write_bytes(b"not json")
    assert _compare(manifest, fresh) == 1
    manifest.write_bytes(b"\xff")
    assert _compare(manifest, fresh) == 1
    assert _compare(manifest, None) == 1
    assert "cannot produce a manifest" in capsys.readouterr().out


def test_a_failing_library_exits_1(tmp_path, monkeypatch, capsys) -> None:
    common, _p, _b = library(
        tmp_path,
        monkeypatch,
        {"ct_eq_fixed": "def ct_eq_fixed(a, b):\n    return False\n"},
    )
    out = tmp_path / "m.json"
    assert main(["run", *common, "--out", str(out)]) == 1
    assert not out.exists()
    assert (
        "not ok warrant/ct_compare/exhaustive/eq_fixed_agrees_w2"
        in capsys.readouterr().out
    )


def test_environment_refusals_exit_2(tmp_path, monkeypatch, capsys) -> None:
    common, _p, _b = library(tmp_path, monkeypatch)
    cases = [
        {"WARRANT_EVIDENCE_FILTER": "/exhaustive/"},  # filter alone
        {"WARRANT_EVIDENCE_FILTER": "/exhaustive/", "WARRANT_EVIDENCE_FAULT": "none"},
        {"WARRANT_EVIDENCE_FAULT": "no-such-fault"},
        {
            "WARRANT_EVIDENCE_FAULT": "invert",
            "WARRANT_EVIDENCE_FILTER": "/matches-nothing/",
        },
    ]
    for env in cases:
        for key, value in env.items():
            monkeypatch.setenv(key, value)
        assert main(["run", *common]) == 2, env
        for key in env:
            monkeypatch.delenv(key)
    assert "error:" in capsys.readouterr().err


def test_a_faulted_run_prints_not_ok_and_never_writes_a_manifest(
    tmp_path, monkeypatch, capsys
) -> None:
    common, _p, _b = library(tmp_path, monkeypatch)
    out = tmp_path / "must-not-exist.json"
    monkeypatch.setenv("WARRANT_EVIDENCE_FAULT", "invert:eq_fixed")
    monkeypatch.setenv("WARRANT_EVIDENCE_FILTER", "/exhaustive/")
    monkeypatch.setenv("WARRANT_EVIDENCE_OUT", str(out))
    assert main(["run", *common]) == 1
    printed = capsys.readouterr().out
    assert any(line.startswith("not ok ") for line in printed.splitlines())
    assert not out.exists()


def test_a_filtered_but_passing_fault_run_writes_nothing(
    tmp_path, monkeypatch, capsys
) -> None:
    # `ignore-high-bit` does not apply to `eq_fixed`, so nothing fails; the run
    # still must not look like evidence.
    common, _p, _b = library(tmp_path, monkeypatch)
    out = tmp_path / "must-not-exist.json"
    monkeypatch.setenv("WARRANT_EVIDENCE_FAULT", "ignore-high-bit:eq_fixed")
    monkeypatch.setenv("WARRANT_EVIDENCE_FILTER", "eq_fixed_agrees_w2")
    assert main(["run", *common, "--out", str(out)]) == 0
    assert "no manifest written" in capsys.readouterr().out
    assert not out.exists()


def test_binding_and_package_dir_are_required(capsys) -> None:
    assert (
        main(["run", "--repo-root", str(REPO_ROOT), "--corpus", str(CORPUS_DIR)]) == 2
    )
    assert (
        main(["sabotage", "--repo-root", str(REPO_ROOT), "--corpus", str(CORPUS_DIR)])
        == 2
    )


def test_sabotage_proves_every_control_makes_the_runner_fail(
    tmp_path, monkeypatch, capsys
) -> None:
    common, _p, _b = library(tmp_path, monkeypatch)
    # The child processes import the reference library from the temp directory.
    monkeypatch.setenv(
        "PYTHONPATH",
        os.pathsep.join([str(tmp_path / "src"), os.environ.get("PYTHONPATH", "")]),
    )
    assert main(["sabotage", *common]) == 0
    lines = capsys.readouterr().out.splitlines()
    assert len(lines) == 2 and all(line.startswith("ok sabotage ") for line in lines)
    assert "ok sabotage invert:eq_fixed (exit 1)" in lines


def test_sabotage_fails_when_a_fault_goes_unnoticed(
    tmp_path, monkeypatch, capsys
) -> None:
    common, _p, _b = library(tmp_path, monkeypatch)
    monkeypatch.setenv(
        "PYTHONPATH",
        os.pathsep.join([str(tmp_path / "src"), os.environ.get("PYTHONPATH", "")]),
    )
    # Poison the child so it never fails: a corpus-less binding makes it exit 2,
    # which `sabotage` must report as "not caught" (it requires exit 1 + not ok).
    broken = list(common)
    broken[broken.index("--binding") + 1] = str(tmp_path / "missing.bindings")
    assert main(["sabotage", *broken]) == 1
    assert "not ok sabotage" in capsys.readouterr().out


def test_validate_manifest_refuses_a_bad_manifest(tmp_path, monkeypatch) -> None:
    common, _p, _b = library(tmp_path, monkeypatch)
    bad = tmp_path / "bad.json"
    bad.write_bytes(b"{}")
    assert main(["validate-manifest", *common, str(bad)]) == 2


def test_module_entry_point(monkeypatch, capsys) -> None:
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "warrant-corpus",
            "list-faults",
            "--repo-root",
            str(REPO_ROOT),
            "--corpus",
            str(CORPUS_DIR),
        ],
    )
    with pytest.raises(SystemExit) as exit_info:
        runpy.run_module("coding_adventures_warrant_corpus", run_name="__main__")
    assert exit_info.value.code == 0
    assert "invert:eq" in capsys.readouterr().out


def test_label_digest_binds_the_toolchain_name_but_not_its_version(corpus) -> None:
    """Another lane's manifest (TypeScript: toolchain `node`) must validate too."""
    from coding_adventures_warrant_corpus.cli import _labels_for

    base = {
        "package": "typescript/x",
        "module": "m",
        "toolchain": {"name": "node", "version": "1"},
    }
    same = _labels_for(corpus, {**base, "toolchain": {"name": "node", "version": "2"}})
    assert _labels_for(corpus, base) == same
    other = _labels_for(corpus, {**base, "toolchain": {"name": "python", "version": "1"}})
    assert other["manifest_sha256"] != same["manifest_sha256"]
