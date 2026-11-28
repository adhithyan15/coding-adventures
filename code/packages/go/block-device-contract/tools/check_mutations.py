#!/usr/bin/env python3
"""Run two clean-controlled source mutations for block-device validation."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO = shutil.which("go")

MUTATIONS = (
    (
        "accept-exclusive-upper-bound",
        "block >= geometry.BlockCount",
        "block > geometry.BlockCount",
        "TestCheckBlockSignedLimitsAndValidationPrecedence/at_count",
    ),
    (
        "write-width-before-address",
        "if err := geometry.CheckBlock(block); err != nil {\n\t\treturn err\n\t}\n\tif len(data) != int(geometry.BytesPerBlock) {",
        "if len(data) != int(geometry.BytesPerBlock) {",
        "TestCheckWriteValidationPrecedenceAndExactWidth/address_before_width",
    ),
)


def run(command: list[str], cwd: Path, *, expect: int) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if result.returncode != expect:
        raise SystemExit(
            f"unexpected exit {result.returncode}, expected {expect}: {' '.join(command)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def main() -> None:
    if GO is None:
        raise SystemExit("go executable not found")
    baseline = run([GO, "test", "-count=1", "./..."], PACKAGE, expect=0)
    print("clean: PASS")
    print(baseline.stdout.strip())

    for name, old, new, witness in MUTATIONS:
        with tempfile.TemporaryDirectory(prefix=f"db06b2a-{name}-") as temp:
            mirror = Path(temp) / "package"
            shutil.copytree(PACKAGE, mirror, ignore=shutil.ignore_patterns(".git", "tools", "testdata"))
            source = mirror / "device.go"
            text = source.read_text(encoding="utf-8")
            if text.count(old) != 1:
                raise SystemExit(f"{name}: expected exactly one source anchor")
            source.write_text(text.replace(old, new, 1), encoding="utf-8")
            result = run([GO, "test", "-count=1", "-run", f"^{witness}$", "."], mirror, expect=1)
            if "FAIL" not in result.stdout or witness.split("/")[-1] not in result.stdout:
                raise SystemExit(f"{name}: intended witness did not fail\n{result.stdout}\n{result.stderr}")
            print(f"{name}: KILLED by {witness}")


if __name__ == "__main__":
    main()
