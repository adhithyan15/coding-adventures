#!/usr/bin/env python3
"""Prove Ready-epoch refusal checks with exact clean-controlled witnesses."""
from __future__ import annotations

import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "ready_commit.go"


def witness(test_name: str, expect_failure: bool, label: str) -> None:
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", f"^{test_name}$", "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    marker = f"--- {'FAIL' if expect_failure else 'PASS'}: {test_name}"
    if (result.returncode != 0) != expect_failure or marker not in result.stdout:
        raise SystemExit(f"{label}: expected {marker}; exit={result.returncode}\n{result.stdout}")
    print(f"{label}: {marker}")


def mutate(label: str, old: str, new: str, test_name: str) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    if old not in original:
        raise SystemExit(f"{label}: source anchor not found")
    SOURCE.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        witness(test_name, True, label)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> None:
    resetting = "\tif selected.Phase == storagewalcheckpointrecord.Resetting {\n\t\treturn Result{}, ErrCheckpointResetting\n\t}"
    epoch = "\tif selected.Epoch != epoch {\n\t\treturn Result{}, ErrCheckpointEpoch\n\t}"
    witness("TestResettingPrecedesEpochMismatchAndHasNoEffects", False, "clean Resetting precedence")
    witness("TestReadyEpochMismatchHasNoEffects", False, "clean epoch mismatch")
    mutate("Resetting admission mutant", resetting, "", "TestResettingPrecedesEpochMismatchAndHasNoEffects")
    mutate("epoch admission mutant", epoch, "", "TestReadyEpochMismatchHasNoEffects")


if __name__ == "__main__":
    main()
