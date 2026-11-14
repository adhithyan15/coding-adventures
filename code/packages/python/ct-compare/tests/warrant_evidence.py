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

import sys
from pathlib import Path

from coding_adventures_warrant_corpus import main

PACKAGE = Path(__file__).resolve().parents[1]
REPO_ROOT = PACKAGE.parents[3]
CORPUS = REPO_ROOT / "code" / "specs" / "fixtures" / "ct-compare-warrant-v1"


def run(argv: list[str]) -> int:
    """Forward ``argv`` (``<command> [options]``) with this package's paths."""
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
    sys.exit(run(sys.argv[1:]))
