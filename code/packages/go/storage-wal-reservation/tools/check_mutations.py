#!/usr/bin/env python3
"""Run named clean-controlled WAL reservation mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "reservation.go"


def run_mutation(name: str, old: str, new: str, test_name: str) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    if old not in original:
        raise SystemExit(f"{name}: source anchor missing")
    SOURCE.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        result = subprocess.run(
            ["go", "test", "-count=1", "-run", test_name, "."],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        if result.returncode == 0:
            raise SystemExit(f"{name}: mutant survived\n{result.stdout}")
        print(f"{name}: rejected by {test_name}")
        print(result.stdout.rstrip())
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 1:
        raise SystemExit("usage: python3 tools/check_mutations.py")
    loop = "for page := int64(0); page < layout.PageCount; page++ {"
    run_mutation(
        "stop-at-first-vacant-page",
        loop,
        "for page := int64(0); page < 1; page++ {",
		"TestCaptureRejectsBlockWidthAndPreservesHardScanErrors$",
    )
    run_mutation(
        "read-beyond-reservation",
        "\tscan, err := storagewalscan.ScanPages(pages)",
        "\tif _, err := pager.ReadPage(layout.PageCount); err != nil {\n\t\treturn storagepageio.Pager{}, storagewalscan.Scan{}, err\n\t}\n\tscan, err := storagewalscan.ScanPages(pages)",
        "TestCaptureReadsFullReservationInOrderAtNonzeroOffset$",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
