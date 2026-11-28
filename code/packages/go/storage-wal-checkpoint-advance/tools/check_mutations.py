#!/usr/bin/env python3
"""Run two exact clean-controlled checkpoint-advance mutations."""

from __future__ import annotations

import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "advance.go"


def witness(name: str, should_fail: bool, label: str) -> None:
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", f"^{name}$", "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    expected = "FAIL" if should_fail else "PASS"
    marker = f"--- {expected}: {name}"
    if (result.returncode != 0) != should_fail or marker not in result.stdout:
        raise SystemExit(f"{label}: expected {marker}; exit={result.returncode}\n{result.stdout}")
    print(f"{label}: observed {marker}")


def mutate(name: str, old: str, new: str, test_name: str) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    if original.count(old) != 1:
        raise SystemExit(f"{name}: expected one exact source anchor")
    SOURCE.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        witness(test_name, True, name)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> None:
    witness("TestAdvanceMemoryPublishesResettingClearsAndPublishesReady", False, "clean ordered-advance witness")
    witness("TestAdvanceSelectedResettingResumesWithoutReplay", False, "clean Resetting-resume witness")
    mutate(
        "clear-before-resetting-publication",
        "report.Resetting, err = storagewalcheckpointpublication.Publish(device, log, data, checkpoint, resetting)\n\t\tif err != nil {\n\t\t\treturn report, err\n\t\t}",
        "report.Clear, err = storagewalcheckpointclear.Clear(device, log, data, checkpoint)\n\t\tif err != nil {\n\t\t\treturn report, err\n\t\t}\n\t\treport.Resetting, err = storagewalcheckpointpublication.Publish(device, log, data, checkpoint, resetting)\n\t\tif err != nil {\n\t\t\treturn report, err\n\t\t}",
        "TestAdvanceMemoryPublishesResettingClearsAndPublishesReady",
    )
    mutate(
        "replay-selected-resetting-log",
        "case storagewalcheckpointrecord.Resetting:\n\t\tif observed.Epoch != target {",
        "case storagewalcheckpointrecord.Resetting:\n\t\tif _, err := storagewalrecovery.Recover(device, log, data); err != nil {\n\t\t\treturn report, err\n\t\t}\n\t\tif observed.Epoch != target {",
        "TestAdvanceSelectedResettingResumesWithoutReplay",
    )


if __name__ == "__main__":
    main()
