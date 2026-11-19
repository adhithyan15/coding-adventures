"""Run focused runtime controls against intentionally broken source copies."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src/fenwick_tree/fenwick_tree.py"
TEST = ROOT / "tests/test_fenwick_tree_v1.py"
FIXTURE = ROOT.parents[2] / "specs/fixtures/fenwick-tree-v1/cases.json"
BOOTSTRAP = ROOT.parent / "test-bootstrap/src"

MUTATIONS = {
    "omit update propagation": (
        "self._bit[i] += delta",
        "self._bit[i] += 0",
        "test_exhaustive_signed_arrays_updates_and_all_queries",
    ),
    "skip constructor domain check": (
        "if magnitude > _PORTABLE_INTEGER_LIMIT - absolute_total:",
        "if False:",
        "test_shared_fixture_uses_public_api_and_preserves_errors",
    ),
    "skip update delta bound": (
        "if abs(delta) > _PORTABLE_INTEGER_LIMIT:",
        "if False:",
        "test_shared_fixture_uses_public_api_and_preserves_errors",
    ),
    "skip resulting state bound": (
        "if new_magnitude > _PORTABLE_INTEGER_LIMIT - other_absolute_total:",
        "if False:",
        "test_shared_fixture_uses_public_api_and_preserves_errors",
    ),
}


def main() -> None:
    source = SOURCE.read_text(encoding="utf-8")
    with tempfile.TemporaryDirectory(prefix="fenwick-mutations-") as scratch:
        base = Path(scratch)
        for name, (before, after, expected_test) in MUTATIONS.items():
            if source.count(before) != 1:
                raise RuntimeError(f"mutation anchor is not unique: {name}")
            package = base / name.replace(" ", "-")
            (package / "src/fenwick_tree").mkdir(parents=True)
            (package / "tests").mkdir()
            shutil.copy2(
                ROOT / "src/fenwick_tree/__init__.py",
                package / "src/fenwick_tree/__init__.py",
            )
            (package / "src/fenwick_tree/fenwick_tree.py").write_text(
                source.replace(before, after), encoding="utf-8"
            )
            shutil.copy2(TEST, package / "tests/test_fenwick_tree_v1.py")
            env = os.environ.copy()
            env["PYTHONPATH"] = os.pathsep.join(
                [str(package / "src"), str(BOOTSTRAP)]
            )
            env["FENWICK_FIXTURE"] = str(FIXTURE)
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "pytest",
                    "-p",
                    "no:platformdirs",
                    "tests/test_fenwick_tree_v1.py",
                    "-q",
                ],
                cwd=package,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            output = result.stdout + result.stderr
            expected_failure = f"FAILED tests/test_fenwick_tree_v1.py::{expected_test}"
            if result.returncode != 1 or expected_failure not in output:
                raise RuntimeError(f"undetected or invalid mutation {name}:\n{output}")
            print(f"Detected runtime mutation: {name}")
    print(f"Detected {len(MUTATIONS)}/{len(MUTATIONS)} source mutations.")


if __name__ == "__main__":
    main()
