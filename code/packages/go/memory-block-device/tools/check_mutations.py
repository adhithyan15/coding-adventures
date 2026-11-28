#!/usr/bin/env python3
"""Run clean-controlled persistence and ownership source mutations."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO = shutil.which("go")
MUTATIONS = (
    (
        "persist-before-flush",
        "device.pending[block] = cloneBytes(data)",
        "device.durable[block] = cloneBytes(data)",
        "TestPendingVisibilityFlushAndRestart",
    ),
    (
        "share-caller-write-buffer",
        "device.pending[block] = cloneBytes(data)",
        "device.pending[block] = data",
        "TestPendingVisibilityFlushAndRestart",
    ),
)


def run(command: list[str], cwd: Path, expected_exit: int) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if result.returncode != expected_exit:
        raise SystemExit(
            f"unexpected exit {result.returncode}, expected {expected_exit}: {' '.join(command)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def main() -> None:
    if GO is None:
        raise SystemExit("go executable not found")
    clean = run([GO, "test", "-count=1", "./..."], PACKAGE, 0)
    print("clean: PASS")
    print(clean.stdout.strip())
    for name, anchor, replacement, witness in MUTATIONS:
        with tempfile.TemporaryDirectory(prefix=f"db06b2b1-{name}-") as temp:
            root = Path(temp) / "code" / "packages" / "go"
            root.mkdir(parents=True)
            mirror = root / "memory-block-device"
            shutil.copytree(PACKAGE, mirror, ignore=shutil.ignore_patterns("tools", "testdata"))
            shutil.copytree(PACKAGE.parent / "block-device-contract", root / "block-device-contract")
            source = mirror / "memory.go"
            contents = source.read_text(encoding="utf-8")
            if contents.count(anchor) != 1:
                raise SystemExit(f"{name}: expected exactly one source anchor")
            source.write_text(contents.replace(anchor, replacement, 1), encoding="utf-8")
            killed = run([GO, "test", "-count=1", "-run", f"^{witness}$", "."], mirror, 1)
            if "FAIL" not in killed.stdout or witness not in killed.stdout:
                raise SystemExit(f"{name}: intended test did not fail\n{killed.stdout}\n{killed.stderr}")
            print(f"{name}: KILLED by {witness}")


if __name__ == "__main__":
    main()
