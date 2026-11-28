#!/usr/bin/env python3
"""Run exact clean-controlled checkpoint publication mutations."""
from __future__ import annotations

import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "publish.go"
INITIALIZE_SOURCE = ROOT / "initialize.go"


def witness(test: str, subtest: str | None, should_fail: bool, label: str) -> None:
    selector = f"^{test}$" if subtest is None else f"^{test}$/^{subtest}$"
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", selector, "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    suffix = "" if subtest is None else f"/{subtest}"
    marker = f"--- {'FAIL' if should_fail else 'PASS'}: {test}{suffix}"
    if (result.returncode != 0) != should_fail or marker not in result.stdout:
        raise SystemExit(f"{label}: expected {marker}; exit={result.returncode}\n{result.stdout}")
    print(f"{label}: observed {marker}")


def mutate(label: str, old: str, new: str, test: str, subtest: str | None, source: pathlib.Path = SOURCE) -> None:
    original = source.read_text(encoding="utf-8")
    if old not in original:
        raise SystemExit(f"{label}: source anchor missing")
    source.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        witness(test, subtest, True, label)
    finally:
        source.write_text(original, encoding="utf-8")


def main() -> None:
    witness("TestPublishThreeSteps", "block-width-13", False, "clean padded-page witness")
    witness("TestPublishBarrierRefusalsHaveExactReceipts", "width-8-flush-3", False, "clean final-ack witness")
    mutate(
        "skip-body-barrier",
        "if err := pager.Flush(); err != nil {",
        "if err := error(nil); err != nil {",
        "TestPublishThreeSteps",
        "block-width-13",
    )
    mutate(
        "confirm-before-final-ack",
        "if err := device.Flush(); err != nil {\n\t\treturn receipt, err\n\t}\n\treceipt.Confirmed = true",
        "if err := device.Flush(); err != nil {\n\t\treceipt.Confirmed = true\n\t\treturn receipt, err\n\t}\n\treceipt.Confirmed = true",
        "TestPublishBarrierRefusalsHaveExactReceipts",
        "width-8-flush-3",
    )
    witness("TestInitializeRefusesNonemptyData", None, False, "clean nonempty-data witness")
    witness("TestInitializeRefusesOccupiedAlternateSlot", None, False, "clean occupied-alternate witness")
    mutate(
        "accept-nonempty-data",
        "if octet != 0 {\n\t\t\t\treturn nonempty\n\t\t\t}",
        "if octet != 0 && false {\n\t\t\t\treturn nonempty\n\t\t\t}",
        "TestInitializeRefusesNonemptyData",
        None,
        INITIALIZE_SOURCE,
    )
    mutate(
        "overwrite-occupied-alternate-slot",
        "if states[1] != storagewalcheckpointslot.Vacant {",
        "if false {",
        "TestInitializeRefusesOccupiedAlternateSlot",
        None,
        INITIALIZE_SOURCE,
    )


if __name__ == "__main__":
    main()
