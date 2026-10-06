"""Native adoption evidence for the checked source-input registry."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

import build_tool.hasher as hasher
from build_tool.discovery import Package
from build_tool.source_input_registry import registry_digest, registry_json

REPO_ROOT = Path(__file__).resolve().parents[5]
FIXTURES = REPO_ROOT / "code/specs/fixtures/build-tool-v1"
REGISTRY = FIXTURES / "language-source-input-registry.json"
CASES = (
    "source-collection-extension.json",
    "source-collection-declared.json",
    "source-collection-registry-roles.json",
    "source-collection-engram-wasm-exact-inputs.json",
    "source-collection-typescript-blog-exact-inputs.json",
    "source-collection-typescript-landing-page-exact-inputs.json",
    "source-collection-typescript-site-foreign-package.json",
)


def test_production_projection_equals_complete_checked_registry() -> None:
    """No missing or extra selector may hide in a test-only lookup."""
    checked = json.loads(REGISTRY.read_text(encoding="utf-8"))
    production = json.loads(registry_json())
    assert production == checked
    assert registry_json() == REGISTRY.read_bytes()
    canonical = json.dumps(
        checked, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    expected = hashlib.sha256(
        b"coding-adventures/build-tool-language-source-input-registry/v1\x00"
        + len(canonical).to_bytes(8, "big")
        + canonical
    ).hexdigest()
    assert registry_digest() == expected
    for name in CASES:
        case = json.loads((FIXTURES / "cases" / name).read_text(encoding="utf-8"))
        assert case["input"]["options"]["registry_sha256"] == expected


def test_sync_command_finds_no_projection_drift() -> None:
    command = (
        REPO_ROOT
        / "code/programs/python/build-tool/tools/sync_source_input_registry.py"
    )
    completed = subprocess.run(
        [sys.executable, str(command), "--check"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr


@pytest.mark.parametrize("name", CASES)
def test_production_collector_matches_neutral_source_case(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, name: str
) -> None:
    """Materialize inert candidates and call the real disk collector."""
    case = json.loads((FIXTURES / "cases" / name).read_text(encoding="utf-8"))
    options = case["input"]["options"]
    package_root = tmp_path / options["package_root"]
    inert_boundaries = {
        package_root / candidate["path"]
        for candidate in options["candidates"]
        if candidate["kind"] in {"symlink", "reparse_point"}
    }
    for candidate in options["candidates"]:
        if candidate["kind"] != "file":
            # Existing native tests exercise real symlink and reparse pruning.
            continue
        path = package_root / candidate["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(bytes.fromhex(candidate["content_hex"]))

    # A neutral record is not a portable filesystem symlink. Model the two
    # inert directory boundaries through the production link check while real
    # OS link/junction behavior remains covered by test_hasher.py.
    real_link_check = hasher._is_link_or_reparse
    monkeypatch.setattr(
        hasher,
        "_is_link_or_reparse",
        lambda path: path in inert_boundaries or real_link_check(path),
    )

    package = Package(
        name=f"{options['language']}/fixture",
        path=package_root,
        language=options["language"],
        is_starlark=options["mode"] == "declared_sources",
        declared_srcs=options["declared_srcs"],
        repository_root=tmp_path,
    )
    collected = hasher._collect_source_files(package)
    assert [path.relative_to(package_root).as_posix() for path in collected] == [
        entry["path"] for entry in case["expected"]["result"]["files"]
    ]
    assert [hasher._hash_file(path) for path in collected] == [
        entry["digest"] for entry in case["expected"]["result"]["files"]
    ]


def test_unknown_language_fails_before_enumeration(tmp_path: Path, monkeypatch) -> None:
    package = Package(
        name="unknown/example",
        path=tmp_path / "code/packages/python/example",
        language="unknown",
        repository_root=tmp_path,
    )

    def forbidden_walk(*_args, **_kwargs):
        raise AssertionError("unknown language must fail before walking")

    monkeypatch.setattr(os, "walk", forbidden_walk)
    with pytest.raises(ValueError, match="unknown source-input language"):
        hasher._collect_source_files(package)


def test_empty_declared_sources_do_not_widen_to_recursive_suffixes(
    tmp_path: Path,
) -> None:
    root = tmp_path / "code/packages/python/empty-srcs"
    root.mkdir(parents=True)
    (root / "BUILD").write_bytes(b"build\n")
    (root / "main.py").write_bytes(b"source\n")
    package = Package(
        name="python/empty-srcs",
        path=root,
        language="python",
        is_starlark=True,
        declared_srcs=[],
    )
    assert [path.name for path in hasher._collect_source_files(package)] == ["BUILD"]


def test_exact_package_rule_requires_matching_lane(tmp_path: Path) -> None:
    root = tmp_path / "code/packages/python/engram-wasm"
    root.mkdir(parents=True)
    (root / "BUILD").write_bytes(b"build\n")
    package = Package(
        name="rust/engram-wasm",
        path=root,
        language="rust",
        repository_root=tmp_path,
    )
    with pytest.raises(ValueError, match="package root language"):
        hasher._collect_source_files(package)


def test_registered_site_uses_typescript_registry_despite_legacy_graph_name(
    tmp_path: Path,
) -> None:
    root = tmp_path / "code/sites/landing-page"
    (root / "data").mkdir(parents=True)
    (root / "BUILD").write_bytes(b"build\n")
    (root / "data/index.landing").write_bytes(b"page\n")
    package = Package(
        name="unknown/landing-page",
        path=root,
        language="unknown",
        repository_root=tmp_path,
    )
    assert [
        path.relative_to(root).as_posix()
        for path in hasher._collect_source_files(package)
    ] == [
        "BUILD", "data/index.landing"
    ]


def test_registered_site_hash_frames_real_repository_paths(tmp_path: Path) -> None:
    root = tmp_path / "code/sites/landing-page"
    root.mkdir(parents=True)
    (root / "BUILD").write_bytes(b"build\n")
    package = Package(
        name="unknown/landing-page",
        path=root,
        language="unknown",
        repository_root=tmp_path,
    )
    expected = hashlib.sha256()
    hasher._update_file_frame(
        expected, "code/sites/landing-page/BUILD", root / "BUILD", root
    )
    assert hasher.hash_package(package) == expected.hexdigest()


def test_registered_site_requires_repository_anchor(tmp_path: Path) -> None:
    root = tmp_path / "code/sites/blog"
    root.mkdir(parents=True)
    package = Package(name="unknown/blog", path=root, language="unknown")
    with pytest.raises(ValueError, match="anchored repository root"):
        hasher._collect_source_files(package)


def test_nested_site_suffix_cannot_claim_exact_registration(tmp_path: Path) -> None:
    root = tmp_path / "code/other/code/sites/blog"
    root.mkdir(parents=True)
    package = Package(
        name="unknown/blog", path=root, language="unknown", repository_root=tmp_path
    )
    with pytest.raises(ValueError):
        hasher._collect_source_files(package)


def test_linked_package_root_rejected_before_walk(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "code/packages/python/linked"
    root.mkdir(parents=True)
    package = Package(
        name="python/linked",
        path=root,
        language="python",
        repository_root=tmp_path,
    )
    real_check = hasher._is_link_or_reparse
    monkeypatch.setattr(
        hasher,
        "_is_link_or_reparse",
        lambda path: path == root or real_check(path),
    )
    monkeypatch.setattr(
        os,
        "walk",
        lambda *_args, **_kwargs: pytest.fail("linked root was enumerated"),
    )
    with pytest.raises(OSError, match="linked directory"):
        hasher._collect_source_files(package)


def test_hardlinked_source_cannot_import_ambient_bytes(tmp_path: Path) -> None:
    root = tmp_path / "code/packages/python/hardlink"
    root.mkdir(parents=True)
    (root / "BUILD").write_bytes(b"build\n")
    ambient = tmp_path / "ambient.py"
    ambient.write_bytes(b"secret\n")
    try:
        os.link(ambient, root / "main.py")
    except OSError as error:
        pytest.skip(f"hardlinks unavailable: {error}")
    package = Package(
        name="python/hardlink",
        path=root,
        language="python",
        repository_root=tmp_path,
    )
    with pytest.raises(OSError, match="not a regular file"):
        hasher.hash_package(package)


@pytest.mark.parametrize(
    "relative",
    [
        "e\u0301.py",
        "CON.py",
        "CONIN$.py",
        "COM¹.py",
        "bad. ",
        "bad:part.py",
        "bad\u202epath.py",
    ],
)
def test_nonportable_candidate_name_is_rejected(relative: str) -> None:
    with pytest.raises(OSError):
        hasher._validate_candidate_path(relative, {})


def test_casefold_alias_is_rejected_even_for_unselected_inputs() -> None:
    seen: dict[str, str] = {}
    hasher._validate_candidate_path("Dir/a.txt", seen)
    with pytest.raises(OSError, match="portable identity alias"):
        hasher._validate_candidate_path("dir/b.txt", seen)


def test_glob_work_ceiling_is_checked_before_matching() -> None:
    with pytest.raises(OSError, match="SOURCE_HASH_LIMIT_EXCEEDED"):
        hasher._charge_glob_work("*.py", "main.py", hasher._MAX_GLOB_WORK)


def test_candidate_ceiling_applies_incrementally(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "code/packages/python/candidate-cap"
    root.mkdir(parents=True)
    (root / "a.py").write_bytes(b"a")
    (root / "b.py").write_bytes(b"b")
    monkeypatch.setattr(hasher, "_MAX_CANDIDATES", 1)
    package = Package(
        name="python/candidate-cap",
        path=root,
        language="python",
        repository_root=tmp_path,
    )
    with pytest.raises(OSError, match="SOURCE_HASH_LIMIT_EXCEEDED"):
        hasher._collect_source_files(package)


def test_source_byte_limits_checked_before_reading(tmp_path: Path) -> None:
    root = tmp_path / "code/packages/python/large"
    root.mkdir(parents=True)
    source = root / "main.py"
    with source.open("wb") as target:
        target.truncate(hasher._MAX_FILE_BYTES + 1)
    package = Package(
        name="python/large", path=root, language="python", repository_root=tmp_path
    )
    with pytest.raises(OSError, match="SOURCE_HASH_LIMIT_EXCEEDED"):
        hasher.hash_package(package)
    source.write_bytes(b"x")
    with pytest.raises(OSError, match="SOURCE_HASH_LIMIT_EXCEEDED"):
        hasher._update_file_frame(
            hashlib.sha256(), "code/packages/python/large/main.py", source, root, 0
        )


@pytest.mark.skipif(os.name == "nt", reason="POSIX named pipe only")
def test_named_pipe_source_fails_without_blocking(tmp_path: Path) -> None:
    root = tmp_path / "code/packages/python/fifo"
    root.mkdir(parents=True)
    os.mkfifo(root / "main.py")
    package = Package(
        name="python/fifo", path=root, language="python", repository_root=tmp_path
    )
    with pytest.raises(OSError, match="not a regular file"):
        hasher.hash_package(package)
