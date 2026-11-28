#!/usr/bin/env python3
"""Run named clean-controlled checkpoint capture mutations."""
from __future__ import annotations

import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "capture.go"


def witness(test: str, subtest: str | None, fail: bool, label: str) -> None:
    selector = f"^{test}$" if subtest is None else f"^{test}$/^{subtest}$"
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", selector, "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    marker = f"--- {'FAIL' if fail else 'PASS'}: {test}"
    if subtest is not None:
        marker += f"/{subtest}"
    if (result.returncode != 0) != fail or marker not in result.stdout:
        raise SystemExit(f"{label}: expected {marker}; exit={result.returncode}\n{result.stdout}")
    print(f"{label}: observed {marker}")


def mutate(label: str, old: str, new: str, test: str, subtest: str | None) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    if old not in original:
        raise SystemExit(f"{label}: source anchor missing")
    SOURCE.write_text(original.replace(old, new, 1), encoding="utf-8")
    try:
        witness(test, subtest, True, label)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> None:
    witness("TestCaptureReadOrderAndSelect", "width-16", False, "clean physical-order witness")
    witness("TestCapturePaddingPrecedesFrameSelection", None, False, "clean padding witness")
    mutate(
        "read-pages-in-reverse-ordinal",
        "pager.ReadPage(int64(ordinal))",
        "pager.ReadPage(int64(1 - ordinal))",
        "TestCaptureReadOrderAndSelect",
        "width-16",
    )
    mutate(
        "ignore-nonzero-padding",
        "if value != 0 {",
        "if value != 0 && len(page) == 0 {",
        "TestCapturePaddingPrecedesFrameSelection",
        None,
    )


if __name__ == "__main__":
    main()
