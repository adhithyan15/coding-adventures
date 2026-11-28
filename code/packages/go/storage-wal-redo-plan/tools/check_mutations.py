#!/usr/bin/env python3
"""Run named clean-controlled WAL redo-plan mutations."""

from __future__ import annotations

import pathlib
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "plan.go"


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
		"select-uncommitted-write",
		"\treturn plan, nil\n}",
		"\tfor _, state := range transactions {\n\t\tif state.active {\n\t\t\tfor _, record := range state.writes {\n\t\t\t\tplan = append(plan, RedoWrite{Record: record})\n\t\t\t}\n\t\t}\n\t}\n\treturn plan, nil\n}",
		"TestPlanEmptyAndIncompleteTransactionsSelectNothing$",
	)
	run_mutation(
		"reverse-commit-order",
		"\treturn plan, nil\n}",
		"\tfor left, right := 0, len(plan)-1; left < right; left, right = left+1, right-1 {\n\t\tplan[left], plan[right] = plan[right], plan[left]\n\t}\n\treturn plan, nil\n}",
		"TestPlanOrdersSingleWriteTransactionsByCommit$",
	)
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
