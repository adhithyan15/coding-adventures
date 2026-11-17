"""Run the Warrant corpus against this package (VER04i, the Python lane).

This is the consumer registry's ``runner_entry`` for ``python/ct-compare``.  It
fixes the four package-specific paths and forwards the command to the shared
runner, ``coding_adventures_warrant_corpus``::

    python tests/warrant_evidence.py run --out evidence/test-manifest.json
    python tests/warrant_evidence.py check evidence/test-manifest.json
    python tests/warrant_evidence.py sabotage
    python tests/warrant_evidence.py validate-manifest evidence/test-manifest.json
    python tests/warrant_evidence.py exec -m pytest tests/

``run --out`` regenerates the committed manifest from a real execution;
``check`` regenerates it in memory and fails if the committed file drifted;
``sabotage`` proves a deliberately broken export makes the runner print
``not ok`` and exit non-zero, once per negative control.
"""

from __future__ import annotations

import sys

# VER04i 8.1 (WD-004b3): run as a script, CPython puts THIS file's directory first
# on ``sys.path``, so a planted ``tests/shutil.py`` (or ``shutil.cpython-*.so``,
# ``.abi3.so``, ``.pyd``) would be imported by the ``import shutil`` below, before any
# isolation exists.  Drop that entry before the first import that is not already
# loaded: ``sys`` is built in and ``os`` is loaded by interpreter start-up.  The
# re-run below also passes ``-P`` (Python 3.11+, this lane requires 3.12), which
# never adds the script directory at all.  Only when run as a script: a test that
# imports this module must keep the path it was given.
if __name__ == "__main__" and not sys.flags.safe_path:
    del sys.path[0]

import os  # noqa: E402
from pathlib import Path  # noqa: E402

PACKAGE = Path(__file__).resolve().parents[1]
REPO_ROOT = PACKAGE.parents[3]
CORPUS = REPO_ROOT / "code" / "specs" / "fixtures" / "ct-compare-warrant-v1"


# The variable the runner checks (coding_adventures_warrant_corpus.isolation).
ENV_PRIVATE = "WARRANT_PYCACHE_PRIVATE"


def isolate_bytecode_cache() -> None:
    """Re-run this script under a fresh private bytecode cache (VER04i 8.1).

    CPython runs a cached ``__pycache__/*.pyc`` instead of its source, and a
    forged one cannot be told from a real one.  With ``sys.pycache_prefix`` set
    it reads and writes bytecode only under that directory, so the checkout's
    caches are never consulted -- whatever ran before this script, in whatever
    order.  This must run BEFORE anything from the checkout is imported, so the
    runner and the library under test are loaded without them.

    The first call (no prefix yet) creates a private, empty directory, runs the
    same command line in a child with ``PYTHONPYCACHEPREFIX`` pointing at it,
    removes it afterwards and exits with the child's status.  The child runs
    with ``-P`` so the script's own directory is not on ``sys.path`` either, and
    the imports below happen with nothing beside this file importable.  In the
    child the prefix is set and this returns.  The runner then checks, by name, that the
    directory is the one this function created.
    """
    if sys.pycache_prefix is not None:
        return
    import shutil
    import subprocess
    import tempfile

    private = tempfile.mkdtemp(prefix="warrant-pycache-")
    try:
        if os.listdir(private):
            raise SystemExit("error: the private bytecode cache is not empty")
        env = dict(os.environ, PYTHONPYCACHEPREFIX=private, **{ENV_PRIVATE: private})
        code = subprocess.run(  # noqa: S603 - same interpreter, same script
            [sys.executable, "-P", *sys.argv], env=env, check=False
        ).returncode
    finally:
        shutil.rmtree(private, ignore_errors=True)
    raise SystemExit(code)


def run(argv: list[str]) -> int:
    """Forward ``argv`` (``<command> [options]``) with this package's paths."""
    command, rest = argv[0], argv[1:]
    if command == "exec":
        # `exec -m MODULE [args]`: run another module (the BUILD's pytest) in this
        # same isolated interpreter, so no BUILD line imports the runner or the
        # library outside the private bytecode cache.
        import runpy

        if len(rest) < 2 or rest[0] != "-m":
            raise SystemExit("usage: warrant_evidence.py exec -m MODULE [args]")
        sys.argv = [rest[1], *rest[2:]]
        runpy.run_module(rest[1], run_name="__main__", alter_sys=True)
        return 0
    from coding_adventures_warrant_corpus import main

    common = [
        "--repo-root", str(REPO_ROOT),
        "--corpus", str(CORPUS),
        "--binding", str(PACKAGE / "evidence.bindings"),
        "--package-dir", str(PACKAGE),
    ]  # fmt: skip
    if command == "check":  # `check <manifest>`: the positional goes last
        return main(["check", *common, *rest])
    return main([command, *common, *rest])


if __name__ == "__main__":
    isolate_bytecode_cache()
    sys.exit(run(sys.argv[1:]))
