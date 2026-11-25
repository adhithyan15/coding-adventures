"""Run the actual default CLI and validate its portable core report."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

from microbenchmark import Report


def main() -> int:
    executable_name = (
        "heap-construction-benchmark.exe"
        if os.name == "nt"
        else "heap-construction-benchmark"
    )
    executable = Path(sys.executable).parent / executable_name
    result = subprocess.run(
        [
            str(executable),
            "--revision",
            "build-smoke",
        ],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    document = json.loads(result.stdout)
    core = {key: document[key] for key in ("config", "batch_size", "pairs", "analysis")}
    report = Report.from_dict(core)
    if report.config.pairs != 40 or len(report.pairs) != 40:
        raise RuntimeError("default CLI must emit 40 paired observations")
    if any(row.baseline_ns <= 0 or row.candidate_ns <= 0 for row in report.pairs):
        raise RuntimeError("CLI observations must be positive")
    orders = [row.order for row in report.pairs]
    if orders.count("baseline-first") != 20 or orders.count("candidate-first") != 20:
        raise RuntimeError("default CLI order schedule must be balanced 20/20")
    if document.get("workload") != "heap-construction-repeated-push-vs-floyd":
        raise RuntimeError("unexpected workload metadata")
    if not all(
        document.get(field)
        for field in ("invocation", "revision", "runtime", "os", "arch")
    ):
        raise RuntimeError("CLI metadata is incomplete")
    print("default 40-pair CLI smoke passed; no speed threshold was applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
