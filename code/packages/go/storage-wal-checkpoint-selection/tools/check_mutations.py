#!/usr/bin/env python3
"""Run clean-controlled checkpoint selection mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "select.go"


def witness(test_name: str, case_name: str, fail: bool, label: str) -> None:
    selector = f"^{test_name}$/^{case_name}$"
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", selector, "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    marker = f"--- {'FAIL' if fail else 'PASS'}: {test_name}/{case_name}"
    if (result.returncode == 0) == fail or marker not in result.stdout:
        raise SystemExit(f"{label}: expected {marker}; command exited {result.returncode}\n{result.stdout}")
    print(f"{label}: observed {marker}")


def mutate(label: str, old: str, new: str, test_name: str, case_name: str) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    if old not in original:
        raise SystemExit(f"{label}: source anchor missing")
    SOURCE.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        witness(test_name, case_name, True, label)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 1:
        raise SystemExit("usage: python3 tools/check_mutations.py")
    witness("TestSelectionCrossings", "higher-epoch-slot-1", False, "clean epoch-winner witness")
    witness("TestSelectionCrossings", "same-epoch-conflict", False, "clean conflict witness")
    mutate(
        "choose-lower-epoch",
        "if first.Epoch > second.Epoch {",
        "if first.Epoch < second.Epoch {",
        "TestSelectionCrossings",
        "higher-epoch-slot-1",
    )
    mutate(
        "accept-conflicting-prior-ranks",
        "if first.PreviousLSN != second.PreviousLSN {",
        "if false {",
        "TestSelectionCrossings",
        "same-epoch-conflict",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
