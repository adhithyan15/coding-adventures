#!/usr/bin/env python3
"""Run two clean-controlled CRC-32C semantic mutations."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO = shutil.which("go")
MUTATIONS = (
    (
        "wrong-reflected-polynomial",
        "const reflectedCastagnoli uint32 = 0x82f63b78",
        "const reflectedCastagnoli uint32 = 0x82f63b79",
        "TestPortableVectors/standard-check",
    ),
    (
        "reset-register-instead-of-continuing",
        "register := ^previous",
        "register := ^uint32(0)",
        "TestUpdateSeedsAndEveryShortSplit",
    ),
)


def run(command: list[str], cwd: Path, expected: int) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if result.returncode != expected:
        raise SystemExit(
            f"unexpected exit {result.returncode}, expected {expected}: {' '.join(command)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def main() -> None:
    if GO is None:
        raise SystemExit("go executable not found")
    baseline = run([GO, "test", "-count=1", "./..."], PACKAGE, 0)
    print("clean: PASS")
    print(baseline.stdout.strip())
    for name, old, new, witness in MUTATIONS:
        with tempfile.TemporaryDirectory(prefix=f"db10a-{name}-") as temporary:
            mirror = Path(temporary) / "package"
            shutil.copytree(PACKAGE, mirror, ignore=shutil.ignore_patterns("tools", "__pycache__"))
            source = mirror / "crc32c.go"
            text = source.read_text(encoding="utf-8")
            if text.count(old) != 1:
                raise SystemExit(f"{name}: expected one source anchor")
            source.write_text(text.replace(old, new, 1), encoding="utf-8")
            result = run([GO, "test", "-count=1", "-run", f"^{witness}$", "."], mirror, 1)
            if "FAIL" not in result.stdout or witness.split("/")[-1] not in result.stdout:
                raise SystemExit(f"{name}: witness did not fail as intended\n{result.stdout}\n{result.stderr}")
            print(f"{name}: KILLED by {witness}")


if __name__ == "__main__":
    main()
