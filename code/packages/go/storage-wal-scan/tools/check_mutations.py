#!/usr/bin/env python3
"""Run clean-controlled WAL scan tail and sequence mutations."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO = shutil.which("go")
MUTATIONS = (
    (
        "stop-at-first-vacant-page",
        """if err != nil || present {
\t\t\t\treturn Scan{}, ErrPageAfterTail
\t\t\t}
\t\t\tcontinue""",
        "return Scan{Records: records, Tail: tail, TailPage: tailPage, Cause: cause}, nil",
        "TestTailRejectsLaterOccupiedOrDamaged/occupied_after_vacant",
    ),
    (
        "admit-skipped-rank",
        "record.LSN != nextLSN",
        "record.LSN < nextLSN",
        "TestRejectsMissingAndRepeatedRanks/missing_rank",
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
        with tempfile.TemporaryDirectory(prefix=f"db12b2-{name}-") as temporary:
            root = Path(temporary)
            mirror = root / "storage-wal-scan"
            shutil.copytree(PACKAGE, mirror, ignore=shutil.ignore_patterns("tools", "__pycache__"))
            for dependency in ("storage-wal-page-slot", "storage-wal-record", "storage-record-frame", "storage-crc32c"):
                shutil.copytree(PACKAGE.parent / dependency, root / dependency)
            source = mirror / "scan.go"
            text = source.read_text(encoding="utf-8")
            if text.count(old) != 1:
                raise SystemExit(f"{name}: expected exactly one source anchor")
            source.write_text(text.replace(old, new, 1), encoding="utf-8")
            result = run([GO, "test", "-count=1", "-run", f"^{witness}$", "."], mirror, 1)
            if "FAIL" not in result.stdout or witness.split("/")[-1] not in result.stdout:
                raise SystemExit(f"{name}: intended witness did not fail\n{result.stdout}\n{result.stderr}")
            print(f"{name}: KILLED by {witness}")


if __name__ == "__main__":
    main()
