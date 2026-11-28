#!/usr/bin/env python3
"""Run clean-controlled checkpoint slot publication mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "slot.go"


def witness(test_name: str, case_name: str, failure: bool, label: str) -> None:
    selector = f"^{test_name}$/^{case_name}$" if case_name else f"^{test_name}$"
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", selector, "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    full_name = f"{test_name}/{case_name}" if case_name else test_name
    marker = f"--- {'FAIL' if failure else 'PASS'}: {full_name}"
    if (result.returncode == 0) == failure or marker not in result.stdout:
        raise SystemExit(f"{label}: expected {marker}; command exited {result.returncode}\n{result.stdout}")
    print(f"{label}: observed {marker}")


def mutate(label: str, edits: list[tuple[str, str]], test_name: str, case_name: str) -> None:
    original = SOURCE.read_text(encoding="utf-8")
    changed = original
    for old, new in edits:
        if old not in changed:
            raise SystemExit(f"{label}: source anchor missing: {old!r}")
        changed = changed.replace(old, new, 1)
    SOURCE.write_text(changed, encoding="utf-8")
    try:
        witness(test_name, case_name, True, label)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 1:
        raise SystemExit("usage: python3 tools/check_mutations.py")
    witness("TestInspectAllGuardBytesAndSlotKinds", "guard-a5", False, "clean staged-guard witness")
    witness("TestStageMatchesFinalFrameWithOnlyStagedGuardChange", "", False, "clean checksum witness")
    mutate(
        "admit-staged-body-before-guard-publication",
        [
            (
                "if slot[GuardOffset] == StagedGuard {\n\t\treturn Staged, storagewalcheckpointrecord.Record{}, nil\n\t}",
                "if slot[GuardOffset] == StagedGuard {\n"
                "\t\tpublished := append([]byte(nil), slot...)\n"
                "\t\tpublished[GuardOffset] = 0\n"
                "\t\trecord, err := storagewalcheckpointrecord.Decode(published)\n"
                "\t\treturn Published, record, err\n\t}",
            )
        ],
        "TestInspectAllGuardBytesAndSlotKinds",
        "guard-a5",
    )
    import_line = '\tstoragewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"\n'
    expanded_import = import_line + '\tstoragerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"\n'
    stage_lines = "\tframe[GuardOffset] = StagedGuard\n\treturn frame, nil"
    recompute = (
        "\tpayload, _ := storagerecordframe.Decode(frame)\n"
        "\tpayload[6] = StagedGuard\n"
        "\tframe, _ = storagerecordframe.Encode(payload)\n"
        "\treturn frame, nil"
    )
    mutate(
        "checksum-includes-staged-guard",
        [(import_line, expanded_import), (stage_lines, recompute)],
        "TestStageMatchesFinalFrameWithOnlyStagedGuardChange",
        "",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
