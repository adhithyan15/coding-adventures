"""A file beside the entry script must never be imported before isolation.

Run as a script, CPython puts the script's directory first on ``sys.path``.  The
entry imports ``shutil``, ``subprocess`` and ``tempfile`` to build the private
bytecode cache, so a planted ``tests/shutil.py`` -- or an extension module
``shutil.cpython-*.so``, ``shutil.abi3.so``, ``shutil.pyd`` -- would run before
any isolation exists (VER04i 8.1, WD-004b3).  The entry drops its own directory
from ``sys.path`` before importing anything and re-runs with ``-P``.

Each test copies the entry beside a planted module whose import leaves a marker
file, runs the copy, and asserts there is no marker.  The control shows that a
plain script in the same directory really does import the planted module.
"""

from __future__ import annotations

import importlib.machinery
import os
import subprocess
import sys
from pathlib import Path

import pytest

ENTRY = Path(__file__).resolve().parent / "warrant_evidence.py"
STDLIB_NAMES = ("shutil", "subprocess", "tempfile")


def planted_source(marker: Path) -> str:
    return (
        f"open({str(marker)!r}, 'w').write('planted module ran')\n"
        "raise SystemExit(97)\n"
    )


def copy_entry(root: Path) -> Path:
    """Copy the entry into ``root/a/b/pkg/tests`` (it needs four parent levels)."""
    tests = root / "a" / "b" / "pkg" / "tests"
    tests.mkdir(parents=True)
    entry = tests / "warrant_evidence.py"
    entry.write_text(ENTRY.read_text(encoding="utf-8"), encoding="utf-8")
    return entry


def run(args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    env = {
        k: v
        for k, v in os.environ.items()
        if k not in ("PYTHONPYCACHEPREFIX", "PYTHONPATH", "PYTHONSAFEPATH")
    }
    return subprocess.run(  # noqa: S603 - fixed argv
        [sys.executable, *args],
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def test_the_planted_module_really_shadows_a_plain_script(tmp_path: Path) -> None:
    # The control: nothing special protects an ordinary script in that directory.
    marker = tmp_path / "marker"
    (tmp_path / "shutil.py").write_text(planted_source(marker), encoding="utf-8")
    (tmp_path / "plain.py").write_text("import shutil\n", encoding="utf-8")
    done = run([str(tmp_path / "plain.py")], tmp_path)
    assert marker.exists()
    assert done.returncode == 97


@pytest.mark.parametrize("name", STDLIB_NAMES)
def test_a_planted_source_module_is_never_imported(tmp_path: Path, name: str) -> None:
    marker = tmp_path / "marker"
    entry = copy_entry(tmp_path)
    (entry.parent / f"{name}.py").write_text(planted_source(marker), encoding="utf-8")
    done = run([str(entry), "exec", "-m", "this"], tmp_path)
    assert not marker.exists(), done.stderr
    assert done.returncode == 0, done.stderr
    assert "Zen of Python" in done.stdout  # the real stdlib ran the command


@pytest.mark.parametrize("suffix", [*importlib.machinery.EXTENSION_SUFFIXES, ".pyd"])
def test_a_planted_extension_module_is_never_imported(
    tmp_path: Path, suffix: str
) -> None:
    # Not a valid library: if the import system reached it, the import would fail.
    entry = copy_entry(tmp_path)
    for name in STDLIB_NAMES:
        (entry.parent / f"{name}{suffix}").write_bytes(b"not a shared object")
    done = run([str(entry), "exec", "-m", "this"], tmp_path)
    assert done.returncode == 0, done.stderr
    assert "Zen of Python" in done.stdout


def test_a_planted_module_is_not_imported_from_the_working_directory(
    tmp_path: Path,
) -> None:
    # `-P` in the re-run also keeps the working directory off sys.path.
    marker = tmp_path / "marker"
    cwd = tmp_path / "cwd"
    cwd.mkdir()
    (cwd / "shutil.py").write_text(planted_source(marker), encoding="utf-8")
    entry = copy_entry(tmp_path)
    done = run([str(entry), "exec", "-m", "this"], cwd)
    assert not marker.exists(), done.stderr
    assert done.returncode == 0, done.stderr
