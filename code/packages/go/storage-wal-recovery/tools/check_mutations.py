#!/usr/bin/env python3
"""Run exact clean-controlled WAL recovery mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "recovery.go"


def witness(test_name: str, should_fail: bool, label: str) -> None:
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", f"^{test_name}$", "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    expected = "FAIL" if should_fail else "PASS"
    marker = f"--- {expected}: {test_name}"
    if (result.returncode != 0) != should_fail or marker not in result.stdout:
        raise SystemExit(f"{label}: expected {marker}; exit={result.returncode}\n{result.stdout}")
    print(f"{label}: observed {marker}")


def mutate(label: str, old: str, new: str, test_name: str) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    if old not in original:
        raise SystemExit(f"{label}: source anchor missing")
    SOURCE.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        witness(test_name, True, label)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 1:
        raise SystemExit("usage: python3 tools/check_mutations.py")

    witness("TestRecoverDamagedTailUsesAppendTypedErrorBeforeDataEffects", False, "clean damaged-tail refusal")
    witness("TestRecoverWithBoundaryIncludesUnfinishedTrailingRecord", False, "clean admitted-boundary witness")
    witness("TestRecoverWithBoundaryPreservesLegacyAndPartialReplayResults", False, "clean partial-replay witness")
    mutate(
        "replay-damaged-tail-prefix",
        "if scan.Tail == storagewalscan.Damaged {\n\t\treturn Result{}, &storagewalappend.DamagedTailError{Page: scan.TailPage, Cause: scan.Cause}\n\t}",
        "if scan.Tail == storagewalscan.Full {\n\t\treturn Result{}, &storagewalappend.DamagedTailError{Page: scan.TailPage, Cause: scan.Cause}\n\t}",
        "TestRecoverDamagedTailUsesAppendTypedErrorBeforeDataEffects",
    )
    mutate(
        "write-before-full-admission",
        "if scan.Tail == storagewalscan.Damaged {",
        "_ = device.WriteBlock(data.FirstBlock, make([]byte, int(data.Geometry.BytesPerBlock)))\n\tif scan.Tail == storagewalscan.Damaged {",
        "TestRecoverDamagedTailUsesAppendTypedErrorBeforeDataEffects",
    )
    mutate(
        "use-committed-rank-instead-of-last-admitted",
        "result.LastLSN = scan.Records[len(scan.Records)-1].LSN",
        "result.LastLSN = scan.Records[len(scan.Records)-2].LSN",
        "TestRecoverWithBoundaryIncludesUnfinishedTrailingRecord",
    )
    mutate(
        "expose-boundary-after-partial-replay",
        "return Result{Replay: receipt}, err",
        "return Result{Replay: receipt, LastLSN: scan.Records[len(scan.Records)-1].LSN}, err",
        "TestRecoverWithBoundaryPreservesLegacyAndPartialReplayResults",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
