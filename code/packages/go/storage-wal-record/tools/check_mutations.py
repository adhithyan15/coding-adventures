#!/usr/bin/env python3
"""Run clean-controlled WAL field-order and image-copy mutations."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO = shutil.which("go")
KIND_THEN_LSN = '''\tif !validKind(record.Kind) {
\t\treturn nil, ErrKind
\t}
\tif record.LSN == 0 || record.LSN > maxIdentifier {
\t\treturn nil, ErrLSN
\t}'''
LSN_THEN_KIND = '''\tif record.LSN == 0 || record.LSN > maxIdentifier {
\t\treturn nil, ErrLSN
\t}
\tif !validKind(record.Kind) {
\t\treturn nil, ErrKind
\t}'''
MUTATIONS = (
    (
        "field-order-lsn-before-kind",
        KIND_THEN_LSN,
        LSN_THEN_KIND,
        "TestEncodeValidationOrder/kind-before-lsn",
        "record.go",
    ),
    (
        "omit-page-image-copy",
        "copy(payload[fixedPayloadBytes:], record.Image)",
        "copy(payload[fixedPayloadBytes:], record.Image[:len(record.Image)-1])",
        "TestExactPortableVectors/write-abc",
        "record.go",
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
    for name, old, new, witness, filename in MUTATIONS:
        with tempfile.TemporaryDirectory(prefix=f"db12a-{name}-") as temporary:
            root = Path(temporary)
            mirror = root / "storage-wal-record"
            shutil.copytree(PACKAGE, mirror, ignore=shutil.ignore_patterns("tools", "__pycache__"))
            shutil.copytree(PACKAGE.parent / "storage-record-frame", root / "storage-record-frame")
            shutil.copytree(PACKAGE.parent / "storage-crc32c", root / "storage-crc32c")
            source = mirror / filename
            text = source.read_text(encoding="utf-8")
            if text.count(old) != 1:
                raise SystemExit(f"{name}: expected exactly one mutation anchor")
            source.write_text(text.replace(old, new, 1), encoding="utf-8")
            result = run([GO, "test", "-count=1", "-run", f"^{witness}$", "."], mirror, 1)
            if "FAIL" not in result.stdout or witness.split("/")[-1] not in result.stdout:
                raise SystemExit(f"{name}: intended witness did not fail\n{result.stdout}\n{result.stderr}")
            print(f"{name}: KILLED by {witness}")


if __name__ == "__main__":
    main()
