"""Fixtures shared by the warrant-corpus tests."""

from __future__ import annotations

import json
import os
import shutil
import sys
import tempfile

import pytest
from helpers import CORPUS_DIR, REPO_ROOT, new_library

from coding_adventures_warrant_corpus import RunConfig, execute, load_corpus, prepare
from coding_adventures_warrant_corpus.isolation import ENV_PRIVATE


@pytest.fixture(scope="session", autouse=True)
def private_pycache():
    """Run the whole suite the way the entry script runs the runner (VER04i 8.1).

    A manifest can only be built under a private bytecode cache, so every test
    that builds one needs ``sys.pycache_prefix`` set to a fresh directory this
    process owns.  The tests that exercise a missing or wrong cache override it.
    """
    directory = tempfile.mkdtemp(prefix="warrant-pycache-")
    with pytest.MonkeyPatch.context() as monkeypatch:
        monkeypatch.setattr(sys, "pycache_prefix", directory)
        monkeypatch.setenv(ENV_PRIVATE, directory)
        yield directory
    shutil.rmtree(directory, ignore_errors=True)
    assert not os.path.exists(directory)


@pytest.fixture(scope="session")
def corpus():
    """The real, Rust-emitted ct-compare corpus, validated once."""
    return load_corpus(REPO_ROOT, CORPUS_DIR)


@pytest.fixture(scope="session")
def full_run(tmp_path_factory):
    """ONE complete clean run of a reference library over the real corpus.

    Everything that needs "the whole corpus passed" shares this run: the
    digests it computed are compared with the ones the Rust compiler wrote.
    """
    tmp_path = tmp_path_factory.mktemp("full-run")
    with pytest.MonkeyPatch.context() as monkeypatch:
        _name, package_dir, binding_path = new_library(tmp_path, monkeypatch)
        prepared = prepare(RunConfig(REPO_ROOT, CORPUS_DIR, binding_path, package_dir))
        lines: list[str] = []
        outcome = execute(prepared, lines.append)
    assert outcome.manifest is not None
    return prepared, outcome, lines, json.dumps(outcome.manifest).encode()
