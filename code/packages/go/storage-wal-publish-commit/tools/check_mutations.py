#!/usr/bin/env python3
"""Run clean-controlled WAL publication mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "publish.go"
TEST_NAME = "TestPublishStopsAtEveryFailedRecordBlock"
CASE_NAME = f"{TEST_NAME}/record-0-block-0"
TEST_FILTER = f"^{TEST_NAME}$/^record-0-block-0$"


def run_witness(expect_failure: bool, label: str) -> None:
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", TEST_FILTER, "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    marker = f"--- {'FAIL' if expect_failure else 'PASS'}: {CASE_NAME}"
    if result.returncode == 0 and expect_failure:
        raise SystemExit(f"{label}: mutant survived\n{result.stdout}")
    if result.returncode != 0 and not expect_failure:
        raise SystemExit(f"{label}: clean witness failed\n{result.stdout}")
    if marker not in result.stdout:
        raise SystemExit(f"{label}: expected witness marker missing\n{result.stdout}")
    print(f"{label}: observed {marker}")


def run_mutation(name: str, old: str, new: str) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    if old not in original:
        raise SystemExit(f"{name}: source anchor missing")
    SOURCE.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        run_witness(True, name)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 1:
        raise SystemExit("usage: python3 tools/check_mutations.py")
    run_witness(False, "clean control")
    stop = "if err != nil {\n\t\t\treturn result, err\n\t\t}"
    run_mutation(
        "continue-after-failed-append",
        stop,
        "if err != nil {\n\t\t\tresult.AcknowledgedRecords++\n\t\t\tcontinue\n\t\t}",
    )
    final = "var result Result\n\tfor _, record := range prepared {"
    run_mutation(
        "expose-commit-before-acknowledgement",
        final,
        "var result Result\n\tresult.Committed = prepared\n\tfor _, record := range prepared {",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
