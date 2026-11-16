"""The private bytecode cache (VER04i 8.1): Python runs a cached ``.pyc`` instead of
its source, so evidence is only produced with ``sys.pycache_prefix`` pointing at
a fresh private directory, and the manifest says so."""

from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

import pytest
from helpers import REPO_ROOT, mini_corpus, new_library

from coding_adventures_warrant_corpus import main
from coding_adventures_warrant_corpus.errors import LaneError
from coding_adventures_warrant_corpus.isolation import (
    ENV_PRIVATE,
    verify_isolated_cache,
)
from coding_adventures_warrant_corpus.jsonio import render_json
from coding_adventures_warrant_corpus.manifest import validate_manifest


def test_the_suite_runs_under_a_private_cache(private_pycache):
    assert sys.pycache_prefix == private_pycache
    verify_isolated_cache()  # does not raise


def test_a_missing_prefix_is_refused(monkeypatch):
    monkeypatch.setattr(sys, "pycache_prefix", None)
    with pytest.raises(LaneError) as error:
        verify_isolated_cache()
    assert error.value.code == "pycache-isolation"


def test_a_prefix_the_runner_did_not_create_is_refused(monkeypatch, tmp_path):
    other = tempfile.mkdtemp(prefix="warrant-pycache-")
    monkeypatch.setattr(sys, "pycache_prefix", other)  # env still names the other one
    with pytest.raises(LaneError) as error:
        verify_isolated_cache()
    assert error.value.code == "pycache-isolation"
    monkeypatch.delenv(ENV_PRIVATE)
    with pytest.raises(LaneError):
        verify_isolated_cache()


def test_a_prefix_that_is_not_a_private_directory_under_temp_is_refused(monkeypatch, tmp_path):
    for bad in (tmp_path / "missing", REPO_ROOT / "code"):
        monkeypatch.setattr(sys, "pycache_prefix", str(bad))
        monkeypatch.setenv(ENV_PRIVATE, str(bad))
        with pytest.raises(LaneError) as error:
            verify_isolated_cache()
        assert error.value.code == "pycache-isolation"


@pytest.mark.skipif(os.name == "nt", reason="POSIX permission bits")
def test_a_shared_or_linked_cache_directory_is_refused(monkeypatch):
    shared = Path(tempfile.mkdtemp(prefix="warrant-pycache-"))
    shared.chmod(0o755)
    monkeypatch.setattr(sys, "pycache_prefix", str(shared))
    monkeypatch.setenv(ENV_PRIVATE, str(shared))
    with pytest.raises(LaneError) as error:
        verify_isolated_cache()
    assert error.value.code == "pycache-isolation"
    link = Path(tempfile.mkdtemp(prefix="warrant-pycache-")) / "link"
    link.symlink_to(shared)
    monkeypatch.setattr(sys, "pycache_prefix", str(link))
    monkeypatch.setenv(ENV_PRIVATE, str(link))
    with pytest.raises(LaneError):
        verify_isolated_cache()


def test_run_check_and_sabotage_refuse_to_start_without_the_cache(monkeypatch, tmp_path, capsys):
    monkeypatch.setattr(sys, "pycache_prefix", None)
    root, corpus_dir = mini_corpus(tmp_path)
    _name, package_dir, binding_path = new_library(tmp_path, monkeypatch)
    flags = [
        "--repo-root", str(root), "--corpus", str(corpus_dir),
        "--binding", str(binding_path), "--package-dir", str(package_dir),
    ]  # fmt: skip
    for command in (["run"], ["check", str(tmp_path / "m.json")], ["sabotage"]):
        head, *rest = command
        assert main([head, *flags, *rest]) == 2
        assert "pycache-isolation" in capsys.readouterr().err


def test_a_manifest_records_the_isolated_cache(full_run):
    _prepared, outcome, _lines, _raw = full_run
    assert outcome.manifest["pycache_isolated"] is True


def test_the_validator_requires_the_isolation_field(corpus, full_run):
    _prepared, outcome, _lines, _raw = full_run
    relative = "code/specs/fixtures/ct-compare-warrant-v1"
    good = json.loads(json.dumps(outcome.manifest))
    validate_manifest(render_json(good).encode(), corpus, relative)
    for edit in (lambda d: d.pop("pycache_isolated"), lambda d: d.update(pycache_isolated=False),
                 lambda d: d.update(pycache_isolated=1), lambda d: d.update(pycache_isolated="true")):
        bad = json.loads(json.dumps(good))
        edit(bad)
        with pytest.raises(LaneError):
            validate_manifest(render_json(bad).encode(), corpus, relative)
