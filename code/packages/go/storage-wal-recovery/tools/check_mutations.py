#!/usr/bin/env python3
"""Run clean-controlled WAL recovery mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "recovery.go"


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
    tail = "if scan.Tail == storagewalscan.Damaged {\n\t\treturn storagewalredoapply.Receipt{}, &storagewalappend.DamagedTailError{Page: scan.TailPage, Cause: scan.Cause}\n\t}"
    run_mutation(
        "replay-damaged-tail-prefix",
        tail,
        "if scan.Tail == storagewalscan.Full {\n\t\treturn storagewalredoapply.Receipt{}, &storagewalappend.DamagedTailError{Page: scan.TailPage, Cause: scan.Cause}\n\t}",
        "TestRecoverDamagedTailUsesAppendTypedErrorBeforeDataEffects$",
    )
    before_admission = "if scan.Tail == storagewalscan.Damaged {"
    run_mutation(
        "write-before-full-admission",
        before_admission,
        "_ = device.WriteBlock(data.FirstBlock, make([]byte, int(data.Geometry.BytesPerBlock)))\n\tif scan.Tail == storagewalscan.Damaged {",
        "TestRecoverDamagedTailUsesAppendTypedErrorBeforeDataEffects$",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
