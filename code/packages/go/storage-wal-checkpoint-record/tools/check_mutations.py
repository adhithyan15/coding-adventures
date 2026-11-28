#!/usr/bin/env python3
"""Run clean-controlled checkpoint-record validation mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "checkpoint.go"


def witness(test_name: str, case_name: str, failure: bool, label: str) -> None:
    selector = f"^{test_name}$/^{case_name}$"
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", selector, "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    marker = f"--- {'FAIL' if failure else 'PASS'}: {test_name}/{case_name}"
    if (result.returncode == 0) == failure or marker not in result.stdout:
        state = "survived" if failure and result.returncode == 0 else "did not show intended witness"
        raise SystemExit(f"{label}: mutant {state}\n{result.stdout}")
    print(f"{label}: observed {marker}")


def mutation(label: str, old: str, new: str, test_name: str, case_name: str) -> None:
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
    witness("TestDecodePayloadStructuralAndFieldOrder", "reserved", False, "clean reserved witness")
    witness("TestEncodePortableBoundsAndPrecedence", "epoch_before_rank", False, "clean epoch witness")
    mutation(
        "admit-nonzero-reserved-byte",
        "if payload[6] != 0 || payload[7] != 0 {",
        "if false {",
        "TestDecodePayloadStructuralAndFieldOrder",
        "reserved",
    )
    mutation(
        "admit-zero-epoch",
        "if record.Epoch == 0 || record.Epoch > maxPortableInteger {",
        "if record.Epoch > maxPortableInteger {",
        "TestEncodePortableBoundsAndPrecedence",
        "epoch_before_rank",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
