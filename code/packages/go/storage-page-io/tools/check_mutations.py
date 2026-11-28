#!/usr/bin/env python3
"""Run two clean-controlled page-I/O source mutations."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO = shutil.which("go")
DEPENDENCIES = (
    "block-device-contract",
    "storage-page-layout",
    "memory-block-device",
    "hdd-write-crash-cuts",
    "hdd-writeback-cache",
    "hdd-block-device",
)
MUTATIONS = (
    (
        "skip-first-error-in-page-read",
        "if err != nil {\n\t\t\treturn nil, err\n\t\t}",
        "if err != nil && offset == 0 {\n\t\t\treturn nil, err\n\t\t}",
        "TestScriptedReadOffsetsFirstErrorsWidthsAndDetachment",
    ),
    (
        "write-at-the-wrong-block-offset",
        "pager.device.WriteBlock(span.FirstBlock+offset, data[start:end])",
        "pager.device.WriteBlock(span.FirstBlock+offset+1, data[start:end])",
        "TestScriptedWriteCallsExactOffsetsAndPrefixCounts",
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
        with tempfile.TemporaryDirectory(prefix=f"db11b-{name}-") as temp:
            root = Path(temp) / "code" / "packages" / "go"
            root.mkdir(parents=True)
            mirror = root / "storage-page-io"
            shutil.copytree(PACKAGE, mirror, ignore=shutil.ignore_patterns("tools", "testdata"))
            for dependency in DEPENDENCIES:
                shutil.copytree(PACKAGE.parent / dependency, root / dependency)
            source = mirror / "page_io.go"
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
