#!/usr/bin/env python3
"""Run named clean-controlled WAL redo-apply mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "apply.go"


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
	run_mutation(
		"flush-after-failed-page-write",
		"written, err := pager.WritePage(write.Record.PageID, write.Record.Image)\n\t\tif err == nil {\n\t\t\terr = pager.Flush()\n\t\t}",
		"written, err := pager.WritePage(write.Record.PageID, write.Record.Image)\n\t\tif err != nil {\n\t\t\t_ = pager.Flush()\n\t\t}\n\t\tif err == nil {\n\t\t\terr = pager.Flush()\n\t\t}",
		"TestReplayEveryPageWritePrefixStopsBeforeBarrier/block-prefix-0$",
	)
	run_mutation(
		"reorder-committed-images",
		"for _, write := range plan {",
		"for index := len(plan) - 1; index >= 0; index-- {\n\t\twrite := plan[index]",
		"TestReplayWritesOnePageThenBarrierAndReturnsCompleteCount$",
	)
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
