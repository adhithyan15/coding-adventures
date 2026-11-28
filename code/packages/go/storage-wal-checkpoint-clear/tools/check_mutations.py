#!/usr/bin/env python3
"""Run exact clean-controlled Resetting-gate and barrier-ack mutations."""
from __future__ import annotations

import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "clear.go"


def witness(test: str, should_fail: bool, label: str) -> None:
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", f"^{test}$", "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    marker = f"--- {'FAIL' if should_fail else 'PASS'}: {test}"
    if (result.returncode != 0) != should_fail or marker not in result.stdout:
        raise SystemExit(f"{label}: expected {marker}; exit={result.returncode}\n{result.stdout}")
    print(f"{label}: observed {marker}")


def mutate(label: str, old: str, new: str, test: str) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    if old not in original:
        raise SystemExit(f"{label}: source anchor missing")
    SOURCE.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        witness(test, True, label)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> None:
    witness("TestClearRefusesReadyCheckpoint", False, "clean Resetting gate witness")
    witness("TestClearFlushRefusalDoesNotAcknowledge", False, "clean barrier receipt witness")
    mutate(
        "clear-under-ready-record",
        "if captured.Selection.Record.Phase != storagewalcheckpointrecord.Resetting {",
        "if false {",
        "TestClearRefusesReadyCheckpoint",
    )
    mutate(
        "acknowledge-before-barrier",
        "if err := device.Flush(); err != nil {\n\t\treturn receipt, err\n\t}\n\treceipt.Acknowledged = true",
        "receipt.Acknowledged = true\n\tif err := device.Flush(); err != nil {\n\t\treturn receipt, err\n\t}\n\treceipt.Acknowledged = true",
        "TestClearFlushRefusalDoesNotAcknowledge",
    )


if __name__ == "__main__":
    main()
