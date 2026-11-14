"""Source digests (`warrant-src-v1`) and manifest validation."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import sys

import pytest
from helpers import CORPUS_DIR, REGISTRY_PATH, REPO_ROOT

from coding_adventures_warrant_corpus import (
    frame_sources,
    source_digest,
    validate_manifest,
)
from coding_adventures_warrant_corpus.errors import CorpusError
from coding_adventures_warrant_corpus.manifest import read_source_tree, runner_digest

REL = "code/specs/fixtures/ct-compare-warrant-v1"


def test_frame_sources_has_the_specified_layout() -> None:
    framed = frame_sources([("b.py", b"22"), ("a.py", b"1")])
    expected = (
        b"warrant-src-v1\x00"
        + (2).to_bytes(8, "little")
        + (4).to_bytes(8, "little") + b"a.py" + (1).to_bytes(8, "little") + b"1"
        + (4).to_bytes(8, "little") + b"b.py" + (2).to_bytes(8, "little") + b"22"
    )  # fmt: skip
    assert framed == expected  # sorted by path bytes, every field length-prefixed
    assert frame_sources([]) == b"warrant-src-v1\x00" + bytes(8)


def test_framing_cannot_confuse_a_path_with_content() -> None:
    assert frame_sources([("a", b"bc")]) != frame_sources([("ab", b"c")])
    assert frame_sources([("a", b""), ("b", b"x")]) != frame_sources(
        [("a", b"x"), ("b", b"")]
    )


def test_source_digest_selects_python_sources_only(tmp_path) -> None:
    (tmp_path / "pkg").mkdir()
    (tmp_path / "pkg" / "__init__.py").write_bytes(b"x = 1\n")
    (tmp_path / "pkg" / "mod.pyi").write_bytes(b"x: int\n")
    (tmp_path / "py.typed").write_bytes(b"")
    (tmp_path / "notes.txt").write_bytes(b"ignored")
    (tmp_path / "__pycache__").mkdir()
    (tmp_path / "__pycache__" / "mod.cpython-313.pyc").write_bytes(b"\x00")
    names = [name for name, _data in read_source_tree(tmp_path)]
    assert names == ["pkg/__init__.py", "pkg/mod.pyi", "py.typed"]
    digest = source_digest(tmp_path)
    assert (
        digest == hashlib.sha256(frame_sources(read_source_tree(tmp_path))).hexdigest()
    )
    (tmp_path / "pkg" / "__init__.py").write_bytes(b"x = 2\n")
    assert source_digest(tmp_path) != digest  # editing production code changes it
    (tmp_path / "notes.txt").write_bytes(b"changed")
    assert source_digest(tmp_path) == source_digest(tmp_path)


def test_source_tree_refuses_what_it_cannot_account_for(tmp_path, monkeypatch) -> None:
    (tmp_path / "a.py").write_bytes(b"")
    os.symlink(tmp_path / "a.py", tmp_path / "link.py")
    with pytest.raises(CorpusError, match="symlink"):
        read_source_tree(tmp_path)
    with pytest.raises(CorpusError):
        source_digest(tmp_path / "missing")

    big = tmp_path / "big"
    big.mkdir()
    (big / "huge.py").write_bytes(("x" * 20).encode())
    monkeypatch.setattr(
        "coding_adventures_warrant_corpus.manifest.MAX_SOURCE_FILE_BYTES", 10
    )
    with pytest.raises(CorpusError, match="larger"):
        read_source_tree(big)

    deep = tmp_path / "deep"
    deep.mkdir()
    cursor = deep
    for index in range(3):
        cursor = cursor / f"d{index}"
        cursor.mkdir()
    monkeypatch.setattr("coding_adventures_warrant_corpus.manifest.MAX_SOURCE_DEPTH", 1)
    with pytest.raises(CorpusError, match="deeply"):
        read_source_tree(deep)


@pytest.mark.skipif(
    sys.platform == "win32",
    reason="NTFS stores names as UTF-16, so a non-UTF-8 file name cannot exist there",
)
def test_a_non_utf8_file_name_is_refused(tmp_path) -> None:
    try:
        with open(os.fsencode(tmp_path) + b"/\xff.py", "wb"):
            pass
    except OSError:
        pytest.skip("this filesystem refuses non-UTF-8 names")
    with pytest.raises(CorpusError, match="UTF-8"):
        read_source_tree(tmp_path)


def test_unreadable_directories_are_errors(tmp_path) -> None:
    if sys.platform == "win32" or not hasattr(os, "geteuid"):
        pytest.skip("chmod(0) cannot make a directory unreadable on Windows")
    if os.geteuid() == 0:
        pytest.skip("root can read everything")
    locked = tmp_path / "locked"
    locked.mkdir()
    locked.chmod(0)
    try:
        with pytest.raises(CorpusError):
            read_source_tree(locked)
    finally:
        locked.chmod(0o700)


def test_runner_digest_is_stable() -> None:
    assert runner_digest() == runner_digest() and len(runner_digest()) == 64


@pytest.fixture(scope="module")
def manifest_bytes(full_run):
    """The real manifest from the session-wide full run."""
    return full_run[3]


def mutated(manifest_bytes: bytes, change) -> bytes:
    document = json.loads(manifest_bytes)
    change(document)
    return json.dumps(document).encode()


def test_a_real_manifest_validates(manifest_bytes, corpus) -> None:
    validate_manifest(manifest_bytes, corpus, REL)
    validate_manifest(manifest_bytes, corpus, REL, REGISTRY_PATH.read_bytes())


def test_the_planned_registry_row_is_not_active(manifest_bytes, corpus) -> None:
    with pytest.raises(CorpusError, match="not active"):
        validate_manifest(
            manifest_bytes, corpus, REL, REGISTRY_PATH.read_bytes(), require_active=True
        )
    active = json.loads(REGISTRY_PATH.read_text())
    active["consumers"][0]["status"] = "active"
    validate_manifest(
        manifest_bytes, corpus, REL, json.dumps(active).encode(), require_active=True
    )


def test_registry_joins(manifest_bytes, corpus) -> None:
    registry = json.loads(REGISTRY_PATH.read_text())
    for change in (
        lambda r: r.update(consumers=[]),
        lambda r: r["consumers"][0].update(module="other"),
        lambda r: r["consumers"].append(dict(r["consumers"][0])),
        lambda r: r.update(consumers=3),
    ):
        broken = copy.deepcopy(registry)
        change(broken)
        with pytest.raises(CorpusError):
            validate_manifest(manifest_bytes, corpus, REL, json.dumps(broken).encode())


@pytest.mark.parametrize(
    "change",
    [
        lambda d: d.update(extra=1),
        lambda d: d.pop("tests"),
        lambda d: d.update(schema="warrant-test-evidence-v1"),
        lambda d: d.update(lane="Python"),
        lambda d: d.update(package="ruby/ct-compare"),
        lambda d: d.update(package="python/ct-compare/extra"),
        lambda d: d.update(module="other"),
        lambda d: d.update(spec="code/specs/other.warrant"),
        lambda d: d.update(spec_sha256="0" * 64),
        lambda d: d.update(corpus="code/specs/fixtures/elsewhere"),
        lambda d: d.update(corpus_sha256="0" * 64),
        lambda d: d.update(cases_sha256="0" * 64),
        lambda d: d["runner"].update(name="other"),
        lambda d: d["runner"].update(lane="ruby"),
        lambda d: d["runner"].update(src_sha256="XYZ"),
        lambda d: d["runner"].update(extra=1),
        lambda d: d.update(binding_sha256="nothex"),
        lambda d: d.update(source_sha256=3),
        lambda d: d["toolchain"].update(version=3),
        lambda d: d["toolchain"].pop("name"),
        lambda d: d.update(evidence_seed=5),
        lambda d: d.update(evidence_seed="05"),
        lambda d: d.update(evidence_seed=str(1 << 64)),
        lambda d: d.update(faulted=True),
        lambda d: d.update(tests=d["tests"][:-1]),
        lambda d: d.update(tests=[*d["tests"][:-1], d["tests"][0]]),
        lambda d: d["tests"].reverse(),
        lambda d: d["tests"][0].update(extra=1),
        lambda d: d["tests"][0].update(cases=2),
        lambda d: d["tests"][0].update(verdict="fail"),
        lambda d: d["tests"][0].update(layer="native"),
        lambda d: d["tests"][0].update(kind="property"),
        lambda d: d["tests"][0].update(op="eq_u64"),
        lambda d: d["tests"][0].update(id=3),
        lambda d: d["tests"][12].update(inputs_sha256="0" * 64),
        lambda d: d["tests"][12].update(calls=1),
        lambda d: d["tests"][12].update(domain="Bytes[1]"),
        lambda d: d["tests"][12].update(claim="other"),
        lambda d: d["tests"][12].update(discarded=1),
        lambda d: d["tests"][21].update(caught_by=[]),
        lambda d: d["tests"][21].update(
            caught_by=["warrant/ct_compare/exhaustive/eq_u64_agrees"]
        ),
        lambda d: d["tests"][21].update(caught_by=[d["tests"][21]["caught_by"][0]] * 2),
        lambda d: d["tests"][21].update(target="property"),
        lambda d: d["tests"][21].update(fault="invert"),
        lambda d: d["tests"][21].update(caught_by="x"),
    ],
)
def test_validation_rejects_every_kind_of_disagreement(
    manifest_bytes, corpus, change
) -> None:
    with pytest.raises(CorpusError):
        validate_manifest(mutated(manifest_bytes, change), corpus, REL)


def test_validation_rejects_malformed_json(corpus) -> None:
    with pytest.raises(CorpusError):
        validate_manifest(b"{", corpus, REL)


def test_corpus_constants() -> None:
    assert CORPUS_DIR.relative_to(REPO_ROOT).as_posix() == REL
