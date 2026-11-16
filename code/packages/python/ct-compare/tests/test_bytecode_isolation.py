"""A forged ``__pycache__/*.pyc`` in the source tree must never be executed.

The probe module's source says ``VALUE = 1``; a forged *unchecked-hash* pyc
(PEP 552) beside it says ``VALUE = 2``.  CPython never revalidates such a pyc, so
a plain run imports the forgery -- the control below proves the attack is real.
The entry script's ``isolate_bytecode_cache`` re-runs under a private
``sys.pycache_prefix`` before anything is imported, so the forgery is never
consulted, whatever ran in the checkout before and in whatever order.
"""

from __future__ import annotations

import os
import py_compile
import shutil
import subprocess
import sys
import textwrap
from pathlib import Path

TESTS = Path(__file__).resolve().parent


def forge(directory: Path) -> None:
    """Write ``probe_mod.py`` (VALUE = 1) and a forged hash-based pyc (VALUE = 2)."""
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "probe_mod.py").write_text("VALUE = 1\n", encoding="utf-8")
    evil = directory / "evil_source.py"
    evil.write_text("VALUE = 2\n", encoding="utf-8")
    cache = directory / "__pycache__"
    cache.mkdir()
    target = cache / f"probe_mod.{sys.implementation.cache_tag}.pyc"
    py_compile.compile(
        str(evil),
        cfile=str(target),
        doraise=True,
        invalidation_mode=py_compile.PycInvalidationMode.UNCHECKED_HASH,
    )
    evil.unlink()


def drive(tmp_path: Path, isolate: bool) -> str:
    probe = tmp_path / "probe"
    forge(probe)
    driver = tmp_path / "driver.py"
    driver.write_text(
        textwrap.dedent(
            f"""
            import sys
            sys.path.insert(0, {str(TESTS)!r})
            import warrant_evidence
            if {isolate!r}:
                warrant_evidence.isolate_bytecode_cache()
            sys.path.insert(0, {str(probe)!r})
            import probe_mod
            print("VALUE", probe_mod.VALUE, "prefix", sys.pycache_prefix is not None)
            """
        ),
        encoding="utf-8",
    )
    env = {k: v for k, v in os.environ.items() if k != "PYTHONPYCACHEPREFIX"}
    env.pop("PYTHONDONTWRITEBYTECODE", None)
    done = subprocess.run(  # noqa: S603 - fixed argv
        [sys.executable, str(driver)],
        env=env,
        capture_output=True,
        text=True,
        check=True,
    )
    return done.stdout.strip()


def test_the_forgery_really_runs_without_isolation(tmp_path: Path) -> None:
    # The control: the attack works against a plain interpreter.
    assert drive(tmp_path, isolate=False) == "VALUE 2 prefix False"


def test_a_forged_pyc_is_never_run_under_the_isolated_cache(tmp_path: Path) -> None:
    assert drive(tmp_path, isolate=True) == "VALUE 1 prefix True"


def test_the_private_cache_is_removed_afterwards(tmp_path: Path) -> None:
    import tempfile

    before = set(os.listdir(tempfile.gettempdir()))
    drive(tmp_path, isolate=True)
    leaked = [
        n
        for n in set(os.listdir(tempfile.gettempdir())) - before
        if n.startswith("warrant-pycache-")
    ]
    assert leaked == []


def test_the_entry_script_runs_the_runner_under_the_isolated_cache() -> None:
    """End to end: ``check`` through the real entry script reports the manifest ok."""
    package = TESTS.parent
    env = {k: v for k, v in os.environ.items() if k != "PYTHONPYCACHEPREFIX"}
    done = subprocess.run(  # noqa: S603 - fixed argv
        [sys.executable, str(TESTS / "warrant_evidence.py"), "check",
         str(package / "evidence" / "test-manifest.json")],
        env=env, capture_output=True, text=True, check=False,
    )  # fmt: skip
    assert done.returncode == 0, done.stdout + done.stderr
    assert "ok manifest test-manifest.json matches" in done.stdout


def forged_runner_copy(tmp_path: Path) -> Path:
    """A copy of the runner source whose ``cli`` and ``__init__`` have forged pycs.

    The forgery exits with a marker, so any process that imports the runner from
    this copy through the cache prints it and fails.
    """
    runner = TESTS.parent.parent / "warrant-corpus" / "src"
    copy = tmp_path / "runner-src"
    shutil.copytree(runner, copy, ignore=shutil.ignore_patterns("__pycache__"))
    package = copy / "coding_adventures_warrant_corpus"
    evil = tmp_path / "evil.py"
    marker = "raise SystemExit('FORGED RUNNER PYC EXECUTED')\n"
    evil.write_text(marker, encoding="utf-8")
    cache = package / "__pycache__"
    cache.mkdir()
    for module in ("__init__", "cli", "manifest", "isolation", "__main__"):
        py_compile.compile(
            str(evil),
            cfile=str(cache / f"{module}.{sys.implementation.cache_tag}.pyc"),
            doraise=True,
            invalidation_mode=py_compile.PycInvalidationMode.UNCHECKED_HASH,
        )
    return copy


def build_lines(name: str) -> list[str]:
    text = (TESTS.parent / name).read_text(encoding="utf-8")
    return [
        line.strip()
        for line in text.splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def test_no_build_line_imports_the_runner_outside_the_isolated_entry() -> None:
    # Both BUILD fronts: every line that starts a Python process must go through
    # the entry script (which re-runs under a private pycache prefix); none may
    # use `-m` of the runner or pytest directly.
    for name in ("BUILD", "BUILD_windows"):
        python_lines = [
            line
            for line in build_lines(name)
            if line.startswith((".venv/bin/python", ".venv\\Scripts\\python"))
        ]
        assert len(python_lines) >= 4, name
        for line in python_lines:
            assert "tests/warrant_evidence.py" in line.replace("\\", "/"), (name, line)
            assert " -m coding_adventures_warrant_corpus" not in line, (name, line)
        joined = "\n".join(python_lines)
        for command in ("exec -m pytest", "check", "sabotage", "validate-manifest"):
            assert command in joined, (name, command)


def test_a_forged_runner_pyc_runs_when_started_directly(tmp_path: Path) -> None:
    # The control: the old BUILD line (`python -m coding_adventures_warrant_corpus`)
    # executes the forgery, which is why it is no longer used.
    copy = forged_runner_copy(tmp_path)
    env = {k: v for k, v in os.environ.items() if k != "PYTHONPYCACHEPREFIX"}
    env["PYTHONPATH"] = str(copy)
    done = subprocess.run(  # noqa: S603 - fixed argv
        [sys.executable, "-m", "coding_adventures_warrant_corpus", "list-faults"],
        env=env, capture_output=True, text=True, check=False,
    )  # fmt: skip
    assert "FORGED RUNNER PYC EXECUTED" in done.stdout + done.stderr


def test_a_forged_runner_pyc_is_never_run_by_an_entry_command(tmp_path: Path) -> None:
    copy = forged_runner_copy(tmp_path)
    package = TESTS.parent
    repo = package.parents[3]
    corpus = repo / "code/specs/fixtures/ct-compare-warrant-v1"
    env = {k: v for k, v in os.environ.items() if k != "PYTHONPYCACHEPREFIX"}
    env["PYTHONPATH"] = os.pathsep.join([str(copy), str(package / "src")])
    manifest = str(package / "evidence" / "test-manifest.json")
    commands = {
        "validate-manifest": [
            "validate-manifest", manifest,
            "--registry", str(corpus / "consumers.json"),
        ],
        "check": ["check", manifest],
        "exec": ["exec", "-m", "pytest", "--version", "-p", "no:cacheprovider"],
    }  # fmt: skip
    for name, args in commands.items():
        done = subprocess.run(  # noqa: S603 - fixed argv
            [sys.executable, str(TESTS / "warrant_evidence.py"), *args],
            env=env, capture_output=True, text=True, check=False,
        )  # fmt: skip
        output = done.stdout + done.stderr
        assert "FORGED RUNNER PYC EXECUTED" not in output, (name, output)
        assert done.returncode == 0, (name, output)
