#!/usr/bin/env python3
"""Run clean-controlled WAL publication/replay ordering mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "commit.go"


def run_witness(test_name: str, case_name: str, expect_failure: bool, label: str) -> None:
    selector = f"^{test_name}$" if not case_name else f"^{test_name}$/^{case_name}$"
    result = subprocess.run(
        ["go", "test", "-count=1", "-run", selector, "-v", "."],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    marker_name = f"{test_name}/{case_name}" if case_name else test_name
    marker = f"--- {'FAIL' if expect_failure else 'PASS'}: {marker_name}"
    if result.returncode == 0 and expect_failure:
        raise SystemExit(f"{label}: mutant survived\n{result.stdout}")
    if result.returncode != 0 and not expect_failure:
        raise SystemExit(f"{label}: clean witness failed\n{result.stdout}")
    if marker not in result.stdout:
        raise SystemExit(f"{label}: expected witness marker missing\n{result.stdout}")
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
        run_witness(test_name, case_name, True, label)
    finally:
        SOURCE.write_text(original, encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 1:
        raise SystemExit("usage: python3 tools/check_mutations.py")

    run_witness(
        "TestCommitLogFailureReturnsLogReceiptBeforeAnyDataWrite",
        "",
        False,
        "clean log-failure witness",
    )
    run_witness(
        "TestCommitAfterEarlierTransactionReplaysOnlyNewImages",
        "",
        False,
        "clean prior-transaction witness",
    )

    publish_anchor = "\tpublished, err := storagewalpublishcommit.Publish(device, log, data, txID, images)"
    early_replay = (
        "\tprepared, prepareErr := storagewalpreparecommit.Prepare(log, data, nil, txID, images)\n"
        "\tif prepareErr == nil {\n"
        "\t\t_, _ = storagewalredoapply.Replay(device, data, prepared)\n"
        "\t}\n"
        + publish_anchor
    )
    mutate(
        "apply-data-before-log-success",
        [(publish_anchor, early_replay)],
        "TestCommitLogFailureReturnsLogReceiptBeforeAnyDataWrite",
        "",
    )

    replay_anchor = "\treplayed, err := storagewalredoapply.Replay(device, data, published.Committed)"
    reservation_import = (
        '\tstoragewalredoapply "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-apply"\n'
    )
    expanded_import = reservation_import + (
        '\tstoragewalreservation "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation"\n'
    )
    prior_replay = (
        "\t_, priorScan, _ := storagewalreservation.Capture(device, log)\n"
        "\trecords := published.Committed\n"
        "\tpriorCount := len(priorScan.Records) - len(published.Committed)\n"
        "\tif priorCount > 0 {\n"
        "\t\trecords = priorScan.Records[:priorCount]\n"
        "\t}\n"
        "\treplayed, err := storagewalredoapply.Replay(device, data, records)"
    )
    mutate(
        "replay-previous-transaction-images",
        [(reservation_import, expanded_import), (replay_anchor, prior_replay)],
        "TestCommitAfterEarlierTransactionReplaysOnlyNewImages",
        "",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
