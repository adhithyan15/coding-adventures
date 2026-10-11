"""Tests for cow-file selection -- the path-traversal fix for issue #12169.

Each test builds a throwaway tree like this::

    <tmp>/
      secret.cow          <- must NEVER be readable via -f
      cows/
        default.cow       <- the fallback
        tux.cow           <- a legitimate cow
        nested/inner.cow  <- exists, but "nested/inner" is not a bare name

and asserts that every hostile spelling of "secret" draws the default cow.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

# The program is a script directory, not an installed package, so make its
# modules importable. cow_path uses only the standard library on purpose.
PROGRAM_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROGRAM_DIR))

from cow_path import is_safe_cow_name, resolve_cow_path  # noqa: E402


def _write_cow(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"$the_cow = <<EOC;\n{body}\nEOC\n")


@pytest.fixture()
def cow_tree(tmp_path: Path) -> tuple[Path, Path]:
    cows = tmp_path / "cows"
    _write_cow(cows / "default.cow", "DEFAULT")
    _write_cow(cows / "tux.cow", "TUX")
    _write_cow(cows / "nested" / "inner.cow", "NESTED")
    _write_cow(tmp_path / "secret.cow", "SECRET")
    return tmp_path, cows


def _drawn(cow_name: str, cows: Path) -> str:
    """Return the body text of whichever file resolve_cow_path picked."""
    return Path(resolve_cow_path(cow_name, str(cows))).read_text()


@pytest.mark.parametrize(
    "name", ["default", "tux", "bud-frogs", "three_eyes", "v2", "dragon.and.cow"]
)
def test_bare_names_are_safe(name: str) -> None:
    assert is_safe_cow_name(name)


@pytest.mark.parametrize(
    "name",
    [
        "",
        "..",
        "../secret",
        "..\\secret",
        "a/b",
        "a\\b",
        "/etc/passwd",
        "C:secret",
        "C:\\Windows\\win",
        "tux\0",
        "..%2Fsecret",
        "%2e%2e/secret",
    ],
)
def test_hostile_names_are_unsafe(name: str) -> None:
    assert not is_safe_cow_name(name)


def test_normal_cow_names_still_load(cow_tree: tuple[Path, Path]) -> None:
    _, cows = cow_tree
    assert "TUX" in _drawn("tux", cows)
    assert "DEFAULT" in _drawn("default", cows)


def test_unknown_cow_falls_back_to_default(cow_tree: tuple[Path, Path]) -> None:
    _, cows = cow_tree
    assert "DEFAULT" in _drawn("does-not-exist", cows)


@pytest.mark.parametrize(
    "hostile", ["../secret", "..\\secret", "./../secret", "tux/../../secret"]
)
def test_relative_traversal_falls_back(
    cow_tree: tuple[Path, Path], hostile: str
) -> None:
    _, cows = cow_tree
    # Sanity: the target really is reachable by naive joining.
    assert (cows / ".." / "secret.cow").exists()
    assert "DEFAULT" in _drawn(hostile, cows)


def test_absolute_path_falls_back(cow_tree: tuple[Path, Path]) -> None:
    base, cows = cow_tree
    assert "DEFAULT" in _drawn(str(base / "secret"), cows)


@pytest.mark.parametrize("nested", ["nested/inner", "nested\\inner"])
def test_nested_names_are_refused(cow_tree: tuple[Path, Path], nested: str) -> None:
    _, cows = cow_tree
    assert "DEFAULT" in _drawn(nested, cows)


@pytest.mark.parametrize(
    "hostile", ["..%2Fsecret", "%2e%2e%2fsecret", "%2E%2E%5Csecret", "tux\0../secret"]
)
def test_encoded_and_nul_names_fall_back(
    cow_tree: tuple[Path, Path], hostile: str
) -> None:
    _, cows = cow_tree
    assert "DEFAULT" in _drawn(hostile, cows)


@pytest.mark.skipif(
    not hasattr(os, "symlink") or sys.platform == "win32",
    reason="creating symlinks needs elevated privileges on Windows",
)
def test_symlink_escape_falls_back(cow_tree: tuple[Path, Path]) -> None:
    # Layer 2 in action: "evil" is syntactically fine, but the file it names is
    # a symlink leading out of the cows directory.
    base, cows = cow_tree
    os.symlink(base / "secret.cow", cows / "evil.cow")
    assert "DEFAULT" in _drawn("evil", cows)


def test_repository_cows_load() -> None:
    cows = PROGRAM_DIR.parent.parent.parent / "specs" / "cows"
    assert _drawn("tux", cows) != _drawn("default", cows)


def test_non_string_names_are_refused() -> None:
    # The CLI always passes a str, but a library caller might not.
    for bad in (None, b"default", 42):
        assert is_safe_cow_name(bad) is False  # type: ignore[arg-type]
