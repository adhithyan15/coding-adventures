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
