"""Run the Warrant corpus against this package (VER04i, the Python lane).

This is the consumer registry's ``runner_entry`` for ``python/ct-compare``.  It
fixes the four package-specific paths and forwards the command to the shared
runner, ``coding_adventures_warrant_corpus``::

    python tests/warrant_evidence.py run --out evidence/test-manifest.json
    python tests/warrant_evidence.py check evidence/test-manifest.json
    python tests/warrant_evidence.py sabotage

``run --out`` regenerates the committed manifest from a real execution;
``check`` regenerates it in memory and fails if the committed file drifted;
``sabotage`` proves a deliberately broken export makes the runner print
``not ok`` and exit non-zero, once per negative control.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

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
    removes it afterwards and exits with the child's status.  In the child the
    prefix is set and this returns.  The runner then checks, by name, that the
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
            [sys.executable, *sys.argv], env=env, check=False
        ).returncode
    finally:
        shutil.rmtree(private, ignore_errors=True)
    raise SystemExit(code)


def run(argv: list[str]) -> int:
    """Forward ``argv`` (``<command> [options]``) with this package's paths."""
    from coding_adventures_warrant_corpus import main

    command, rest = argv[0], argv[1:]
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
