"""Fixtures shared by the warrant-corpus tests."""

from __future__ import annotations

import json

import pytest
from helpers import CORPUS_DIR, REPO_ROOT, new_library

from coding_adventures_warrant_corpus import RunConfig, execute, load_corpus, prepare


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
